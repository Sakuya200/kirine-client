from __future__ import annotations

import argparse
import sys
from pathlib import Path

def ensure_src_root_on_path() -> Path:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)
    return src_root


ensure_src_root_on_path()

from moss_tts_local.common import load_backend, save_generated_audio
from moss_tts_local.params import load_voice_clone_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run MOSS-TTS Local voice cloning inference.")
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def generate_audio(args: argparse.Namespace) -> None:
    if not args.text.strip():
        raise ValueError("Text cannot be empty.")

    ref_audio_path = Path(args.ref_audio_path).expanduser().resolve()
    if not ref_audio_path.exists():
        raise FileNotFoundError(f"Reference audio file not found: {ref_audio_path}")

    model, processor, deps, runtime = load_backend(
        model_path=args.init_model_path,
        device=args.device,
        requested_attn_implementation=args.attn_implementation,
    )
    conversation = [[
        processor.build_user_message(
            text=args.text,
            reference=[str(ref_audio_path)],
            language=args.language,
        )
    ]]
    batch = processor(conversation, mode="generation")

    with deps.torch.no_grad():
        outputs = model.generate(
            input_ids=batch["input_ids"].to(runtime.device),
            attention_mask=batch["attention_mask"].to(runtime.device),
            max_new_tokens=int(args.max_new_tokens),
            n_vq_for_inference=int(args.n_vq_for_inference),
        )

    message = processor.decode(outputs)[0]
    audio = message.audio_codes_list[0]
    save_generated_audio(args.output_path, audio, processor.model_config.sampling_rate)


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    generate_audio(load_voice_clone_params(cli_args.params_file).to_namespace())


if __name__ == "__main__":
    main()