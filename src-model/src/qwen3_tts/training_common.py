import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import sys
from types import SimpleNamespace


@dataclass
class TrainingRuntimeOptions:
    is_cpu: bool
    accelerator_kwargs: dict[str, object]
    model_load_kwargs: dict[str, object]
    mode_label: str


@dataclass
class TrainingPipelineContext:
    runtime: TrainingRuntimeOptions
    accelerator: object
    qwen3tts: object
    model: object
    optimizer: object
    scheduler: object | None
    train_dataloader: object
    train_data: list[dict[str, object]]


def add_common_training_args(parser: argparse.ArgumentParser) -> argparse.ArgumentParser:
    parser.add_argument(
        "--init_model_path",
        "--init-model-path",
        dest="init_model_path",
        type=str,
        default="Qwen/Qwen3-TTS-12Hz-1.7B-Base",
    )
    parser.add_argument(
        "--output_model_path",
        "--output-model-path",
        dest="output_model_path",
        type=str,
        default="output",
    )
    parser.add_argument("--train_jsonl", "--train-jsonl", dest="train_jsonl", type=str, required=True)
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--batch_size", "--batch-size", dest="batch_size", type=int, default=2)
    parser.add_argument("--lr", type=float, default=2e-5)
    parser.add_argument("--num_epochs", "--num-epochs", dest="num_epochs", type=int, default=3)
    parser.add_argument("--speaker_name", "--speaker-name", dest="speaker_name", type=str, default="speaker_test")
    parser.add_argument(
        "--gradient_accumulation_steps",
        "--gradient-accumulation-steps",
        dest="gradient_accumulation_steps",
        type=int,
        default=4,
    )
    parser.add_argument(
        "--enable-gradient-checkpointing",
        dest="enable_gradient_checkpointing",
        action=argparse.BooleanOptionalAction,
        default=False,
    )
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="flash_attention_2",
    )
    return parser


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


def load_training_dependencies() -> SimpleNamespace:
    ensure_src_root_on_path()

    import torch
    from accelerate import Accelerator
    from qwen3_tts.dataset import TTSDataset
    from qwen_tts.inference.qwen3_tts_model import Qwen3TTSModel
    from safetensors.torch import save_file
    from torch.optim import AdamW
    from torch.utils.data import DataLoader
    from transformers import AutoConfig, get_linear_schedule_with_warmup

    return SimpleNamespace(
        torch=torch,
        Accelerator=Accelerator,
        TTSDataset=TTSDataset,
        Qwen3TTSModel=Qwen3TTSModel,
        save_file=save_file,
        AdamW=AdamW,
        DataLoader=DataLoader,
        AutoConfig=AutoConfig,
        get_linear_schedule_with_warmup=get_linear_schedule_with_warmup,
    )


def load_training_rows(train_jsonl: str) -> list[dict[str, object]]:
    with open(train_jsonl, "r", encoding="utf-8") as file:
        train_data = file.readlines()
    return [json.loads(line) for line in train_data if line.strip()]


def enable_gradient_checkpointing(model, require_input_grads: bool = False) -> None:
    enabled = False
    for candidate in (model, getattr(model, "talker", None), getattr(getattr(model, "talker", None), "model", None)):
        if candidate is None:
            continue
        enable = getattr(candidate, "gradient_checkpointing_enable", None)
        if callable(enable):
            enable()
            enabled = True

    if not enabled:
        raise RuntimeError("Gradient checkpointing was requested, but the model does not expose a supported API.")

    if require_input_grads:
        enable_inputs = getattr(model, "enable_input_require_grads", None)
        if callable(enable_inputs):
            enable_inputs()


def build_train_dataloader(args: argparse.Namespace, deps, qwen3tts):
    config = deps.AutoConfig.from_pretrained(args.init_model_path)
    train_data = load_training_rows(args.train_jsonl)
    dataset = deps.TTSDataset(train_data, qwen3tts.processor, config)
    train_dataloader = deps.DataLoader(
        dataset,
        batch_size=args.batch_size,
        shuffle=True,
        collate_fn=dataset.collate_fn,
    )
    return train_data, train_dataloader