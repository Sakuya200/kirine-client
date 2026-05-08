import argparse
from pathlib import Path
import shutil
import sys


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", dest="base_model", type=str, required=True)
    parser.add_argument("--model-scale", dest="model_scale", type=str, required=True)
    parser.add_argument("--target-root-dir", dest="target_root_dir", type=str, required=True)
    parser.add_argument("--log-path", dest="log_path", type=str, required=False)
    parser.add_argument("--task-log-file", dest="task_log_file", type=str, required=False)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    target_root = Path(args.target_root_dir).expanduser().resolve()
    target_dir = target_root / args.base_model
    target_dir.mkdir(parents=True, exist_ok=True)

    legacy_dir = Path(__file__).resolve().parents[1] / "base-models" / "gpt_sovits_v2pp"
    if legacy_dir.exists() and legacy_dir != target_dir:
        if not any(target_dir.iterdir()):
            for entry in legacy_dir.iterdir():
                destination = target_dir / entry.name
                if destination.exists():
                    continue
                if entry.is_dir():
                    shutil.copytree(entry, destination)
                else:
                    shutil.copy2(entry, destination)

    if not any(target_dir.iterdir()):
        raise SystemExit(
            "gpt_sovits_cpufast download.py currently expects an existing GPT-SoVITS-CPUFast runtime under "
            f"{legacy_dir} or {target_dir}. Populate one of those locations and retry installation."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
