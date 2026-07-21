"""MOSS-TTS-Realtime 适配器公共工具：路径解析与 subprocess 调用上游 finetuning 脚本。

上游项目经 ``download.py`` 克隆到 ``src-model/base-models/moss_tts_realtime/``
（= OpenMOSS/MOSS-TTS 仓库根）。本模块脚本由 ``begin_llm_task.ps1`` 用适配器共享的
conda_env/venv 解释器运行；subprocess 调用上游 ``moss_tts_realtime/finetuning/``
下脚本时用 ``sys.executable`` 保证同一解释器，并以仓库根目录为 ``cwd``，使
``mossttsrealtime`` 包经 ``sys.path[0]`` 自动可导入（无需 editable install）。
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

# 适配器目录：src-model/moss_tts_realtime/
_ADAPTER_DIR = Path(__file__).resolve().parent
# src-model/
_SRC_MODEL_ROOT = _ADAPTER_DIR.parent

BASE_MODEL_NAME = "moss_tts_realtime"
MODEL_ARTIFACTS_DIR = "base-models"

# 上游克隆仓库内权重目录名（download.py 把 HF 权重放到 repo_root()/models/<NAME>）。
BASE_CHECKPOINT_DIR_NAME = "MOSS-TTS-Realtime"
CODEC_DIR_NAME = "MOSS-Audio-Tokenizer"
# 微调产物规整后的 canonical 目录名，streaming.py 加载 trained 说话人时识别它。
CHECKPOINT_FINAL_DIR = "checkpoint_final"


def repo_root() -> Path:
    """上游 OpenMOSS/MOSS-TTS 克隆仓库根目录（``base-models/moss_tts_realtime``）。"""
    return _SRC_MODEL_ROOT / MODEL_ARTIFACTS_DIR / BASE_MODEL_NAME


def _require_existing(path: Path, label: str) -> Path:
    if not path.exists():
        raise FileNotFoundError(
            f"{label} 不存在: {path}\n"
            f"请先在模型管理页安装 MOSS-TTS-Realtime 模型。"
        )
    return path


def base_checkpoint() -> Path:
    """基座权重路径，用于流式基座合成 / 训练初始化。"""
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

    训练时后端把产物写入 ``output_model_path = <model_dir>/<speaker_id>``；
    流式时后端给出 ``model_root_path = <model_dir>``、``speaker_dir_name = <speaker_id>``。
    两者拼接即训练产物目录。不存在返回 None。
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


def run_upstream_script(script_rel: str, args: list[str]) -> None:
    """以当前解释器运行上游 finetuning 脚本（相对仓库根的路径），流式转发输出。

    ``script_rel`` 形如 ``moss_tts_realtime/finetuning/sft.py``。cwd=仓库根，使上游
    ``mosssttsrealtime`` 包经 ``sys.path[0]`` 可导入（sft.py 自带 REPO_ROOT 注入）。
    """
    script_path = repo_root() / script_rel
    _require_existing(script_path, f"上游脚本 {script_rel}")

    cmd = [sys.executable, "-u", str(script_path), *args]
    print(f"[moss_tts_realtime] run {' '.join(cmd)}", flush=True)
    print(f"[moss_tts_realtime] cwd={repo_root()}", flush=True)

    process = subprocess.Popen(
        cmd,
        cwd=str(repo_root()),
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        bufsize=1,
    )
    assert process.stdout is not None
    try:
        for line in process.stdout:
            sys.stdout.write(line)
            sys.stdout.flush()
    finally:
        process.stdout.close()
        process.wait()

    if process.returncode != 0:
        raise SystemExit(
            f"❌ {script_rel} 退出码 {process.returncode}，任务失败。"
        )
