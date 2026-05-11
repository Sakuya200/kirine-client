import argparse
import os
from pathlib import Path
import subprocess
import shutil
import sys


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", dest="base_model", type=str, required=True)
    parser.add_argument("--model-scale", dest="model_scale", type=str, required=True)
    parser.add_argument("--target-root-dir", dest="target_root_dir", type=str, required=True)
    parser.add_argument("--log-path", dest="log_path", type=str, required=False)
    parser.add_argument("--task-log-file", dest="task_log_file", type=str, required=False)
    parser.add_argument("--source-dir", dest="source_dir", type=str, required=False)
    parser.add_argument(
        "--repo-url",
        dest="repo_url",
        type=str,
        default="https://github.com/baicai-1145/GPT-SoVITS-CPUFast",
    )
    return parser.parse_args(argv)


def _is_cpufast_runtime_ready(root: Path) -> bool:
    return (
        (root / "GPT_SoVITS" / "inference_webui_fast.py").exists()
        and (root / "GPT_SoVITS" / "pretrained_models" / "s1v3.ckpt").exists()
        and (root / "GPT_SoVITS" / "pretrained_models" / "v2Pro" / "s2Gv2ProPlus.pth").exists()
    )


def _copy_tree_contents(source_dir: Path, target_dir: Path) -> None:
    for entry in source_dir.iterdir():
        destination = target_dir / entry.name
        if destination.exists():
            continue
        if entry.is_dir():
            shutil.copytree(entry, destination)
        else:
            shutil.copy2(entry, destination)


def _resolve_source_runtime_root(source_dir: Path) -> Path:
    direct = source_dir / "GPT_SoVITS"
    if direct.exists():
        return source_dir

    nested = source_dir / "GPT-SoVITS-CPUFast"
    if (nested / "GPT_SoVITS").exists():
        return nested

    raise SystemExit(
        "Provided --source-dir does not look like a GPT-SoVITS-CPUFast checkout. "
        f"Expected GPT_SoVITS/ under {source_dir}."
    )


def _clone_repo(repo_url: str, destination: Path) -> Path:
    git_bin = shutil.which("git")
    if git_bin is None:
        raise SystemExit(
            "Custom download requires git for automatic clone. "
            "Install git or pass --source-dir to an existing GPT-SoVITS-CPUFast checkout."
        )

    subprocess.run(
        [git_bin, "clone", "--depth", "1", repo_url, str(destination)],
        check=True,
    )
    return destination


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    target_root = Path(args.target_root_dir).expanduser().resolve()
    target_dir = target_root / args.base_model
    target_dir.mkdir(parents=True, exist_ok=True)

    if _is_cpufast_runtime_ready(target_dir):
        return

    explicit_source = args.source_dir or os.environ.get("GPT_SOVITS_CPUFAST_SOURCE")
    if explicit_source:
        source_root = _resolve_source_runtime_root(Path(explicit_source).expanduser().resolve())
        _copy_tree_contents(source_root, target_dir)

    if _is_cpufast_runtime_ready(target_dir):
        return

    clone_root = target_dir / "GPT-SoVITS-CPUFast"
    if not clone_root.exists():
        _clone_repo(args.repo_url, clone_root)

    cloned_runtime_root = _resolve_source_runtime_root(clone_root)
    _copy_tree_contents(cloned_runtime_root, target_dir)

    if not _is_cpufast_runtime_ready(target_dir):
        raise SystemExit(
            "gpt_sovits_cpufast download failed to produce a runnable runtime layout. "
            "Expected GPT_SoVITS/inference_webui_fast.py and default checkpoints under "
            f"{target_dir}."
        )


if __name__ == "__main__":
    main(sys.argv[1:])
