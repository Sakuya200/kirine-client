"""Irodori-TTS 适配器公共工具：路径解析与 subprocess 调用上游 CLI。

上游项目经 ``download.py`` 克隆到 ``src-model/base-models/irodori_tts_v3/``，
``infer.py`` / ``train.py`` 位于该克隆仓库根目录。本模块的所有脚本由
``begin_llm_task.ps1`` 用适配器共享的 conda_env/venv 解释器运行；subprocess 调用
上游脚本时使用 ``sys.executable`` 保证同一解释器，并以仓库根目录为 ``cwd``，使
``irodori_tts`` 包经 ``sys.path[0]`` 自动可导入（无需 editable install）。
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

# 适配器目录：src-model/irodori_tts_v3/
_ADAPTER_DIR = Path(__file__).resolve().parent
# src-model/
_SRC_MODEL_ROOT = _ADAPTER_DIR.parent

BASE_MODEL_NAME = "irodori_tts_v3"
MODEL_ARTIFACTS_DIR = "base-models"

BASE_CHECKPOINT_REPO_NAME = "Irodori-TTS-500M-v3"
VOICE_DESIGN_CHECKPOINT_REPO_NAME = "Irodori-TTS-600M-v3-VoiceDesign"
CHECKPOINT_FILENAME = "model.safetensors"

SPEAKER_INVERSION_SUFFIX = ".speaker.safetensors"
SPEAKER_INVERSION_FINAL_NAME = f"checkpoint_final{SPEAKER_INVERSION_SUFFIX}"
LORA_ADAPTER_FINAL_DIR = "checkpoint_final"


def repo_root() -> Path:
    """上游 Irodori-TTS 克隆仓库根目录（``base-models/irodori_tts_v3``）。"""
    return _SRC_MODEL_ROOT / MODEL_ARTIFACTS_DIR / BASE_MODEL_NAME


def _require_existing(path: Path, label: str) -> Path:
    if not path.exists():
        raise FileNotFoundError(
            f"{label} 不存在: {path}\n"
            f"请先在模型管理页安装 Irodori-TTS-V3 模型。"
        )
    return path


def base_checkpoint() -> Path:
    """基座权重（speaker-conditioned）路径，用于 TTS / 声音克隆 / 训练初始化。"""
    return _require_existing(
        repo_root() / "models" / BASE_CHECKPOINT_REPO_NAME / CHECKPOINT_FILENAME,
        "Irodori-TTS-500M-v3 基座权重",
    )


def voicedesign_checkpoint() -> Path:
    """音色设计权重（caption-conditioned）路径。"""
    return _require_existing(
        repo_root()
        / "models"
        / VOICE_DESIGN_CHECKPOINT_REPO_NAME
        / CHECKPOINT_FILENAME,
        "Irodori-TTS-600M-v3-VoiceDesign 音色设计权重",
    )


def resolve_speaker_dir(model_root_path: str | None, speaker_dir_name: str | None) -> Path | None:
    """训练产物目录。

    训练时后端将产物写入 ``output_model_path = <model_dir>/<speaker_id>``；
    TTS 时后端给出 ``model_root_path = <model_dir>``、``speaker_dir_name = <speaker_id>``。
    两者拼起来即训练产物目录 ``<model_root_path>/<speaker_dir_name>``。
    """
    if not model_root_path or not speaker_dir_name:
        return None
    candidate = Path(model_root_path).expanduser().resolve() / speaker_dir_name
    return candidate if candidate.exists() else None


def resolve_speaker_artifact(
    model_root_path: str | None, speaker_dir_name: str | None
) -> tuple[str, Path] | None:
    """解析训练产物，返回 ``(kind, path)``。

    - Speaker Inversion：``checkpoint_final.speaker.safetensors`` → ``("ref_embed", path)``
    - LoRA：``checkpoint_final/`` 适配器目录 → ``("lora_adapter", path)``
    """
    speaker_dir = resolve_speaker_dir(model_root_path, speaker_dir_name)
    if speaker_dir is None:
        return None

    ref_embed = speaker_dir / SPEAKER_INVERSION_FINAL_NAME
    if ref_embed.is_file():
        return ("ref_embed", ref_embed)

    lora_dir = speaker_dir / LORA_ADAPTER_FINAL_DIR
    if lora_dir.is_dir():
        return ("lora_adapter", lora_dir)

    return None


def normalize_device(device: str | None) -> str:
    """归一化设备参数为上游 CLI 接受的值（``cuda`` / ``cpu``）。"""
    if not device:
        return "cuda"
    normalized = device.strip().lower()
    if normalized.startswith("cuda"):
        return "cuda"
    if normalized == "cpu":
        return "cpu"
    return normalized


def _run_upstream_script(script_name: str, args: list[str]) -> None:
    """以当前解释器运行上游 ``infer.py`` / ``train.py``，流式转发输出。"""
    script_path = repo_root() / script_name
    _require_existing(script_path, f"上游脚本 {script_name}")

    cmd = [sys.executable, "-u", str(script_path), *args]
    print(f"[irodori_tts] run {' '.join(cmd)}", flush=True)
    print(f"[irodori_tts] cwd={repo_root()}", flush=True)

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
            f"❌ {script_name} 退出码 {process.returncode}，任务失败。"
        )


def run_infer(args: list[str]) -> None:
    """调用上游 ``infer.py``（声音克隆 / 音色设计 / 说话人 TTS）。"""
    _run_upstream_script("infer.py", args)


def run_train(args: list[str]) -> None:
    """调用上游 ``train.py``（LoRA / Speaker Inversion 微调）。"""
    _run_upstream_script("train.py", args)
