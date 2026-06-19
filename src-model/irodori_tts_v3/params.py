"""Irodori-TTS 专用参数映射：把 ``ParamsEntity`` 转成入口脚本可消费的结构。

路径解析约定（详见 ``common.py``）：
- 基座/音色设计权重：经 ``__file__`` 推导，不依赖 ``model_root_path`` 的分支差异。
- 训练产物（说话人）：``<model_root_path>/<speaker_dir_name>/``，与训练时
  ``output_model_path`` 指向同一目录。
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from common import (
    base_checkpoint,
    voicedesign_checkpoint,
    resolve_speaker_artifact,
    normalize_device,
)
from params_entity import ParamsEntity


def _float_param(params: ParamsEntity, key: str, default: float) -> float:
    raw = params.model_param_str(key, str(default))
    if raw is None or str(raw).strip() == "":
        return default
    return float(raw)


def _int_param(params: ParamsEntity, key: str, default: int) -> int:
    raw = params.model_param_int(key, default)
    if raw is None:
        return default
    return int(raw)


def _optional_int_param(params: ParamsEntity, key: str) -> int | None:
    raw = params.model_param(key, None)
    if raw is None or str(raw).strip() == "":
        return None
    try:
        return int(raw)
    except (TypeError, ValueError):
        return None


@dataclass(frozen=True)
class SamplingParams:
    num_steps: int
    cfg_scale_text: float
    cfg_scale_caption: float
    cfg_scale_speaker: float
    model_precision: str
    duration_scale: float
    num_candidates: int
    seed: int | None


def _load_sampling(params: ParamsEntity, *, with_caption: bool) -> SamplingParams:
    return SamplingParams(
        num_steps=_int_param(params, "numSteps", 40),
        cfg_scale_text=_float_param(params, "cfgScaleText", 3.0),
        cfg_scale_caption=_float_param(params, "cfgScaleCaption", 3.0) if with_caption else 3.0,
        cfg_scale_speaker=_float_param(params, "cfgScaleSpeaker", 5.0),
        model_precision=(params.model_param_str("modelPrecision", "fp32") or "fp32"),
        duration_scale=_float_param(params, "durationScale", 1.0),
        num_candidates=_int_param(params, "numCandidates", 1),
        seed=_optional_int_param(params, "seed"),
    )


@dataclass(frozen=True)
class IrodoriTtsParams:
    text: str
    output_path: str
    base_checkpoint: Path
    speaker_kind: str | None  # "ref_embed" | "lora_adapter" | None
    speaker_path: Path | None
    sampling: SamplingParams
    device: str


def load_tts_params(path: str | Path) -> IrodoriTtsParams:
    params = ParamsEntity.from_file(path)
    args = params.tts_args()

    text = (args.text or "").strip()
    if not text:
        raise ValueError("Text cannot be empty.")

    artifact = resolve_speaker_artifact(
        args.common.model_root_path, args.common.speaker_dir_name
    )
    speaker_kind, speaker_path = (None, None)
    if artifact is not None:
        speaker_kind, speaker_path = artifact

    return IrodoriTtsParams(
        text=text,
        output_path=args.output_path,
        base_checkpoint=base_checkpoint(),
        speaker_kind=speaker_kind,
        speaker_path=speaker_path,
        sampling=_load_sampling(params, with_caption=False),
        device=normalize_device(params.runtime.device),
    )


@dataclass(frozen=True)
class IrodoriVoiceCloneParams:
    text: str
    ref_audio_path: str
    output_path: str
    base_checkpoint: Path
    sampling: SamplingParams
    device: str


def load_voice_clone_params(path: str | Path) -> IrodoriVoiceCloneParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_clone_args()

    text = (args.text or "").strip()
    if not text:
        raise ValueError("Text cannot be empty.")

    ref_audio_path = (args.ref_audio_path or "").strip()
    if not ref_audio_path:
        raise ValueError("Voice cloning requires a reference audio path.")
    if not Path(ref_audio_path).exists():
        raise FileNotFoundError(f"Reference audio file not found: {ref_audio_path}")

    return IrodoriVoiceCloneParams(
        text=text,
        ref_audio_path=str(Path(ref_audio_path).expanduser().resolve()),
        output_path=args.output_path,
        base_checkpoint=base_checkpoint(),
        sampling=_load_sampling(params, with_caption=False),
        device=normalize_device(params.runtime.device),
    )


@dataclass(frozen=True)
class IrodoriVoiceDesignParams:
    text: str
    caption: str
    output_path: str
    voicedesign_checkpoint: Path
    ref_audio_path: str | None
    sampling: SamplingParams
    device: str


def load_voice_design_params(path: str | Path) -> IrodoriVoiceDesignParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_design_args()

    text = (args.text or "").strip()
    if not text:
        raise ValueError("Text cannot be empty.")

    caption = (args.instruct or "").strip()

    ref_audio_raw = params.model_param_str("refAudioPath", "") or ""
    ref_audio_raw = ref_audio_raw.strip()
    ref_audio_path: str | None = None
    if ref_audio_raw:
        resolved = Path(ref_audio_raw).expanduser().resolve()
        if not resolved.exists():
            raise FileNotFoundError(f"Voice design reference audio not found: {resolved}")
        ref_audio_path = str(resolved)

    return IrodoriVoiceDesignParams(
        text=text,
        caption=caption,
        output_path=args.output_path,
        voicedesign_checkpoint=voicedesign_checkpoint(),
        ref_audio_path=ref_audio_path,
        sampling=_load_sampling(params, with_caption=True),
        device=normalize_device(params.runtime.device),
    )


@dataclass(frozen=True)
class IrodoriTrainingParams:
    input_jsonl: str
    output_jsonl: str
    output_model_path: str
    base_checkpoint: Path
    batch_size: int
    lr: str | None
    num_epochs: int
    gradient_accumulation_steps: int
    speaker_name: str
    training_mode: str
    lora_r: int
    lora_alpha: int
    max_steps: int | None
    enable_gradient_checkpointing: bool
    device: str


def load_training_params(path: str | Path) -> IrodoriTrainingParams:
    params = ParamsEntity.from_file(path)
    args = params.training_args()

    training_mode = (params.model_param_str("trainingMode", "speaker_inversion") or "speaker_inversion").strip()
    if training_mode not in {"speaker_inversion", "lora"}:
        raise ValueError(
            f"Unsupported trainingMode: {training_mode} "
            f"(expected 'speaker_inversion' or 'lora')"
        )

    return IrodoriTrainingParams(
        input_jsonl=args.input_jsonl,
        output_jsonl=args.output_jsonl,
        output_model_path=args.output_model_path,
        base_checkpoint=base_checkpoint(),
        batch_size=int(args.batch_size),
        lr=args.lr,
        num_epochs=int(args.num_epochs),
        gradient_accumulation_steps=int(args.gradient_accumulation_steps),
        speaker_name=args.speaker_name,
        training_mode=training_mode,
        lora_r=_int_param(params, "loraR", 16),
        lora_alpha=_int_param(params, "loraAlpha", 32),
        max_steps=_optional_int_param(params, "maxSteps"),
        enable_gradient_checkpointing=params.model_param_bool(
            "enableGradientCheckpointing", True
        ),
        device=normalize_device(params.runtime.device),
    )
