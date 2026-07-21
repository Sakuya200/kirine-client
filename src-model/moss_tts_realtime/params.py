"""MOSS-TTS-Realtime 专用参数映射：把 ``ParamsEntity`` 转成入口脚本可消费的结构。

路径解析约定（详见 ``common.py``）：
- 基座/codec 权重：经 ``__file__`` 推导到克隆仓库 ``models/`` 下，不依赖 model_root_path。
- trained 说话人 checkpoint：``<model_root_path>/<speaker_dir_name>/checkpoint_final``，
  与训练时 ``output_model_path`` 指向同一目录。
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from common import (
    base_checkpoint,
    codec_path,
    normalize_device,
    resolve_trained_checkpoint,
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


@dataclass(frozen=True)
class MossRealtimeStreamingParams:
    context_file_path: str
    input_cache_file_path: str
    output_audio_dir: str
    model_root_path: str
    device: str
    temperature: float
    top_p: float
    top_k: int
    repetition_penalty: float
    repetition_window: int
    max_length: int


def load_streaming_params(path: str | Path) -> MossRealtimeStreamingParams:
    params = ParamsEntity.from_file(path)
    args = params.streaming_args()

    return MossRealtimeStreamingParams(
        context_file_path=args.context_file_path,
        input_cache_file_path=args.input_cache_file_path,
        output_audio_dir=args.output_audio_dir,
        model_root_path=args.model_root_path,
        device=normalize_device(params.runtime.device),
        temperature=_float_param(params, "temperature", 0.8),
        top_p=_float_param(params, "topP", 0.6),
        top_k=_int_param(params, "topK", 30),
        repetition_penalty=_float_param(params, "repetitionPenalty", 1.1),
        repetition_window=_int_param(params, "repetitionWindow", 50),
        max_length=_int_param(params, "maxLength", 32768),
    )


@dataclass(frozen=True)
class MossRealtimeTrainingParams:
    input_jsonl: str
    output_jsonl: str
    output_model_path: str
    base_checkpoint: Path
    codec_path: Path
    batch_size: int
    gradient_accumulation_steps: int
    learning_rate: str
    weight_decay: str
    warmup_ratio: str
    num_epochs: int
    mixed_precision: str
    max_grad_norm: str
    speaker_name: str
    device: str


def load_training_params(path: str | Path) -> MossRealtimeTrainingParams:
    params = ParamsEntity.from_file(path)
    args = params.training_args()

    return MossRealtimeTrainingParams(
        input_jsonl=args.input_jsonl,
        output_jsonl=args.output_jsonl,
        output_model_path=args.output_model_path,
        base_checkpoint=base_checkpoint(),
        codec_path=codec_path(),
        batch_size=int(args.batch_size),
        gradient_accumulation_steps=int(args.gradient_accumulation_steps),
        learning_rate=params.model_param_str("learningRate", "1e-5") or "1e-5",
        weight_decay=params.model_param_str("weightDecay", "0.1") or "0.1",
        warmup_ratio=params.model_param_str("warmupRatio", "0.03") or "0.03",
        num_epochs=int(args.num_epochs),
        mixed_precision=(params.model_param_str("mixedPrecision", "bf16") or "bf16"),
        max_grad_norm=params.model_param_str("maxGradNorm", "1.0") or "1.0",
        speaker_name=args.speaker_name,
        device=normalize_device(params.runtime.device),
    )


def resolve_trained_speaker_checkpoint(
    model_root_path: str, speaker_dir_name: str | None
) -> Path | None:
    """供 streaming.py 复用的 trained checkpoint 解析（透传 common）。"""
    return resolve_trained_checkpoint(model_root_path, speaker_dir_name)
