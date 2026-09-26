"""IndexTTS 适配器公共工具：版本产物目录解析、上游包路径注入与设备归一化。

上游项目经 ``download.py`` 落盘到 ``src-model/base-models/index_tts_{2|25}/``，
每个版本目录自包含（上游代码克隆 ``index-tts/`` + 权重 ``checkpoints/`` +
辅助模型 ``checkpoints/hf_cache/``），``requiredModelNameList`` 按此隔离卸载。
``indextts`` 包位于克隆仓库根顶层，``tts.py`` / ``voice_clone.py`` 运行时
（cwd 为 ``src-model/``，``sys.path[0]`` 为本适配器目录）需先调用
``ensure_package_on_path()`` 注入仓库根才能导入，无需 editable install。
"""

from __future__ import annotations

import sys
from pathlib import Path

# 适配器目录：src-model/index_tts/
_ADAPTER_DIR = Path(__file__).resolve().parent
# src-model/
_SRC_MODEL_ROOT = _ADAPTER_DIR.parent

BASE_MODEL_NAME = "index_tts"
MODEL_ARTIFACTS_DIR = "base-models"

# model_version → base-models/ 下自包含产物目录名（download.py 产出、卸载删除的单位）。
VERSION_ARTIFACT_DIR_NAMES = {
    "2.0": "index_tts_2",
    "2.5": "index_tts_25",
}

# 上游克隆仓库目录名（download.py git clone 产出，indextts 包位于其根顶层）。
UPSTREAM_REPO_DIR_NAME = "index-tts"
# 权重目录名（download.py HF snapshot 产出，IndexTTS2 的 model_dir）。
CHECKPOINTS_DIR_NAME = "checkpoints"


def normalize_model_version(model_version: str | None) -> str:
    """归一化 model_version 为 ``"2.0"`` / ``"2.5"``。"""
    normalized = (model_version or "").strip().lower()
    if normalized in {"2.0", "2"}:
        return "2.0"
    if normalized == "2.5":
        return "2.5"
    raise ValueError(
        f"Unsupported IndexTTS model_version: {model_version}. Supported values: 2.0, 2.5"
    )


def artifact_root(model_version: str) -> Path:
    """版本自包含产物目录：``src-model/base-models/index_tts_{2|25}``。"""
    return (
        _SRC_MODEL_ROOT
        / MODEL_ARTIFACTS_DIR
        / VERSION_ARTIFACT_DIR_NAMES[normalize_model_version(model_version)]
    )


def repo_root(model_version: str) -> Path:
    """上游 index-tts 克隆仓库根目录（``indextts`` 包所在处）。"""
    return artifact_root(model_version) / UPSTREAM_REPO_DIR_NAME


def checkpoints_dir(model_version: str) -> Path:
    """版本权重目录（IndexTTS2 的 model_dir，含 config.yaml 与 hf_cache/）。"""
    return artifact_root(model_version) / CHECKPOINTS_DIR_NAME


def ensure_package_on_path(model_version: str) -> Path:
    """将上游仓库根注入 ``sys.path``，返回该目录。

    入口脚本不在仓库根目录运行（cwd 为 ``src-model/``），``sys.path[0]`` 不含
    仓库根，故需在 ``import indextts`` 之前显式调用本函数。未下载时给出明确
    提示，而非抛出令人困惑的 ``ModuleNotFoundError``。
    """
    repo = repo_root(model_version)
    if not (repo / "indextts").is_dir():
        raise SystemExit(
            f"❌ 未找到 indextts 包目录: {repo}\n"
            f"请先在模型管理页下载 IndexTTS {normalize_model_version(model_version)} 模型。"
        )
    repo_str = str(repo)
    if repo_str not in sys.path:
        sys.path.insert(0, repo_str)
    return repo


def resolve_cfg_path(model_version: str) -> Path:
    """探测权重目录下的 config 文件。

    HF 权重仓库（IndexTeam/IndexTTS-2、IndexTTS-2.5）实际分发 ``config.yaml``；
    部分文档对 2.5 写作 ``config_v2_5.yaml``，故做双名探测兜底。
    """
    checkpoints = checkpoints_dir(model_version)
    for name in ("config.yaml", "config_v2_5.yaml"):
        candidate = checkpoints / name
        if candidate.is_file():
            return candidate
    raise FileNotFoundError(
        f"IndexTTS config file not found under: {checkpoints}\n"
        f"请先在模型管理页下载 IndexTTS {normalize_model_version(model_version)} 模型。"
    )


def aux_model_paths(model_dir: Path) -> dict[str, str]:
    """辅助模型路径表（2.0 构造 IndexTTS2 时经 ``aux_paths`` 显式传入）。

    上游 ``ensure_models_available`` 会在推理期联网补齐 w2v-bert-2.0 /
    MaskGCT semantic codec / CAMPPlus / BigVGAN 到 ``model_dir/hf_cache/``；
    ``download.py`` 已在下载阶段预取同一路径。2.0 的构造函数把
    ``aux_paths=None`` 视为触发联网探测，显式传值可完全绕过网络检测；
    2.5 的构造函数自行检查 ``hf_cache/`` 存在性，预取后同样不联网。
    """
    cache = model_dir / "hf_cache"
    w2v_bert = cache / "w2v-bert-2.0"
    semantic_codec = cache / "semantic_codec_model.safetensors"
    campplus = cache / "campplus_cn_common.bin"
    bigvgan = cache / "bigvgan"

    for path, label in (
        (w2v_bert, "w2v-bert-2.0"),
        (semantic_codec, "MaskGCT semantic codec"),
        (campplus, "CAMPPlus"),
        (bigvgan, "BigVGAN"),
    ):
        if not path.exists():
            raise FileNotFoundError(
                f"IndexTTS 辅助模型 {label} 不存在: {path}\n"
                f"请先在模型管理页下载 IndexTTS 模型。"
            )

    return {
        "w2v_bert": str(w2v_bert),
        "semantic_codec": str(semantic_codec),
        "campplus": str(campplus),
        "bigvgan": str(bigvgan),
    }


def normalize_device(device: str | None) -> str | None:
    """归一化设备参数。

    为空返回 None（交给上游 ``IndexTTS2`` 按 CUDA/XPU/MPS/CPU 可用性自动
    选择）；``cuda:*`` 统一为 ``cuda``。CPU 设备由上游自动禁用半精度。
    """
    if not device or not device.strip():
        return None
    normalized = device.strip().lower()
    if normalized.startswith("cuda"):
        return "cuda"
    return normalized
