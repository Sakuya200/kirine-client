"""IndexTTS Custom 下载脚本。

按 ``--model-version`` 分支，产出**版本自包含**的产物目录（与
``requiredModelNameList`` 一致，卸载某一版本不波及另一版本）：

``<target-root-dir>/index_tts_2`` 或 ``index_tts_25``
├── index-tts/       ← git clone https://github.com/index-tts/index-tts
└── checkpoints/     ← HF snapshot IndexTeam/IndexTTS-2 | IndexTTS-2.5
    └── hf_cache/    ← 辅助模型预取（上游 ensure_models_available 的同一批资源）

辅助模型若不预取，上游会在首次推理时联网补齐（w2v-bert-2.0 / MaskGCT
semantic codec / CAMPPlus / BigVGAN），任务执行期再走网络既慢又不可控，
故统一在本脚本完成。网络受限环境可设置 ``HF_ENDPOINT`` 环境变量走镜像
（如 ``https://hf-mirror.com``），huggingface_hub 会自动识别。

依赖安装不在本脚本内完成——由 ``init_task_runtime.ps1`` 通过适配器的
``requirements.txt`` / ``requirements-torch.txt`` 装入独立 conda_env/venv。
本脚本不做 editable install：``tts.py`` 运行时由 ``common.ensure_package_on_path()``
把 ``<artifact>/index-tts`` 注入 ``sys.path`` 以导入 ``indextts`` 包。
"""

from __future__ import annotations

import argparse
import shutil
import subprocess
import sys
import time
from pathlib import Path

DEFAULT_REPO_URL = "https://github.com/index-tts/index-tts"
DEFAULT_REPO_BRANCH = "main"

# model_version → HF 权重仓库。
VERSION_WEIGHT_REPO_IDS = {
    "2.0": "IndexTeam/IndexTTS-2",
    "2.5": "IndexTeam/IndexTTS-2.5",
}

# 辅助模型（对齐上游 indextts/utils/model_download.py 的 ensure_models_available）。
W2V_BERT_REPO_ID = "facebook/w2v-bert-2.0"
W2V_BERT_LOCAL_DIR_NAME = "w2v-bert-2.0"
CAMPPLUS_REPO_ID = "funasr/campplus"
CAMPPLUS_FILE = "campplus_cn_common.bin"
MASKGCT_REPO_ID = "amphion/MaskGCT"
MASKGCT_SEMANTIC_FILE = "semantic_codec/model.safetensors"
MASKGCT_SEMANTIC_LOCAL_NAME = "semantic_codec_model.safetensors"
BIGVGAN_REPO_ID = "nvidia/bigvgan_v2_22khz_80band_256x"
BIGVGAN_FILES = ("config.json", "bigvgan_generator.pt")
BIGVGAN_LOCAL_DIR_NAME = "bigvgan"


def _emit(message: str, *, stderr: bool = False) -> None:
    print(message, file=sys.stderr if stderr else sys.stdout, flush=True)


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


def _normalize_model_version(model_version: str) -> str:
    normalized = (model_version or "").strip().lower()
    if normalized in {"2.0", "2"}:
        return "2.0"
    if normalized == "2.5":
        return "2.5"
    raise SystemExit(
        f"❌ Unsupported IndexTTS model_version: {model_version}. Supported: 2.0, 2.5"
    )


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
    timeout = 900  # seconds (15 minutes)

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
                    f"❌ {error_msg}\n"
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


def _snapshot_download(repo_id: str, local_dir: Path) -> None:
    """幂等下载整个 HF 仓库到 local_dir（已存在的文件自动跳过/续传）。"""
    from huggingface_hub import snapshot_download  # pyright: ignore[reportMissingImports]

    _emit(f"📥 Downloading {repo_id} -> {local_dir} ...")
    snapshot_download(repo_id=repo_id, local_dir=str(local_dir))
    _emit(f"✓ {repo_id} downloaded")


def _download_aux_file(repo_id: str, filename: str, final_path: Path) -> None:
    """下载仓库内单个文件并规整为平铺路径（对齐上游 hf_cache 布局）。

    hf_hub_download 会在 local_dir 下保留仓库相对路径（如
    ``semantic_codec/model.safetensors``），故先落到暂存目录再移动。
    """
    if final_path.is_file():
        _emit(f"✓ Aux model already exists: {final_path.name}")
        return

    from huggingface_hub import hf_hub_download  # pyright: ignore[reportMissingImports]

    final_path.parent.mkdir(parents=True, exist_ok=True)
    staging_dir = final_path.parent / f".staging_{final_path.name}"
    _emit(f"📥 Downloading {repo_id}/{filename} -> {final_path} ...")
    downloaded = hf_hub_download(repo_id=repo_id, filename=filename, local_dir=str(staging_dir))
    shutil.move(str(downloaded), str(final_path))
    shutil.rmtree(staging_dir, ignore_errors=True)
    _emit(f"✓ {final_path.name} downloaded")


def _download_weights(model_version: str, checkpoints_dir: Path) -> None:
    repo_id = VERSION_WEIGHT_REPO_IDS[model_version]
    try:
        _snapshot_download(repo_id, checkpoints_dir)
    except Exception as e:  # noqa: BLE001
        _emit(f"⚠️  Model download failed: {e}", stderr=True)
        raise SystemExit(
            f"❌ Failed to download IndexTTS {model_version} weights from {repo_id}.\n"
            f"Please check your network connection and try again.\n"
            f"Network-restricted environments can set HF_ENDPOINT to a mirror, e.g.:\n"
            f"  set HF_ENDPOINT=https://hf-mirror.com\n"
            f"Alternatively, manually download the HF repo and place it under "
            f"'{checkpoints_dir}':\n"
            f"  - {repo_id}"
        )


def _download_aux_models(model_version: str, checkpoints_dir: Path) -> None:
    cache_dir = checkpoints_dir / "hf_cache"

    try:
        w2v_dir = cache_dir / W2V_BERT_LOCAL_DIR_NAME
        if w2v_dir.is_dir() and any(w2v_dir.iterdir()):
            _emit("✓ Aux model already exists: w2v-bert-2.0")
        else:
            _snapshot_download(W2V_BERT_REPO_ID, w2v_dir)

        _download_aux_file(
            CAMPPLUS_REPO_ID, CAMPPLUS_FILE, cache_dir / CAMPPLUS_FILE
        )
        for filename in BIGVGAN_FILES:
            _download_aux_file(
                BIGVGAN_REPO_ID, filename, cache_dir / BIGVGAN_LOCAL_DIR_NAME / filename
            )

        # MaskGCT semantic codec 仅 2.0 推理路径需要（2.5 使用权重仓库自带 codec.pth）。
        if model_version == "2.0":
            _download_aux_file(
                MASKGCT_REPO_ID,
                MASKGCT_SEMANTIC_FILE,
                cache_dir / MASKGCT_SEMANTIC_LOCAL_NAME,
            )
    except SystemExit:
        raise
    except Exception as e:  # noqa: BLE001
        _emit(f"⚠️  Aux model download failed: {e}", stderr=True)
        raise SystemExit(
            f"❌ Failed to download IndexTTS auxiliary models.\n"
            f"Network-restricted environments can set HF_ENDPOINT to a mirror, e.g.:\n"
            f"  set HF_ENDPOINT=https://hf-mirror.com\n"
            f"Alternatively, manually place them under '{cache_dir}' (see "
            f"upstream indextts/utils/model_download.py for the full list)."
        )


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    model_version = _normalize_model_version(args.model_version)

    # 与 common.py 的 VERSION_ARTIFACT_DIR_NAMES 保持一致；本脚本独立运行
    # （sys.path[0] 为适配器目录），不能依赖包内导入。
    artifact_dir_name = "index_tts_2" if model_version == "2.0" else "index_tts_25"
    target_root = Path(args.target_root_dir).expanduser().resolve()
    artifact_dir = target_root / artifact_dir_name
    repo_dir = artifact_dir / "index-tts"
    checkpoints_dir = artifact_dir / "checkpoints"
    target_root.mkdir(parents=True, exist_ok=True)

    if not repo_dir.exists():
        try:
            _emit(f"📥 Cloning index-tts into target directory: {repo_dir}")
            _clone_repo(args.repo_url, args.repo_branch, repo_dir)
            _emit("✓ Clone completed successfully")
        except SystemExit:
            raise
        except Exception as e:  # noqa: BLE001
            raise SystemExit(
                f"❌ Setup failed: IndexTTS runtime is incomplete, "
                f"target directory: {repo_dir}\n\n"
                f"Troubleshooting:\n"
                f"  1. Ensure you have sufficient disk space\n"
                f"  2. Check your network connection\n"
                f"  3. Manually clone from: {args.repo_url}\n"
                f"  4. Retry later if GitHub service is unstable"
            ) from e
    else:
        _emit(f"✓ index-tts checkout already exists at {repo_dir}; skip clone")

    _download_weights(model_version, checkpoints_dir)
    _download_aux_models(model_version, checkpoints_dir)
    _emit(f"✅ IndexTTS {model_version} is ready at {artifact_dir}")


if __name__ == "__main__":
    main(sys.argv[1:])
