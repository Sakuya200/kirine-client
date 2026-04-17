from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import yaml

from vox_cpm2 import RUNTIME_METADATA_FILE_NAME

LORA_DEFAULTS = {
    "enable_lm": True,
    "enable_dit": True,
    "enable_proj": False,
    "r": 32,
    "alpha": 32,
    "dropout": 0.0,
}


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--train-jsonl", dest="train_jsonl", type=str, required=True)
    parser.add_argument("--output-model-path", dest="output_model_path", type=str, required=True)
    parser.add_argument("--init-model-path", dest="init_model_path", type=str, required=True)
    parser.add_argument("--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--batch-size", dest="batch_size", type=int, default=8)
    parser.add_argument("--num-epochs", dest="num_epochs", type=int, default=1)
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--gradient-accumulation-steps", dest="gradient_accumulation_steps", type=int, default=1)
    parser.add_argument("--training-mode", dest="training_mode", choices=["full", "lora"], default="lora")
    parser.add_argument(
        "--enable-gradient-checkpointing",
        dest="enable_gradient_checkpointing",
        action=argparse.BooleanOptionalAction,
        default=False,
    )
    parser.add_argument("--train-script-path", dest="train_script_path", type=str, default="")
    return parser.parse_args(argv)


def resolve_train_script_path(explicit_path: str) -> Path:
    candidates: list[Path] = []
    if explicit_path.strip():
        candidates.append(Path(explicit_path).expanduser().resolve())

    spec = importlib.util.find_spec("voxcpm")
    if spec and spec.origin:
        package_root = Path(spec.origin).resolve().parent
        candidates.extend(
            [
                package_root / "scripts" / "train_voxcpm_finetune.py",
                package_root.parent / "scripts" / "train_voxcpm_finetune.py",
                package_root.parent.parent / "scripts" / "train_voxcpm_finetune.py",
            ]
        )

    workspace_candidates = [
        Path(__file__).resolve().parents[2] / "vendor" / "VoxCPM" / "scripts" / "train_voxcpm_finetune.py",
        Path(__file__).resolve().parents[3] / "VoxCPM" / "scripts" / "train_voxcpm_finetune.py",
    ]
    candidates.extend(workspace_candidates)

    for candidate in candidates:
        if candidate.exists():
            return candidate

    joined = "\n".join(str(candidate) for candidate in candidates)
    raise FileNotFoundError(
        "Unable to locate VoxCPM training entrypoint scripts/train_voxcpm_finetune.py.\n"
        f"Checked:\n{joined}"
    )


def estimate_max_steps(train_jsonl: Path, batch_size: int, num_epochs: int) -> int:
    with train_jsonl.open("r", encoding="utf-8") as file:
        sample_count = sum(1 for line in file if line.strip())
    if sample_count <= 0:
        raise ValueError(f"Training manifest contains no samples: {train_jsonl}")
    steps_per_epoch = max(1, sample_count // max(1, batch_size))
    return max(steps_per_epoch * max(1, num_epochs), max(1, num_epochs))


def build_training_config(args: argparse.Namespace, train_jsonl: Path, output_model_path: Path) -> dict[str, object]:
    checkpoint_dir = output_model_path / "checkpoints" / args.training_mode
    tensorboard_dir = output_model_path / "logs" / args.training_mode
    checkpoint_dir.mkdir(parents=True, exist_ok=True)
    tensorboard_dir.mkdir(parents=True, exist_ok=True)

    max_steps = estimate_max_steps(train_jsonl, args.batch_size, args.num_epochs)
    config: dict[str, object] = {
        "pretrained_path": str(Path(args.init_model_path).expanduser().resolve()),
        "train_manifest": str(train_jsonl),
        "val_manifest": "",
        "sample_rate": 16000,
        "out_sample_rate": 48000,
        "batch_size": args.batch_size,
        "grad_accum_steps": max(1, args.gradient_accumulation_steps),
        "num_workers": 2,
        "num_iters": max_steps,
        "log_interval": 10,
        "valid_interval": max_steps,
        "save_interval": max(100, max_steps),
        "learning_rate": 1e-4 if args.training_mode == "lora" else 1e-5,
        "weight_decay": 0.01,
        "warmup_steps": min(100, max_steps),
        "max_steps": max_steps,
        "max_batch_tokens": 8192,
        "save_path": str(checkpoint_dir),
        "tensorboard": str(tensorboard_dir),
        "lambdas": {
            "loss/diff": 1.0,
            "loss/stop": 1.0,
        },
        "gradient_checkpointing": bool(args.enable_gradient_checkpointing),
    }
    if args.training_mode == "lora":
        config["lora"] = dict(LORA_DEFAULTS)
    return config


def resolve_latest_checkpoint(checkpoint_root: Path) -> Path:
    latest_path = checkpoint_root / "latest"
    if latest_path.exists():
        return latest_path.resolve()

    step_dirs = sorted(path for path in checkpoint_root.glob("step_*") if path.is_dir())
    if step_dirs:
        return step_dirs[-1].resolve()

    raise FileNotFoundError(f"No training checkpoint found under: {checkpoint_root}")


def write_runtime_metadata(output_model_path: Path, init_model_path: Path, training_mode: str, latest_checkpoint: Path) -> None:
    metadata_path = output_model_path / RUNTIME_METADATA_FILE_NAME
    metadata = {
        "trainingMode": training_mode,
        "baseModelPath": str(init_model_path.resolve()),
        "latestCheckpointPath": str(latest_checkpoint.resolve()),
    }
    with metadata_path.open("w", encoding="utf-8") as file:
        json.dump(metadata, file, ensure_ascii=False, indent=2)


def train(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    train_jsonl = Path(args.train_jsonl).expanduser().resolve()
    if not train_jsonl.exists():
        raise FileNotFoundError(f"Training manifest not found: {train_jsonl}")

    output_model_path = Path(args.output_model_path).expanduser().resolve()
    output_model_path.mkdir(parents=True, exist_ok=True)
    init_model_path = Path(args.init_model_path).expanduser().resolve()
    if not init_model_path.exists():
        raise FileNotFoundError(f"Initial model path not found: {init_model_path}")

    train_script_path = resolve_train_script_path(args.train_script_path)
    config = build_training_config(args, train_jsonl, output_model_path)
    config_path = output_model_path / f"voxcpm_{args.training_mode}_config.yaml"
    with config_path.open("w", encoding="utf-8") as file:
        yaml.safe_dump(config, file, allow_unicode=True, sort_keys=False)

    command = [sys.executable, str(train_script_path), "--config_path", str(config_path)]
    subprocess.run(command, check=True, cwd=str(train_script_path.parent.parent))

    checkpoint_root = output_model_path / "checkpoints" / args.training_mode
    latest_checkpoint = resolve_latest_checkpoint(checkpoint_root)
    write_runtime_metadata(output_model_path, init_model_path, args.training_mode, latest_checkpoint)


if __name__ == "__main__":
    train()
