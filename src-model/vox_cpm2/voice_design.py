import argparse
from pathlib import Path
import sys


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from vox_cpm2 import compose_generation_text, load_model_and_dependencies
from vox_cpm2.params import load_voice_design_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def validate_args(args: argparse.Namespace) -> Path:
    if not args.text.strip():
        raise ValueError("Target text cannot be empty.")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    return output_path


def generate_voice_design_audio(args: argparse.Namespace):
    output_path = validate_args(args)
    model, deps = load_model_and_dependencies(args.init_model_path, args.device)
    generation_text = compose_generation_text(args.text, args.instruct)

    wav = model.generate(
        text=generation_text,
        cfg_value=args.cfg_value,
        inference_timesteps=args.inference_timesteps,
    )

    deps.sf.write(str(output_path), wav, model.tts_model.sample_rate)


def main(argv: list[str] | None = None):
    cli_args = parse_args(argv)
    params = load_voice_design_params(cli_args.params_file)
    generate_voice_design_audio(params.to_namespace())


if __name__ == "__main__":
    main()
