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


DEFAULT_MODEL_PATH = "OpenMOSS-Team/MOSS-TTS-Local-Transformer"
DEFAULT_CODEC_PATH = "OpenMOSS-Team/MOSS-Audio-Tokenizer"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run single-GPU MOSS-TTS Local finetuning.")
    parser.add_argument("--train_jsonl", "--train-jsonl", dest="train_jsonl", type=str, required=True)
    parser.add_argument("--init_model_path", "--init-model-path", dest="init_model_path", type=str, default=DEFAULT_MODEL_PATH)
    parser.add_argument("--codec_path", "--codec-path", dest="codec_path", type=str, default=DEFAULT_CODEC_PATH)
    parser.add_argument("--output_model_path", "--output-model-path", dest="output_model_path", type=str, required=True)
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--batch_size", "--batch-size", dest="batch_size", type=int, default=1)
    parser.add_argument(
        "--gradient_accumulation_steps",
        "--gradient-accumulation-steps",
        dest="gradient_accumulation_steps",
        type=int,
        default=8,
    )
    parser.add_argument("--num_epochs", "--num-epochs", dest="num_epochs", type=int, default=3)
    parser.add_argument("--learning_rate", "--learning-rate", dest="learning_rate", type=float, default=1e-5)
    parser.add_argument("--weight_decay", "--weight-decay", dest="weight_decay", type=float, default=0.1)
    parser.add_argument("--warmup_ratio", "--warmup-ratio", dest="warmup_ratio", type=float, default=0.03)
    parser.add_argument("--warmup_steps", "--warmup-steps", dest="warmup_steps", type=int, default=0)
    parser.add_argument("--max_grad_norm", "--max-grad-norm", dest="max_grad_norm", type=float, default=1.0)
    parser.add_argument("--mixed_precision", "--mixed-precision", dest="mixed_precision", type=str, default="bf16")
    parser.add_argument(
        "--enable_gradient_checkpointing",
        "--enable-gradient-checkpointing",
        dest="enable_gradient_checkpointing",
        action=argparse.BooleanOptionalAction,
        default=True,
    )
    parser.add_argument(
        "--skip_reference_audio_codes",
        "--skip-reference-audio-codes",
        dest="skip_reference_audio_codes",
        action=argparse.BooleanOptionalAction,
        default=True,
    )
    parser.add_argument("--prep_batch_size", "--prep-batch-size", dest="prep_batch_size", type=int, default=16)
    parser.add_argument("--prep_n_vq", "--prep-n-vq", dest="prep_n_vq", type=int, default=None)
    parser.add_argument(
        "--channelwise_loss_weight",
        "--channelwise-loss-weight",
        dest="channelwise_loss_weight",
        type=str,
        default="1,32",
    )
    parser.add_argument("--lr_scheduler_type", "--lr-scheduler-type", dest="lr_scheduler_type", type=str, default="cosine")
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
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
    args = parse_args(argv)
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