"""MOSS-TTS-Realtime 专用参数映射：把 ``ParamsEntity`` 转成流式入口可消费的结构。

路径解析约定详见 ``common.py``：基座/codec 权重经 ``__file__`` 推导到克隆仓库
``models/`` 下；trained 说话人 checkpoint 由 ``common.resolve_trained_checkpoint``
解析，供 ``streaming.py`` 加载。
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from common import normalize_device
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


def _str_param(params: ParamsEntity, key: str, default: str) -> str:
    raw = params.model_param_str(key, default)
    if raw is None:
        return default
    normalized = str(raw).strip()
    return normalized if normalized else default


def _optional_bool_param(params: ParamsEntity, key: str) -> bool | None:
    raw = params.model_param(key, None)
    if raw is None:
        return None
    if isinstance(raw, bool):
        return raw
    if isinstance(raw, (int, float)):
        return bool(raw)
    if isinstance(raw, str):
        normalized = raw.strip().lower()
        if normalized in {"", "none", "null", "auto"}:
            return None
        if normalized in {"1", "true", "yes", "on"}:
            return True
        if normalized in {"0", "false", "no", "off"}:
            return False
    return None


@dataclass(frozen=True)
class MossRealtimeStreamingParams:
    context_file_path: str
    input_cache_file_path: str
    output_audio_dir: str
    model_root_path: str
    frames_file_path: str
    device: str
    temperature: float
    top_p: float
    top_k: int
    repetition_penalty: float
    repetition_window: int
    max_length: int
    enable_torch_compile: bool | None
    dtype: str
    float32_matmul_precision: str


def load_streaming_params(path: str | Path) -> MossRealtimeStreamingParams:
    params = ParamsEntity.from_file(path)
    args = params.streaming_args()

    return MossRealtimeStreamingParams(
        context_file_path=args.context_file_path,
        input_cache_file_path=args.input_cache_file_path,
        output_audio_dir=args.output_audio_dir,
        model_root_path=args.model_root_path,
        frames_file_path=args.frames_file_path,
        device=normalize_device(params.runtime.device),
        temperature=_float_param(params, "temperature", 0.8),
        top_p=_float_param(params, "topP", 0.6),
        top_k=_int_param(params, "topK", 30),
        repetition_penalty=_float_param(params, "repetitionPenalty", 1.1),
        repetition_window=_int_param(params, "repetitionWindow", 50),
        max_length=_int_param(params, "maxLength", 32768),
        enable_torch_compile=_optional_bool_param(params, "enableTorchCompile"),
        dtype=_str_param(params, "dtype", "auto").lower(),
        float32_matmul_precision=_str_param(params, "float32MatmulPrecision", "high").lower(),
    )
