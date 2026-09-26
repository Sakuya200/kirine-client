from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from index_tts import common
from index_tts.params_entity import ParamsEntity

INDEX_TTS_BASE_MODEL = "index_tts"

SUPPORTED_MODEL_VERSIONS = ("2.0", "2.5")

# 任务页语言枚举 → IndexTTS 2.5 的 lang 参数（2.0 中英自动识别，忽略该值）。
LANGUAGE_MAP = {
    "chinese": "ZH",
    "english": "EN",
    "japanese": "JA",
}

EMOTION_SOURCES = ("none", "audio", "text", "vector")

# 八维情感向量字段，顺序必须与上游 [高兴, 愤怒, 悲伤, 恐惧, 厌恶, 忧郁, 惊讶, 平静] 一致。
EMOTION_VECTOR_FIELDS = (
    "emoJoy",
    "emoAnger",
    "emoSadness",
    "emoFear",
    "emoDisgust",
    "emoMelancholy",
    "emoSurprise",
    "emoCalm",
)

# 官方约束：每个分量 0.0-1.0，总和不超过 0.8。
EMOTION_VECTOR_MAX_SUM = 0.8

DURATION_FACTOR_RANGE = (0.5, 2.0)
EMO_ALPHA_RANGE = (0.0, 1.0)


def _parse_model_param_float(params: ParamsEntity, key: str, default: float) -> float:
    value = params.model_param(key, default)
    if isinstance(value, bool):
        return float(default)
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        return float(value.strip())
    return float(default)


def _parse_model_param_bool(params: ParamsEntity, key: str, default: bool) -> bool:
    return params.model_param_bool(key, default)


def _require_audio_file(path: str | None, label: str) -> Path:
    if path is None or not str(path).strip():
        raise ValueError(f"IndexTTS params payload is missing {label}")
    resolved = Path(path).expanduser().resolve()
    if not resolved.is_file():
        raise FileNotFoundError(f"Reference audio file not found: {resolved}")
    return resolved


def _resolve_output_path(output_path: str) -> Path:
    resolved = Path(output_path).expanduser().resolve()
    resolved.parent.mkdir(parents=True, exist_ok=True)
    return resolved


def _map_runtime_language(language: str | None) -> str:
    normalized = (language or "chinese").strip().lower()
    if normalized not in LANGUAGE_MAP:
        raise ValueError(f"Unsupported IndexTTS language: {language}")
    return LANGUAGE_MAP[normalized]


def _load_emotion(params: ParamsEntity) -> "EmotionParams":
    source = (params.model_param_str("emotionSource", "none") or "none").strip().lower()
    if source not in EMOTION_SOURCES:
        raise ValueError(
            f"Unsupported emotionSource: {source}. Supported values: {', '.join(EMOTION_SOURCES)}"
        )

    alpha = _parse_model_param_float(params, "emoAlpha", 1.0)
    if not EMO_ALPHA_RANGE[0] <= alpha <= EMO_ALPHA_RANGE[1]:
        raise ValueError(
            f"emoAlpha must be between {EMO_ALPHA_RANGE[0]} and {EMO_ALPHA_RANGE[1]}, got: {alpha}"
        )

    if source == "audio":
        audio_path = _require_audio_file(
            params.model_param_str("emoAudioPath", None), "emoAudioPath"
        )
        return EmotionParams(source="audio", audio_path=audio_path, text=None, vector=None, alpha=alpha)

    if source == "text":
        text = (params.model_param_str("emoText", None) or "").strip()
        if not text:
            raise ValueError("emotionSource 为 text 时，情感描述文本不能为空。")
        return EmotionParams(source="text", audio_path=None, text=text, vector=None, alpha=alpha)

    if source == "vector":
        vector = tuple(
            _parse_model_param_float(params, field, 0.0) for field in EMOTION_VECTOR_FIELDS
        )
        for field, value in zip(EMOTION_VECTOR_FIELDS, vector):
            if not 0.0 <= value <= 1.0:
                raise ValueError(
                    f"情感向量分量 {field} 须在 0.0 到 1.0 之间，当前值: {value}"
                )
        if sum(vector) > EMOTION_VECTOR_MAX_SUM + 1e-9:
            raise ValueError(
                f"八维情感向量总和须不超过 {EMOTION_VECTOR_MAX_SUM}，当前总和: {sum(vector):.2f}"
            )
        return EmotionParams(source="vector", audio_path=None, text=None, vector=vector, alpha=alpha)

    return EmotionParams(source="none", audio_path=None, text=None, vector=None, alpha=alpha)


@dataclass(frozen=True)
class EmotionParams:
    source: str
    audio_path: Path | None
    text: str | None
    vector: tuple[float, ...] | None
    alpha: float


@dataclass(frozen=True)
class IndexTtsParams:
    model_version: str
    cfg_path: Path
    model_dir: Path
    device: str | None
    use_half_precision: bool
    use_qwen_emo: bool
    ref_audio_path: Path
    target_text: str
    lang: str
    emotion: EmotionParams
    use_random: bool
    duration_factor: float
    output_path: Path


def _load_index_tts_params(
    params: ParamsEntity,
    *,
    ref_audio_path: str | None,
    text: str,
    language: str | None,
    output_path: str,
) -> IndexTtsParams:
    model_version = common.normalize_model_version(params.model_version)
    emotion = _load_emotion(params)

    duration_factor = _parse_model_param_float(params, "durationFactor", 1.0)
    if not DURATION_FACTOR_RANGE[0] <= duration_factor <= DURATION_FACTOR_RANGE[1]:
        raise ValueError(
            f"durationFactor must be between {DURATION_FACTOR_RANGE[0]} and "
            f"{DURATION_FACTOR_RANGE[1]}, got: {duration_factor}"
        )

    if not text.strip():
        raise ValueError("Input text cannot be empty.")

    return IndexTtsParams(
        model_version=model_version,
        cfg_path=common.resolve_cfg_path(model_version),
        model_dir=common.checkpoints_dir(model_version),
        device=common.normalize_device(params.runtime.device),
        use_half_precision=_parse_model_param_bool(params, "halfPrecision", True),
        # 情感文本模式需要 QwenEmotion（权重随版本仓库分发），其余模式不加载以省显存。
        use_qwen_emo=emotion.source == "text",
        ref_audio_path=_require_audio_file(ref_audio_path, "refAudioPath"),
        target_text=text.strip(),
        lang=_map_runtime_language(language),
        emotion=emotion,
        use_random=_parse_model_param_bool(params, "useRandom", False),
        duration_factor=duration_factor,
        output_path=_resolve_output_path(output_path),
    )


def load_tts_params(path: str | Path) -> IndexTtsParams:
    params = ParamsEntity.from_file(path)
    args = params.tts_args()
    return _load_index_tts_params(
        params,
        ref_audio_path=params.model_param_str("refAudioPath", None),
        text=args.text,
        language=args.language,
        output_path=args.output_path,
    )


def load_voice_clone_params(path: str | Path) -> IndexTtsParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_clone_args()
    # IndexTTS 无需参考音频文本（自动转写），args.ref_text 忽略。
    return _load_index_tts_params(
        params,
        ref_audio_path=params.model_param_str("refAudioPath", args.ref_audio_path),
        text=args.text,
        language=args.language,
        output_path=args.output_path,
    )


def _build_infer_kwargs(params: IndexTtsParams) -> dict[str, object]:
    infer_kwargs: dict[str, object] = {
        "spk_audio_prompt": str(params.ref_audio_path),
        "text": params.target_text,
        "output_path": str(params.output_path),
        "emo_alpha": params.emotion.alpha,
        "use_random": params.use_random,
        "verbose": True,
    }

    if params.emotion.source == "audio":
        assert params.emotion.audio_path is not None
        infer_kwargs["emo_audio_prompt"] = str(params.emotion.audio_path)
    elif params.emotion.source == "text":
        assert params.emotion.text is not None
        infer_kwargs["use_emo_text"] = True
        infer_kwargs["emo_text"] = params.emotion.text
    elif params.emotion.source == "vector":
        assert params.emotion.vector is not None
        infer_kwargs["emo_vector"] = list(params.emotion.vector)

    if params.model_version == "2.5":
        # lang / duration_factor 为 2.5 独有参数；2.0 中英自动识别、无时长控制。
        infer_kwargs["lang"] = params.lang
        infer_kwargs["duration_factor"] = params.duration_factor

    return infer_kwargs


def run_inference(params: IndexTtsParams) -> Path:
    """进程内直调上游 Python API 完成推理，返回输出文件路径。

    上游 ``indextts2`` CLI 仅支持 2.0 且写用户级持久化配置，与任务级目录
    体系冲突，故不采用。torch / indextts 均在函数体内延迟导入：参数校验
    失败可以在不加载模型的情况下快速报错退出。
    """
    # sys.path 注入必须在 import indextts 之前完成。
    common.ensure_package_on_path(params.model_version)

    if params.model_version == "2.0":
        from indextts.infer_v2 import IndexTTS2  # pyright: ignore[reportMissingImports]

        model = IndexTTS2(
            cfg_path=str(params.cfg_path),
            model_dir=str(params.model_dir),
            use_fp16=params.use_half_precision,
            use_cuda_kernel=False,
            use_deepspeed=False,
            use_qwen_emo=params.use_qwen_emo,
            aux_paths=common.aux_model_paths(params.model_dir),
            device=params.device,
        )
    else:
        # 注意：infer_v2_5 模块导入时会把 HF_HUB_CACHE 改写为 cwd 相对路径；
        # aux 模型已由 download.py 预取到 model_dir/hf_cache/，构造函数发现
        # 文件齐全即不联网，该副作用无实际影响。
        from indextts.infer_v2_5 import IndexTTS2  # pyright: ignore[reportMissingImports]

        model = IndexTTS2(
            cfg_path=str(params.cfg_path),
            model_dir=str(params.model_dir),
            use_bf16=params.use_half_precision,
            use_cuda_kernel=False,
            use_deepspeed=False,
            use_qwen_emo=params.use_qwen_emo,
            device=params.device,
        )

    print(f"[index_tts] InferTTS2 {params.model_version} loaded, start inference...", flush=True)
    model.infer(**_build_infer_kwargs(params))

    if not params.output_path.is_file():
        raise FileNotFoundError(f"IndexTTS output file not found: {params.output_path}")
    print(f"[index_tts] Inference finished: {params.output_path}", flush=True)
    return params.output_path
