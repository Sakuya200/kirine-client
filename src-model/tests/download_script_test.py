import argparse
import subprocess
import sys
from pathlib import Path


SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_TARGET_ROOT_DIR = SRC_MODEL_ROOT / "base-models"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run model-specific download.py under src-model/<model-dir>."
    )
    parser.add_argument(
        "--model-dir",
        dest="model_dir",
        required=True,
        help="Model directory name under src-model, e.g. gpt_sovits_cpufast",
    )
    parser.add_argument(
        "--model-version",
        dest="model_version",
        default="test",
        help="Model version passed to download.py (default: test)",
    )
    parser.add_argument(
        "--target-root-dir",
        dest="target_root_dir",
        default=str(DEFAULT_TARGET_ROOT_DIR),
        help="Target root dir passed to download.py (default: src-model/base-models)",
    )
    parser.add_argument(
        "--python",
        dest="python_executable",
        default=sys.executable,
        help="Python executable used to run download.py (default: current interpreter)",
    )
    parser.add_argument(
        "--extra-arg",
        dest="extra_args",
        action="append",
        default=[],
        help="Extra argument appended to download.py invocation; can be used multiple times",
    )
    return parser.parse_args(argv)


def resolve_download_script(model_dir: str) -> Path:
    model_root = SRC_MODEL_ROOT / model_dir
    if not model_root.exists() or not model_root.is_dir():
        raise FileNotFoundError(f"Model directory not found: {model_root}")

    download_script = model_root / "download.py"
    if not download_script.exists() or not download_script.is_file():
        raise FileNotFoundError(f"download.py not found under model directory: {download_script}")

    return download_script


def build_command(args: argparse.Namespace, download_script: Path) -> list[str]:
    command = [
        args.python_executable,
        str(download_script),
        "--base-model",
        args.model_dir,
        "--model-version",
        args.model_version,
        "--target-root-dir",
        str(Path(args.target_root_dir).expanduser().resolve()),
    ]
    if args.extra_args:
        command.extend(args.extra_args)
    return command


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)

    try:
        download_script = resolve_download_script(args.model_dir)
    except FileNotFoundError as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2

    command = build_command(args, download_script)
    print("Running download script:")
    print(" ".join(command))

    completed = subprocess.run(command, cwd=SRC_MODEL_ROOT)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
