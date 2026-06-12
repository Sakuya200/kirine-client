import argparse
from pathlib import Path
import subprocess
import shutil
import sys
import time

DOTS_TTS_INFERENCE_MODEL_PATH = "rednote-hilab/dots.tts-soar"
DOTS_TTS_INFERENCE_MODEL_NAME = "dots.tts-soar"
DOTS_TTS_TRAINING_MODEL_PATH = "rednote-hilab/dots.tts-base"
DOTS_TTS_TRAINING_MODEL_NAME = "dots.tts-base"

DEFAULT_REPO_BRANCH = "main"

def _emit(message: str, *, stderr: bool = False) -> None:
    print(message, file=sys.stderr if stderr else sys.stdout)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", dest="base_model", type=str, required=True)
    parser.add_argument("--model-version", dest="model_version", type=str, required=True)
    parser.add_argument("--target-root-dir", dest="target_root_dir", type=str, required=True)
    parser.add_argument("--log-path", dest="log_path", type=str, required=False)
    parser.add_argument("--task-log-file", dest="task_log_file", type=str, required=False)
    parser.add_argument(
        "--repo-url",
        dest="repo_url",
        type=str,
        default="https://github.com/rednote-hilab/dots.tts",
    )
    parser.add_argument(
        "--repo-branch",
        dest="repo_branch",
        type=str,
        default=DEFAULT_REPO_BRANCH,
        help="Git branch to fetch before checking out fixed revision.",
    )
    parser.add_argument(
        "--asset-source",
        dest="asset_source",
        choices=["HF", "ModelScope"],
        default="ModelScope",
        help="Where to download pretrained assets from (matches upstream install.ps1).",
    )
    return parser.parse_args(argv)



def _has_standard_structure(root: Path) -> bool:
    """Check if standard directory structure exists."""
    return (root / "scripts").exists()


def _is_runtime_ready(root: Path) -> bool:
    """Check if dots_tts runtime is complete and ready to use."""
    # Must have the core directory structure
    if not _has_standard_structure(root):
        return False


def _clone_repo(
    repo_url: str,
    branch: str,
    destination: Path,
) -> Path:
    """Clone a git repository with retry logic and timeout handling."""
    git_bin = shutil.which("git")
    if git_bin is None:
        raise SystemExit(
            "Custom download requires git for automatic clone. "
            "Install git and make sure it is available in PATH."
        )

    max_retries = 3
    retry_delay = 5  # seconds
    timeout = 300  # seconds (5 minutes)
    
    for attempt in range(1, max_retries + 1):
        try:
            subprocess.run(
                [
                    git_bin,
                    "clone",
                    "--branch",
                    branch,
                    "--single-branch",
                    repo_url,
                    str(destination),
                ],
                check=True,
                timeout=timeout,
            )
            return destination
        except subprocess.TimeoutExpired:
            error_msg = f"Git clone timed out after {timeout}s (attempt {attempt}/{max_retries})"
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                time.sleep(retry_delay)
                retry_delay *= 2  # exponential backoff
            else:
                raise SystemExit(
                    f"❌ Git clone failed: {error_msg}.\n"
                    f"Please check your network connection and try again.\n"
                    f"Alternatively, you can:\n"
                    f"  1. Use a proxy: git config --global http.proxy <proxy_url>\n"
                    f"  2. Retry later if GitHub is unavailable"
                )
        except subprocess.CalledProcessError as e:
            error_msg = f"Git clone failed with exit code {e.returncode} (attempt {attempt}/{max_retries})"
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                # Clean up partial clone if any
                if destination.exists():
                    shutil.rmtree(destination)
                time.sleep(retry_delay)
                retry_delay *= 2  # exponential backoff
            else:
                raise SystemExit(
                    f"❌ {error_msg}.\n"
                    f"Error details: {e}\n"
                    f"Common causes:\n"
                    f"  - Network connectivity issues (RPC failed, connection reset)\n"
                    f"  - GitHub is temporarily unavailable\n"
                    f"  - Firewall/proxy blocking connection\n"
                    f"  - SSH key issues (if using SSH)\n"
                    f"\n"
                    f"Solutions:\n"
                    f"  1. Check your network connection\n"
                    f"  2. Try again in a few moments\n"
                    f"  3. Use a proxy if behind firewall: git config --global http.proxy <proxy_url>\n"
                    f"  4. Retry later if GitHub service is unstable"
                )
                
def _mount_project(target_root: Path, target_dir: Path) -> None:
    pip_path = target_root.parent / "dots_tts" / "venv" / "Scripts" / "pip.exe"
    if not pip_path.exists():
        raise SystemExit(
            f"❌ Cannot mount project: pip executable not found at expected path: {pip_path}\n"
            f"Please ensure you have set up the virtual environment correctly and that pip is available."
        )
    # 将下载好的代码项目挂载到对应模型层中作为外部依赖来使用
    try:
        subprocess.run(
            [
                str(pip_path),
                "install",
                "-e",
                str(target_dir),
            ],
            check=True,
        )
    except subprocess.CalledProcessError as e:
        raise SystemExit(
            f"❌ Failed to mount project: {e}\n"
            f"Please ensure you have set up the virtual environment correctly and try again."
        )
    
def _download_models(target_dir: Path, asset_source: str) -> None:
    if not (target_dir / "models").exists():
        # 创建空的 models 目录以满足代码依赖，但实际模型文件需要用户根据选定的模型版本手动下载并放置在该目录下。
        (target_dir / "models").mkdir(parents=True, exist_ok=True)
    
    # 下载 base 以及 inference 模型文件到 models 目录下，供后续训练和推理使用
    # 这里可以使用 modelscope 或 huggingface_hub 等库来下载模型
    try:
        if asset_source == "HF":
            _emit("Downloading models from Hugging Face...")
            from huggingface_hub import snapshot_download
            snapshot_download(repo_id=DOTS_TTS_INFERENCE_MODEL_PATH, local_dir=str(target_dir / "models" / DOTS_TTS_INFERENCE_MODEL_NAME))
            snapshot_download(repo_id=DOTS_TTS_TRAINING_MODEL_PATH, local_dir=str(target_dir / "models" / DOTS_TTS_TRAINING_MODEL_NAME))
        elif asset_source == "ModelScope":
            _emit("Downloading models from ModelScope...")
            from modelscope.hub import snapshot_download
            snapshot_download(repo_id=DOTS_TTS_INFERENCE_MODEL_PATH, local_dir=str(target_dir / "models" / DOTS_TTS_INFERENCE_MODEL_NAME))
            snapshot_download(repo_id=DOTS_TTS_TRAINING_MODEL_PATH, local_dir=str(target_dir / "models" / DOTS_TTS_TRAINING_MODEL_NAME))
    except Exception as e:
        _emit(f"⚠️  Model download failed: {e}", stderr=True)
        raise SystemExit(
            f"❌ Failed to download models from {asset_source}.\n"
            f"Please check your network connection and try again.\n"
            f"Alternatively, you can manually download the models from the respective platform and place them in the 'models' directory."
        )
    
    raise ValueError(f"Unsupported asset source: {asset_source}")

def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    target_root = Path(args.target_root_dir).expanduser().resolve()
    target_dir = target_root / args.base_model
    target_root.mkdir(parents=True, exist_ok=True)

    # Check 1: Ensure code project exists
    if not _has_standard_structure(target_dir):
        if target_dir.exists():
            _emit(
                f"⚠️  Existing target directory is not a valid project structure: {target_dir}",
            )
            _emit("🧹 Removing invalid directory for a clean direct clone...")
            shutil.rmtree(target_dir)

        _emit(
            f"📥 Cloning dots_tts directly into target directory: {target_dir}",
        )
        _clone_repo(
            args.repo_url,
            args.repo_branch,
            target_dir,
        )
        _emit("✓ Clone completed successfully")
    else:
        _emit(
            f"✓ dots_tts checkout already exists at {target_dir}; skip clone",
        )

    # Check 2: If runtime already complete, finish early after pinning.
    if _is_runtime_ready(target_dir):
        _emit(
            f"✓ dots_tts is already complete at {target_dir}",
        )
        return

    if not _is_runtime_ready(target_dir):
        raise SystemExit(
            f"❌ Setup failed after direct clone: dots_tts runtime is incomplete.\n"
            f"\n\n"
            f"Troubleshooting:\n"
            f"  1. Ensure you have sufficient disk space\n"
            f"  2. Check your network connection\n"
            f"  3. Manually download from: {args.repo_url}\n"
            f"  4. Try a different asset source via --asset-source HF-Mirror\n"
            f"  5. Verify selected model-scale/asset-version points to available checkpoints"
        )
        
    _mount_project(target_root, target_dir)
    _download_models(target_dir / "models")
    
    _emit(f"✅ dots_tts is ready at {target_dir}")
    



if __name__ == "__main__":
    main(sys.argv[1:])
