"""MOSS-TTS-Realtime 适配器公共工具：路径解析、包路径注入与设备归一化。

上游项目经 ``download.py`` 克隆到 ``src-model/base-models/moss_tts_realtime/``
（= OpenMOSS/MOSS-TTS 仓库根）。``mossttsrealtime`` 包位于仓库根的 ``moss_tts_realtime/``
子目录下。本适配器脚本（``streaming.py``）由 ``begin_llm_task.ps1`` 以 ``src-model/``
为 cwd 运行，``sys.path[0]`` 为脚本目录（``src-model/moss_tts_realtime/``），既非仓库根
亦非包目录，故 ``mossttsrealtime`` 不会自动可导入--需在加载模型前显式调用
``ensure_package_on_path()`` 把包目录注入 ``sys.path``（无需 editable install）。
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# 适配器目录：src-model/moss_tts_realtime/
_ADAPTER_DIR = Path(__file__).resolve().parent
# src-model/
_SRC_MODEL_ROOT = _ADAPTER_DIR.parent

BASE_MODEL_NAME = "moss_tts_realtime"
MODEL_ARTIFACTS_DIR = "base-models"

# 随应用分发的 ffmpeg shared build 解压目录名。NSIS 把 ffmpeg-8.1.2.zip 解压到
# src-model 同级目录（<lib>/ffmpeg-8.1.2/，与 src-model 并列），并规整 zip 顶层目录名。
FFMPEG_DIR_NAME = "ffmpeg-8.1.2"

# 上游克隆仓库内权重目录名（download.py 把 HF 权重放到 repo_root()/models/<NAME>）。
BASE_CHECKPOINT_DIR_NAME = "MOSS-TTS-Realtime"
CODEC_DIR_NAME = "MOSS-Audio-Tokenizer"
# 微调产物规整后的 canonical 目录名，streaming.py 加载 trained 说话人时识别它。
CHECKPOINT_FINAL_DIR = "checkpoint_final"


def repo_root() -> Path:
    """上游 OpenMOSS/MOSS-TTS 克隆仓库根目录（``base-models/moss_tts_realtime``）。"""
    return _SRC_MODEL_ROOT / MODEL_ARTIFACTS_DIR / BASE_MODEL_NAME


def package_dir() -> Path:
    """``mossttsrealtime`` 包所在目录（``repo_root()/moss_tts_realtime``）。

    上游仓库结构为 ``<repo_root>/moss_tts_realtime/mossttsrealtime/``，包并不在仓库根
    目录顶层，故即便 cwd=repo_root 也无法直接 ``import mossttsrealtime``；必须把本目录
    注入 ``sys.path``。
    """
    return repo_root() / "moss_tts_realtime"


def ensure_package_on_path() -> Path:
    """将 ``mossttsrealtime`` 包目录注入 ``sys.path``，返回该目录。

    ``streaming.py`` 不在仓库根目录运行（cwd 为 ``src-model/``），``sys.path[0]`` 不含
    包目录，故需在 ``import mossttsrealtime`` 之前显式调用本函数。未下载时给出明确提示，
    而非抛出令人困惑的 ``ModuleNotFoundError``。
    """
    pkg = package_dir()
    if not pkg.is_dir():
        raise SystemExit(
            f"❌ 未找到 mossttsrealtime 包目录: {pkg}\n"
            f"请先在模型管理页下载 MOSS-TTS-Realtime 模型。"
        )
    pkg_str = str(pkg)
    if pkg_str not in sys.path:
        sys.path.insert(0, pkg_str)
    return pkg


def ensure_ffmpeg_dlls() -> Path:
    """把随应用分发的 ffmpeg shared build 的 ``bin`` 注入 DLL 搜索路径。

    moss_tts_realtime 经 torchaudio/torchcodec 加载 ffmpeg 共享库（avcodec 等），
    Python 3.8+ 的安全 DLL 搜索不会自动扫 PATH，故需在导入 torch 音频后端前显式
    ``os.add_dll_directory``。ffmpeg 由 NSIS 安装钩子解压到 **src-model 同级目录**
    （``_SRC_MODEL_ROOT.parent/ffmpeg-8.1.2/bin``，与 src-model 并列）。目录缺失即报错。
    """
    ffmpeg_bin = _SRC_MODEL_ROOT.parent / FFMPEG_DIR_NAME / "bin"
    if not ffmpeg_bin.is_dir():
        raise SystemExit(
            f"❌ 未找到 ffmpeg shared build 目录: {ffmpeg_bin}\n"
            f"moss_tts_realtime 需要随应用分发的 ffmpeg 共享库，请重新安装应用以解压 ffmpeg。"
        )
    try:
        os.add_dll_directory(str(ffmpeg_bin))
    except OSError as exc:
        raise SystemExit(f"❌ 注册 ffmpeg DLL 目录失败: {ffmpeg_bin}\n{exc}") from exc
    print(
        f"[moss_tts_realtime] Added DLL directory for ffmpeg: {ffmpeg_bin}",
        file=sys.stderr,
        flush=True,
    )
    return ffmpeg_bin


def _require_existing(path: Path, label: str) -> Path:
    if not path.exists():
        raise FileNotFoundError(
            f"{label} 不存在: {path}\n"
            f"请先在模型管理页安装 MOSS-TTS-Realtime 模型。"
        )
    return path


def base_checkpoint() -> Path:
    """基座权重路径，用于流式基座合成。"""
    return _require_existing(
        repo_root() / "models" / BASE_CHECKPOINT_DIR_NAME,
        "MOSS-TTS-Realtime 基座权重",
    )


def codec_path() -> Path:
    """MOSS-Audio-Tokenizer 编解码器路径（voice prompt 编码 + 音频解码）。"""
    return _require_existing(
        repo_root() / "models" / CODEC_DIR_NAME,
        "MOSS-Audio-Tokenizer 编解码器",
    )


def resolve_speaker_dir(model_root_path: str | None, speaker_dir_name: str | None) -> Path | None:
    """训练产物目录 ``<model_root_path>/<speaker_dir_name>``。

    流式时后端给出 ``model_root_path = <model_dir>``、``speaker_dir_name = <speaker_id>``，
    两者拼接即已训练说话人 checkpoint 所在目录。不存在返回 None。
    """
    if not model_root_path or not speaker_dir_name:
        return None
    candidate = Path(model_root_path).expanduser().resolve() / speaker_dir_name
    return candidate if candidate.exists() else None


def resolve_trained_checkpoint(
    model_root_path: str | None, speaker_dir_name: str | None
) -> Path | None:
    """解析 trained 说话人微调 checkpoint：``<speaker_dir>/checkpoint_final``。

    目录存在则返回其路径，供 ``MossTTSRealtime.from_pretrained`` 加载；否则 None。
    """
    speaker_dir = resolve_speaker_dir(model_root_path, speaker_dir_name)
    if speaker_dir is None:
        return None
    final = speaker_dir / CHECKPOINT_FINAL_DIR
    return final if final.is_dir() else None


def normalize_device(device: str | None) -> str:
    """归一化设备参数。MOSS-TTS-Realtime 仅支持 CUDA，但仍保留 cpu 分支以对齐约定。"""
    if not device:
        return "cuda"
    normalized = device.strip().lower()
    if normalized.startswith("cuda"):
        return "cuda"
    if normalized == "cpu":
        return "cpu"
    return normalized
