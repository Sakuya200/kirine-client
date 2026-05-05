import argparse
from pathlib import Path
import sys
from types import SimpleNamespace


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from qwen3_tts.params import load_training_params
from qwen3_tts.encode_audio import (
    encode_dataset,
    load_jsonl,
    load_tokenizer,
    write_jsonl,
)
from qwen3_tts import training_full


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def encode_training_dataset(params) -> str:
    rows = load_jsonl(params.input_jsonl)
    if not rows:
        raise ValueError(f"Qwen3 training input jsonl is empty: {params.input_jsonl}")

    if all("audio_codes" in row for row in rows):
        return params.input_jsonl

    tokenizer = load_tokenizer(
        params.tokenizer_model_path,
        params.runtime.device,
        params.runtime.attn_implementation,
    )
    encoded_rows = encode_dataset(tokenizer, rows)
    write_jsonl(params.output_jsonl, encoded_rows)
    return params.output_jsonl


def build_training_namespace(params) -> argparse.Namespace:
    base_namespace = params.to_namespace()
    output_jsonl = encode_training_dataset(params)
    return SimpleNamespace(**vars(base_namespace), output_jsonl=output_jsonl)


def train(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_training_params(cli_args.params_file)
    training_full.run_training(build_training_namespace(params))


if __name__ == "__main__":
    train()