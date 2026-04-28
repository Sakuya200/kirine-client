from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

def ensure_src_root_on_path() -> Path:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)
    return src_root


ensure_src_root_on_path()

from moss_tts_local.common import current_python_executable, run_subprocess
from moss_tts_local.params import load_training_params


DEFAULT_MODEL_PATH = "OpenMOSS-Team/MOSS-TTS-Local-Transformer"
DEFAULT_CODEC_PATH = "OpenMOSS-Team/MOSS-Audio-Tokenizer"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run single-GPU MOSS-TTS Local finetuning.")
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def build_prepare_command(args: argparse.Namespace, prepared_jsonl: Path) -> list[str]:
    command = [
        current_python_executable(),
        "-m",
        "moss_tts_local.finetuning.prepare_data",
        "--model-path",
        args.init_model_path,
        "--codec-path",
        args.codec_path,
        "--device",
        args.device,
        "--input-jsonl",
        args.train_jsonl,
        "--output-jsonl",
        str(prepared_jsonl),
        "--batch-size",
        str(args.prep_batch_size),
    ]
    if args.prep_n_vq is not None:
        command.extend(["--n-vq", str(args.prep_n_vq)])
    if args.skip_reference_audio_codes:
        command.append("--skip-reference-audio-codes")
    return command


def build_train_command(args: argparse.Namespace, prepared_jsonl: Path) -> list[str]:
    command = [
        current_python_executable(),
        "-m",
        "accelerate.commands.launch",
        "--num_processes",
        "1",
        "-m",
        "moss_tts_local.finetuning.sft",
        "--model-path",
        args.init_model_path,
        "--codec-path",
        args.codec_path,
        "--train-jsonl",
        str(prepared_jsonl),
        "--output-dir",
        args.output_model_path,
        "--per-device-batch-size",
        str(args.batch_size),
        "--gradient-accumulation-steps",
        str(args.gradient_accumulation_steps),
        "--learning-rate",
        str(args.learning_rate),
        "--weight-decay",
        str(args.weight_decay),
        "--num-epochs",
        str(args.num_epochs),
        "--mixed-precision",
        args.mixed_precision,
        "--channelwise-loss-weight",
        args.channelwise_loss_weight,
        "--lr-scheduler-type",
        args.lr_scheduler_type,
        "--max-grad-norm",
        str(args.max_grad_norm),
        "--attn-implementation",
        "auto",
    ]
    if args.enable_gradient_checkpointing:
        command.append("--gradient-checkpointing")
    if args.warmup_steps > 0:
        command.extend(["--warmup-steps", str(args.warmup_steps)])
    else:
        command.extend(["--warmup-ratio", str(args.warmup_ratio)])
    return command


def train(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    args = load_training_params(cli_args.params_file).to_namespace()
    train_jsonl_path = Path(args.train_jsonl).expanduser().resolve()
    if not train_jsonl_path.exists():
        raise FileNotFoundError(f"Training JSONL not found: {train_jsonl_path}")

    output_root = Path(args.output_model_path).expanduser().resolve()
    prepared_dir = output_root / "prepared"
    prepared_dir.mkdir(parents=True, exist_ok=True)
    prepared_jsonl = prepared_dir / "train_with_codes.jsonl"

    run_subprocess(build_prepare_command(args, prepared_jsonl), cwd=Path(__file__).resolve().parents[1])
    run_subprocess(build_train_command(args, prepared_jsonl), cwd=Path(__file__).resolve().parents[1])

    metadata = {
        "modelPath": str(output_root),
        "preparedJsonl": str(prepared_jsonl),
        "initModelPath": args.init_model_path,
        "codecPath": args.codec_path,
        "singleGpu": True,
    }
    with (output_root / "moss_runtime.json").open("w", encoding="utf-8") as file:
        json.dump(metadata, file, indent=2, ensure_ascii=False)


if __name__ == "__main__":
    train()