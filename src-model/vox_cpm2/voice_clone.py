import argparse
from pathlib import Path

from vox_cpm2 import compose_generation_text, load_model_and_dependencies


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--init-model-path", dest="init_model_path", type=str, required=True)
    parser.add_argument("--mode", type=str, default="reference")
    parser.add_argument("--ref-audio-path", dest="ref_audio_path", type=str, required=True)
    parser.add_argument("--ref-text", dest="ref_text", type=str, default="")
    parser.add_argument("--text", type=str, required=True)
    parser.add_argument("--style-prompt", dest="style_prompt", type=str, default="")
    parser.add_argument("--cfg-value", dest="cfg_value", type=float, default=2.0)
    parser.add_argument("--inference-timesteps", dest="inference_timesteps", type=int, default=10)
    parser.add_argument("--language", type=str, default="auto")
    parser.add_argument("--output-path", dest="output_path", type=str, required=True)
    parser.add_argument("--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--device", type=str, default="cuda:0")
    return parser.parse_args(argv)


def validate_args(args: argparse.Namespace) -> tuple[Path, Path]:
    ref_audio_path = Path(args.ref_audio_path).expanduser().resolve()
    if not ref_audio_path.exists():
        raise FileNotFoundError(f"Reference audio file not found: {ref_audio_path}")
    if not args.text.strip():
        raise ValueError("Target text cannot be empty.")

    mode = args.mode.strip().lower()
    if mode not in {"reference", "ultimate"}:
        raise ValueError(f"Unsupported VoxCPM2 clone mode: {args.mode}")
    if mode == "ultimate" and not args.ref_text.strip():
        raise ValueError("Ultimate cloning requires reference transcript.")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    return ref_audio_path, output_path


def generate_voice_clone_audio(args: argparse.Namespace):
    ref_audio_path, output_path = validate_args(args)
    model, deps = load_model_and_dependencies(args.init_model_path, args.device)
    generation_text = compose_generation_text(args.text, args.style_prompt)
    mode = args.mode.strip().lower()

    if mode == "ultimate":
        wav = model.generate(
            text=generation_text,
            prompt_wav_path=str(ref_audio_path),
            prompt_text=args.ref_text.strip(),
            reference_wav_path=str(ref_audio_path),
            cfg_value=args.cfg_value,
            inference_timesteps=args.inference_timesteps,
        )
    else:
        wav = model.generate(
            text=generation_text,
            reference_wav_path=str(ref_audio_path),
            cfg_value=args.cfg_value,
            inference_timesteps=args.inference_timesteps,
        )

    deps.sf.write(str(output_path), wav, model.tts_model.sample_rate)


def main(argv: list[str] | None = None):
    args = parse_args(argv)
    generate_voice_clone_audio(args)


if __name__ == "__main__":
    main()