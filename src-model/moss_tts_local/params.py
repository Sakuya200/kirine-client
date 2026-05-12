from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
from pathlib import Path

from moss_tts_local.params_entity import CommonTaskArgs, ParamsEntity, RuntimeOptions


MOSS_MODEL_NAME = "MOSS-TTS-Local-Transformer"
MOSS_AUDIO_TOKENIZER_NAME = "MOSS-Audio-Tokenizer"


@dataclass
class MossTrainingParams:
    common: CommonTaskArgs
    init_model_path: str
    codec_path: str
    input_jsonl: str
    output_model_path: str
    batch_size: int
    gradient_accumulation_steps: int
    num_epochs: int
    learning_rate: float
    weight_decay: float
    warmup_ratio: float
    warmup_steps: int
    max_grad_norm: float
    mixed_precision: str
    enable_gradient_checkpointing: bool
    skip_reference_audio_codes: bool
    prep_batch_size: int
    prep_n_vq: int | None
    channelwise_loss_weight: str
    lr_scheduler_type: str
    runtime: RuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            train_jsonl=self.input_jsonl,
            init_model_path=self.init_model_path,
            codec_path=self.codec_path,
            output_model_path=self.output_model_path,
            device=self.runtime.device,
            batch_size=self.batch_size,
            gradient_accumulation_steps=self.gradient_accumulation_steps,
            num_epochs=self.num_epochs,
            learning_rate=self.learning_rate,
            weight_decay=self.weight_decay,
            warmup_ratio=self.warmup_ratio,
            warmup_steps=self.warmup_steps,
            max_grad_norm=self.max_grad_norm,
            mixed_precision=self.mixed_precision,
            enable_gradient_checkpointing=self.enable_gradient_checkpointing,
            skip_reference_audio_codes=self.skip_reference_audio_codes,
            prep_batch_size=self.prep_batch_size,
            prep_n_vq=self.prep_n_vq,
            channelwise_loss_weight=self.channelwise_loss_weight,
            lr_scheduler_type=self.lr_scheduler_type,
            logging_dir=self.runtime.logging_dir,
            attn_implementation=self.runtime.attn_implementation,
        )


@dataclass
class MossTtsParams:
    common: CommonTaskArgs
    init_model_path: str
    text: str
    language: str
    n_vq_for_inference: int
    output_path: str
    runtime: RuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            text=self.text,
            language=self.language,
            n_vq_for_inference=self.n_vq_for_inference,
            output_path=self.output_path,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
            max_new_tokens=8192,
        )


@dataclass
class MossVoiceCloneParams:
    common: CommonTaskArgs
    init_model_path: str
    text: str
    language: str
    n_vq_for_inference: int
    output_path: str
    ref_audio_path: str
    ref_text: str
    runtime: RuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            text=self.text,
            language=self.language,
            n_vq_for_inference=self.n_vq_for_inference,
            output_path=self.output_path,
            ref_audio_path=self.ref_audio_path,
            ref_text=self.ref_text,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
            max_new_tokens=8192,
        )
def _resolve_locator_candidate(
    common: CommonTaskArgs,
    default_leaf_name: str,
    *,
    prefer_speaker_dir_name: bool,
) -> str | None:
    if common.model_root_path is None:
        return None

    root_path = Path(common.model_root_path).expanduser().resolve()
    leaf_name = default_leaf_name
    if prefer_speaker_dir_name and common.speaker_dir_name:
        leaf_name = common.speaker_dir_name
        if leaf_name.strip().casefold() == "base-models":
            leaf_name = default_leaf_name
    return str((root_path / leaf_name).resolve())


def _require_resolved_path(path: str | None, label: str) -> str:
    if path is None:
        raise ValueError(f"MOSS params payload is missing a resolvable {label}")
    return path


def _resolve_moss_inference_root(common: CommonTaskArgs) -> Path:
    if common.model_root_path is None:
        raise ValueError("MOSS params payload is missing a resolvable inference model path")

    root_path = Path(common.model_root_path).expanduser().resolve()
    speaker_dir_name = common.speaker_dir_name
    if speaker_dir_name and speaker_dir_name.strip().casefold() == "base-models":
        speaker_dir_name = None

    if speaker_dir_name:
        speaker_root = (root_path / speaker_dir_name).resolve()
        bundled_root = (speaker_root / MOSS_MODEL_NAME).resolve()
        if bundled_root.is_dir():
            return bundled_root
        return speaker_root

    return (root_path / MOSS_MODEL_NAME).resolve()


def _resolve_training_model_path(common: CommonTaskArgs) -> str:
    candidate = _resolve_locator_candidate(
        common,
        MOSS_MODEL_NAME,
        prefer_speaker_dir_name=False,
    )
    return _require_resolved_path(candidate, "training model path")


def _resolve_inference_model_path(common: CommonTaskArgs) -> str:
    return str(_resolve_moss_inference_root(common))


def _resolve_codec_path(common: CommonTaskArgs) -> str:
    candidate = _resolve_locator_candidate(
        CommonTaskArgs(
            model_root_path=common.model_root_path,
            speaker_dir_name=None,
            model_params_json=common.model_params_json,
        ),
        MOSS_AUDIO_TOKENIZER_NAME,
        prefer_speaker_dir_name=False,
    )
    return _require_resolved_path(candidate, "codec path")


def _parse_float_with_default(value: str | None, default: float) -> float:
    if value is None:
        return default
    return float(value)


def _normalize_generation_runtime(runtime: RuntimeOptions) -> RuntimeOptions:
    return RuntimeOptions(
        device=runtime.device or "cuda:0",
        logging_dir=runtime.logging_dir or "",
        attn_implementation=runtime.attn_implementation or "auto",
    )


def _normalize_training_runtime(runtime: RuntimeOptions) -> RuntimeOptions:
    return RuntimeOptions(
        device=runtime.device or "cuda:0",
        logging_dir=runtime.logging_dir or "",
        attn_implementation=runtime.attn_implementation or "auto",
    )


def load_training_params(path: str | Path) -> MossTrainingParams:
    params = ParamsEntity.from_file(path)
    args = params.training_args()
    learning_rate = params.model_param_str("learningRate", args.lr)
    weight_decay = params.model_param_str("weightDecay", None)
    warmup_ratio = params.model_param_str("warmupRatio", None)
    max_grad_norm = params.model_param_str("maxGradNorm", None)
    mixed_precision = params.model_param_str("mixedPrecision", None)
    channelwise_loss_weight = params.model_param_str(
        "channelwiseLossWeight",
        None,
    )
    lr_scheduler_type = params.model_param_str("lrSchedulerType", None)

    return MossTrainingParams(
        common=args.common,
        init_model_path=_resolve_training_model_path(args.common),
        codec_path=_resolve_codec_path(args.common),
        input_jsonl=args.input_jsonl,
        output_model_path=args.output_model_path,
        batch_size=args.batch_size,
        gradient_accumulation_steps=args.gradient_accumulation_steps,
        num_epochs=args.num_epochs,
        learning_rate=_parse_float_with_default(learning_rate, 1e-5),
        weight_decay=_parse_float_with_default(weight_decay, 0.1),
        warmup_ratio=_parse_float_with_default(warmup_ratio, 0.03),
        warmup_steps=int(params.model_param_int("warmupSteps", None) or 0),
        max_grad_norm=_parse_float_with_default(max_grad_norm, 1.0),
        mixed_precision=str(mixed_precision or "bf16"),
        enable_gradient_checkpointing=params.model_param_bool(
            "enableGradientCheckpointing",
            args.enable_gradient_checkpointing,
        ),
        skip_reference_audio_codes=params.model_param_bool(
            "skipReferenceAudioCodes",
            True,
        ),
        prep_batch_size=int(params.model_param_int("prepBatchSize", None) or 16),
        prep_n_vq=params.model_param_int("prepNVq", None),
        channelwise_loss_weight=str(channelwise_loss_weight or "1,32"),
        lr_scheduler_type=str(lr_scheduler_type or "cosine"),
        runtime=_normalize_training_runtime(params.runtime),
    )


def load_tts_params(path: str | Path) -> MossTtsParams:
    params = ParamsEntity.from_file(path)
    args = params.tts_args()
    n_vq_for_inference = params.model_param_int(
        "nVqForInference",
        None,
    )

    return MossTtsParams(
        common=args.common,
        init_model_path=_resolve_inference_model_path(args.common),
        text=args.text,
        language=args.language or "auto",
        n_vq_for_inference=int(n_vq_for_inference or 8),
        output_path=args.output_path,
        runtime=_normalize_generation_runtime(params.runtime),
    )


def load_voice_clone_params(path: str | Path) -> MossVoiceCloneParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_clone_args()
    n_vq_for_inference = params.model_param_int(
        "nVqForInference",
        None,
    )

    return MossVoiceCloneParams(
        common=args.common,
        init_model_path=_resolve_inference_model_path(args.common),
        text=args.text,
        language=args.language or "auto",
        n_vq_for_inference=int(n_vq_for_inference or 8),
        output_path=args.output_path,
        ref_audio_path=args.ref_audio_path,
        ref_text=args.ref_text or "",
        runtime=_normalize_generation_runtime(params.runtime),
    )