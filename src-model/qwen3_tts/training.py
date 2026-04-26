import argparse
from pathlib import Path
import sys


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from qwen3_tts.params import load_training_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def resolve_training_module(use_lora: bool):
    if use_lora:
        from qwen3_tts import training_lora as target_module
    else:
        from qwen3_tts import training_full as target_module

    return target_module


def train(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_training_params(cli_args.params_file)
    target_module = resolve_training_module(params.use_lora)
    target_module.train_from_params(params)


if __name__ == "__main__":
    train()