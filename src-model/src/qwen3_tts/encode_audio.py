import argparse
import json
from typing import Iterable

BATCH_INFER_NUM = 32


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--tokenizer_model_path", "--tokenizer-model-path", dest="tokenizer_model_path", type=str, default="Qwen/Qwen3-TTS-Tokenizer-12Hz")
    parser.add_argument("--input_jsonl", "--input-jsonl", dest="input_jsonl", type=str, required=True)
    parser.add_argument("--output_jsonl", "--output-jsonl", dest="output_jsonl", type=str, required=True)
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="flash_attention_2",
    )
    return parser.parse_args(argv)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def build_tokenizer_kwargs(device: str, attn_implementation: str = "flash_attention_2") -> dict[str, str]:
    kwargs = {"device_map": device}
    if not is_cpu_device(device):
        kwargs["attn_implementation"] = attn_implementation
    return kwargs


def load_tokenizer(tokenizer_model_path: str, device: str, attn_implementation: str):
    from qwen_tts import Qwen3TTSTokenizer

    return Qwen3TTSTokenizer.from_pretrained(
        tokenizer_model_path,
        **build_tokenizer_kwargs(device, attn_implementation),
    )


def load_jsonl(path: str) -> list[dict[str, object]]:
    with open(path, "r", encoding="utf-8") as file:
        total_lines = file.readlines()
    return [json.loads(line.strip()) for line in total_lines if line.strip()]


def encode_dataset(tokenizer, rows: Iterable[dict[str, object]]) -> list[dict[str, object]]:
    final_lines = []
    batch_lines = []
    batch_audios = []

    for line in rows:
        batch_lines.append(line)
        batch_audios.append(line["audio"])

        if len(batch_lines) >= BATCH_INFER_NUM:
            enc_res = tokenizer.encode(batch_audios)
            for code, line in zip(enc_res.audio_codes, batch_lines):
                line["audio_codes"] = code.cpu().tolist()
                final_lines.append(line)
            batch_lines.clear()
            batch_audios.clear()

    if len(batch_audios) > 0:
        enc_res = tokenizer.encode(batch_audios)
        for code, line in zip(enc_res.audio_codes, batch_lines):
            line["audio_codes"] = code.cpu().tolist()
            final_lines.append(line)
        batch_lines.clear()
        batch_audios.clear()

    return final_lines


def write_jsonl(path: str, rows: Iterable[dict[str, object]]) -> None:
    final_lines = [json.dumps(line, ensure_ascii=False) for line in rows]

    with open(path, "w", encoding="utf-8") as f:
        for line in final_lines:
            f.writelines(line + "\n")


def main(argv: list[str] | None = None):
    args = parse_args(argv)
    tokenizer = load_tokenizer(args.tokenizer_model_path, args.device, args.attn_implementation)
    rows = load_jsonl(args.input_jsonl)
    encoded_rows = encode_dataset(tokenizer, rows)
    write_jsonl(args.output_jsonl, encoded_rows)


if __name__ == "__main__":
    main()