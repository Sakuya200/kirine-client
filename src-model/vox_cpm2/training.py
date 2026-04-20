from __future__ import annotations

import argparse
import importlib.util
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

import yaml


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from vox_cpm2 import RUNTIME_METADATA_FILE_NAME

SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]

VOXCPM_REPO_GIT_URL = "https://github.com/OpenBMB/VoxCPM.git"
VOXCPM_REPO_ARCHIVE_URL = "https://codeload.github.com/OpenBMB/VoxCPM/zip/refs/heads/main"

LORA_DEFAULTS = {
    "enable_lm": True,
    "enable_dit": True,
    "enable_proj": False,
    "r": 32,
    "alpha": 32,
    "dropout": "0.0",
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
    parser.add_argument("--lora-rank", dest="lora_rank", type=int, default=LORA_DEFAULTS["r"])
    parser.add_argument("--lora-alpha", dest="lora_alpha", type=int, default=LORA_DEFAULTS["alpha"])
    parser.add_argument("--lora-dropout", dest="lora_dropout", type=str, default=LORA_DEFAULTS["dropout"])
    parser.add_argument(
        "--enable-gradient-checkpointing",
        dest="enable_gradient_checkpointing",
        action=argparse.BooleanOptionalAction,
        default=False,
    )
    parser.add_argument("--train-script-path", dest="train_script_path", type=str, default="")
    return parser.parse_args(argv)


def parse_lora_dropout(value: str) -> float:
    normalized = value.strip()
    if not normalized:
        return float(LORA_DEFAULTS["dropout"])

    parsed = float(normalized)
    if parsed < 0.0:
        return float(LORA_DEFAULTS["dropout"])
    return parsed


def get_vendor_repo_root() -> Path:
    return Path(__file__).resolve().parents[1] / "vendor" / "VoxCPM"


def vendor_train_script_path() -> Path:
    return get_vendor_repo_root() / "scripts" / "train_voxcpm_finetune.py"


def install_voxcpm_training_sources_from_git(vendor_root: Path) -> bool:
    git_command = shutil.which("git")
    if not git_command:
        return False

    vendor_root.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [git_command, "clone", "--depth", "1", VOXCPM_REPO_GIT_URL, str(vendor_root)],
        check=True,
    )
    return True


def install_voxcpm_training_sources_from_archive(vendor_root: Path) -> bool:
    vendor_root.parent.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="voxcpm-src-") as temp_dir:
        archive_path = Path(temp_dir) / "voxcpm.zip"
        with urllib.request.urlopen(VOXCPM_REPO_ARCHIVE_URL) as response, archive_path.open("wb") as output:
            shutil.copyfileobj(response, output)

        with zipfile.ZipFile(archive_path) as archive:
            archive.extractall(vendor_root.parent)

        extracted_root = vendor_root.parent / "VoxCPM-main"
        if not extracted_root.exists():
            return False

        if vendor_root.exists():
            shutil.rmtree(vendor_root)
        extracted_root.rename(vendor_root)

    return True


def ensure_voxcpm_training_sources() -> Path | None:
    vendor_root = get_vendor_repo_root()
    train_script = vendor_train_script_path()
    if train_script.exists():
        return train_script

    if vendor_root.exists() and not train_script.exists():
        return None

    installers = [install_voxcpm_training_sources_from_git, install_voxcpm_training_sources_from_archive]
    errors: list[str] = []
    for installer in installers:
        try:
            if installer(vendor_root) and train_script.exists():
                return train_script
        except Exception as exc:
            errors.append(f"{installer.__name__}: {exc}")

    if errors:
        joined = "\n".join(errors)
        raise FileNotFoundError(
            "Unable to bootstrap VoxCPM training sources automatically.\n"
            f"Attempts:\n{joined}"
        )

    return None


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
        vendor_train_script_path(),
        Path(__file__).resolve().parents[3] / "VoxCPM" / "scripts" / "train_voxcpm_finetune.py",
    ]
    candidates.extend(workspace_candidates)

    for candidate in candidates:
        if candidate.exists():
            return candidate

    bootstrapped = ensure_voxcpm_training_sources()
    if bootstrapped and bootstrapped.exists():
        return bootstrapped

    joined = "\n".join(str(candidate) for candidate in candidates)
    raise FileNotFoundError(
        "Unable to locate VoxCPM training entrypoint scripts/train_voxcpm_finetune.py.\n"
        f"Checked:\n{joined}"
    )


def count_training_samples(train_jsonl: Path) -> int:
    with train_jsonl.open("r", encoding="utf-8") as file:
        sample_count = sum(1 for line in file if line.strip())
    if sample_count <= 0:
        raise ValueError(f"Training manifest contains no samples: {train_jsonl}")
    return sample_count


def estimate_training_schedule(
    train_jsonl: Path,
    batch_size: int,
    gradient_accumulation_steps: int,
    num_epochs: int,
) -> dict[str, int]:
    sample_count = count_training_samples(train_jsonl)
    effective_batch_size = max(1, batch_size) * max(1, gradient_accumulation_steps)
    steps_per_epoch = max(1, math.ceil(sample_count / effective_batch_size))
    total_steps = max(1, steps_per_epoch * max(1, num_epochs))
    return {
        "sample_count": sample_count,
        "effective_batch_size": effective_batch_size,
        "steps_per_epoch": steps_per_epoch,
        "total_steps": total_steps,
    }


def resolve_num_workers() -> int:
    cpu_count = os.cpu_count() or 2
    return max(2, min(4, cpu_count))


def resolve_warmup_steps(max_steps: int) -> int:
    if max_steps <= 1:
        return max_steps

    return min(100, max(1, math.ceil(max_steps * 0.1)), max_steps - 1)


def build_training_config(args: argparse.Namespace, train_jsonl: Path, output_model_path: Path) -> dict[str, object]:
    checkpoint_dir = output_model_path / "checkpoints" / args.training_mode
    tensorboard_dir = output_model_path / "logs" / args.training_mode
    checkpoint_dir.mkdir(parents=True, exist_ok=True)
    tensorboard_dir.mkdir(parents=True, exist_ok=True)

    schedule = estimate_training_schedule(
        train_jsonl,
        args.batch_size,
        args.gradient_accumulation_steps,
        args.num_epochs,
    )
    max_steps = schedule["total_steps"]
    steps_per_epoch = schedule["steps_per_epoch"]
    config: dict[str, object] = {
        "pretrained_path": str(Path(args.init_model_path).expanduser().resolve()),
        "train_manifest": str(train_jsonl),
        "val_manifest": "",
        "sample_rate": 16000,
        "out_sample_rate": 48000,
        "batch_size": args.batch_size,
        "grad_accum_steps": max(1, args.gradient_accumulation_steps),
        "num_workers": resolve_num_workers(),
        "num_iters": max_steps,
        "log_interval": max(1, min(10, steps_per_epoch)),
        "valid_interval": max_steps,
        "save_interval": max(100, max_steps),
        "learning_rate": 1e-4 if args.training_mode == "lora" else 1e-5,
        "weight_decay": 0.01,
        "warmup_steps": resolve_warmup_steps(max_steps),
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
        config["lora"] = {
            **LORA_DEFAULTS,
            "r": max(1, int(args.lora_rank)),
            "alpha": max(1, int(args.lora_alpha)),
            "dropout": parse_lora_dropout(args.lora_dropout),
        }
    return config


def resolve_latest_checkpoint(checkpoint_root: Path) -> Path:
    latest_path = checkpoint_root / "latest"
    if latest_path.exists():
        return latest_path.resolve()

    step_dirs = sorted(path for path in checkpoint_root.glob("step_*") if path.is_dir())
    if step_dirs:
        return step_dirs[-1].resolve()

    raise FileNotFoundError(f"No training checkpoint found under: {checkpoint_root}")


def optional_relative_path(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return ""


def write_runtime_metadata(output_model_path: Path, init_model_path: Path, training_mode: str, latest_checkpoint: Path) -> None:
    metadata_path = output_model_path / RUNTIME_METADATA_FILE_NAME
    metadata = {
        "trainingMode": training_mode,
        "baseModelPath": str(init_model_path.resolve()),
        "baseModelRelativePath": optional_relative_path(init_model_path, SRC_MODEL_ROOT),
        "latestCheckpointPath": str(latest_checkpoint.resolve()),
        "latestCheckpointRelativePath": optional_relative_path(latest_checkpoint, output_model_path),
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
