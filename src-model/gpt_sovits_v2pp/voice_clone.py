import argparse
from pathlib import Path
import sys


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from gpt_sovits_v2pp.params import load_voice_clone_params, run_inference_cli


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_voice_clone_params(cli_args.params_file)
    run_inference_cli(params.to_namespace())


if __name__ == "__main__":
    main()