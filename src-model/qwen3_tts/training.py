import argparse
from pathlib import Path
import sys


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()


def resolve_training_module(argv: list[str] | None = None):
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument(
        "--use-lora",
        dest="use_lora",
        action=argparse.BooleanOptionalAction,
        default=False,
    )
    route_args, forwarded_argv = parser.parse_known_args(argv)

    if route_args.use_lora:
        from qwen3_tts import training_lora as target_module
    else:
        from qwen3_tts import training_full as target_module

    return target_module, forwarded_argv


def train(argv: list[str] | None = None) -> None:
    target_module, forwarded_argv = resolve_training_module(argv)
    target_module.train(forwarded_argv)


if __name__ == "__main__":
    train()