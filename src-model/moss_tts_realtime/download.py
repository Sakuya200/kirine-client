"""MOSS-TTS-Realtime Custom 下载脚本。

流程：
1. 将上游项目 git clone 到 ``<target-root-dir>/<base-model>``（即
   ``base-models/moss_tts_realtime`` = OpenMOSS/MOSS-TTS 仓库根）。
2. 下载推理/训练所需权重到克隆目录的 ``models/`` 下：
   - ``OpenMOSS-Team/MOSS-TTS-Realtime``      -> 基座权重（流式基座合成 / 训练初始化）
   - ``OpenMOSS-Team/MOSS-Audio-Tokenizer``    -> 编解码器（voice prompt 编码 + 音频解码）

依赖安装不在本脚本内完成--由 ``init_task_runtime.ps1`` 通过适配器的
``requirements.txt`` / ``requirements-torch.txt`` 装入共享的 conda_env/venv。
本脚本不做 editable install：streaming.py / training.py 从克隆仓库根目录运行即可经
``sys.path[0]`` 导入 ``mossttsrealtime`` 包。
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import time
from pathlib import Path

DEFAULT_REPO_URL = "https://github.com/OpenMOSS/MOSS-TTS"
DEFAULT_REPO_BRANCH = "main"

# 基座权重（流式基座 / 训练初始化）。
BASE_MODEL_REPO_ID = "OpenMOSS-Team/MOSS-TTS-Realtime"
BASE_MODEL_LOCAL_NAME = "MOSS-TTS-Realtime"
# 编解码器（voice prompt 编码 + 音频解码）。
CODEC_REPO_ID = "OpenMOSS-Team/MOSS-Audio-Tokenizer"
CODEC_LOCAL_NAME = "MOSS-Audio-Tokenizer"


def _emit(message: str, *, stderr: bool = False) -> None:
    print(message, file=sys.stderr if stderr else sys.stdout)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", dest="base_model", type=str, required=True)
    parser.add_argument("--model-version", dest="model_version", type=str, required=True)
    parser.add_argument(
        "--target-root-dir", dest="target_root_dir", type=str, required=True
    )
    parser.add_argument("--log-path", dest="log_path", type=str, required=False)
    parser.add_argument(
        "--task-log-file", dest="task_log_file", type=str, required=False
    )
    parser.add_argument(
        "--repo-url", dest="repo_url", type=str, default=DEFAULT_REPO_URL
    )
    parser.add_argument(
        "--repo-branch", dest="repo_branch", type=str, default=DEFAULT_REPO_BRANCH
    )
    return parser.parse_args(argv)


def _clone_repo(repo_url: str, branch: str, destination: Path) -> Path:
    """Clone a git repository with retry logic and timeout handling."""
    git_bin = shutil.which("git")
    if git_bin is None:
        raise SystemExit(
            "Custom download requires git for automatic clone. "
            "Install git and make sure it is available in PATH."
        )

    max_retries = 3
    retry_delay = 5  # seconds
    timeout = 600  # seconds (10 minutes)

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
            error_msg = (
                f"Git clone timed out after {timeout}s (attempt {attempt}/{max_retries})"
            )
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                time.sleep(retry_delay)
                retry_delay *= 2
            else:
                raise SystemExit(
                    f"❌ Git clone failed: {error_msg}.\n"
                    f"Please check your network connection and try again.\n"
                    f"Alternatively, you can:\n"
                    f"  1. Use a proxy: git config --global http.proxy <proxy_url>\n"
                    f"  2. Retry later if GitHub is unavailable"
                )
        except subprocess.CalledProcessError as e:
            error_msg = (
                f"Git clone failed with exit code {e.returncode} "
                f"(attempt {attempt}/{max_retries})"
            )
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                if destination.exists():
                    shutil.rmtree(destination)
                time.sleep(retry_delay)
                retry_delay *= 2
            else:
                raise SystemExit(
                    f"❌ {error_msg}.\n"
                    f"Error details: {e}\n"
                    f"Common causes:\n"
                    f"  - Network connectivity issues (RPC failed, connection reset)\n"
                    f"  - GitHub is temporarily unavailable\n"
                    f"  - Firewall/proxy blocking connection\n"
                    f"\n"
                    f"Solutions:\n"
                    f"  1. Check your network connection\n"
                    f"  2. Try again in a few moments\n"
                    f"  3. Use a proxy if behind firewall: "
                    f"git config --global http.proxy <proxy_url>\n"
                    f"  4. Retry later if GitHub service is unstable"
                )

    raise SystemExit("❌ Git clone exhausted retries.")


def _snapshot_download(repo_id: str, local_dir: str | None) -> None:
    from huggingface_hub import snapshot_download

    _emit(f"📥 Downloading {repo_id} ...")
    if local_dir is None:
        snapshot_download(repo_id=repo_id)
    else:
        snapshot_download(repo_id=repo_id, local_dir=local_dir)
    _emit(f"✓ {repo_id} downloaded")


def _download_models(target_dir: Path) -> None:
    models_dir = target_dir / "models"
    models_dir.mkdir(parents=True, exist_ok=True)

    try:
        _snapshot_download(
            BASE_MODEL_REPO_ID,
            local_dir=str(models_dir / BASE_MODEL_LOCAL_NAME),
        )
        _snapshot_download(
            CODEC_REPO_ID,
            local_dir=str(models_dir / CODEC_LOCAL_NAME),
        )
    except Exception as e:  # noqa: BLE001
        _emit(f"⚠️  Model download failed: {e}", stderr=True)
        raise SystemExit(
            f"❌ Failed to download MOSS-TTS-Realtime weights.\n"
            f"Please check your network connection and try again.\n"
            f"Alternatively, manually download the following HF repos and place them "
            f"under '{models_dir}':\n"
            f"  - {BASE_MODEL_REPO_ID} -> {models_dir / BASE_MODEL_LOCAL_NAME}\n"
            f"  - {CODEC_REPO_ID} -> {models_dir / CODEC_LOCAL_NAME}"
        )


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    target_root = Path(args.target_root_dir).expanduser().resolve()
    target_dir = target_root / args.base_model
    target_root.mkdir(parents=True, exist_ok=True)

    if not target_dir.exists():
        try:
            _emit(f"📥 Cloning MOSS-TTS into target directory: {target_dir}")
            _clone_repo(args.repo_url, args.repo_branch, target_dir)
            _emit("✓ Clone completed successfully")
        except Exception as e:  # noqa: BLE001
            raise SystemExit(
                f"❌ Setup failed after direct clone: MOSS-TTS-Realtime runtime is incomplete, "
                f"target directory: {target_dir}\n\n"
                f"Troubleshooting:\n"
                f"  1. Ensure you have sufficient disk space\n"
                f"  2. Check your network connection\n"
                f"  3. Manually clone from: {args.repo_url}\n"
                f"  4. Retry later if GitHub service is unstable"
            ) from e
    else:
        _emit(f"✓ MOSS-TTS checkout already exists at {target_dir}; skip clone")

    _download_models(target_dir)
    _emit(f"✅ MOSS-TTS-Realtime is ready at {target_dir}")


if __name__ == "__main__":
    main(sys.argv[1:])
