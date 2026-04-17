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


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--init-model-path", dest="init_model_path", type=str, required=True)
    parser.add_argument("--text", type=str, required=True)
    parser.add_argument("--cfg-value", dest="cfg_value", type=float, default=2.0)
    parser.add_argument("--inference-timesteps", dest="inference_timesteps", type=int, default=10)
    parser.add_argument("--output-path", dest="output_path", type=str, required=True)
    parser.add_argument("--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--device", type=str, default="cuda:0")
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
    args = parse_args(argv)
    generate_audio(args)


if __name__ == "__main__":
    main()
