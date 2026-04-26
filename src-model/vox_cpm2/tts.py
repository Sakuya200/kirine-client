import argparse
from pathlib import Path
import sys


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from vox_cpm2 import load_model_and_dependencies
from vox_cpm2.params import load_tts_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def generate_audio(args: argparse.Namespace):
    if not args.text.strip():
        raise ValueError("Input text cannot be empty.")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    model, deps = load_model_and_dependencies(args.init_model_path, args.device)
    wav = model.generate(
        text=args.text.strip(),
        cfg_value=args.cfg_value,
        inference_timesteps=args.inference_timesteps,
    )
    deps.sf.write(str(output_path), wav, model.tts_model.sample_rate)


def main(argv: list[str] | None = None):
    cli_args = parse_args(argv)
    params = load_tts_params(cli_args.params_file)
    generate_audio(params.to_namespace())


if __name__ == "__main__":
    main()
