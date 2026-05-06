from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
import json
from pathlib import Path


@dataclass
class CommonTaskArgs:
    model_root_path: str | None
    speaker_dir_name: str | None
    model_params_json: dict[str, object]


MOSS_MODEL_NAME = "MOSS-TTS-Local-Transformer"
MOSS_AUDIO_TOKENIZER_NAME = "MOSS-Audio-Tokenizer"


@dataclass
class MossGenerationRuntimeOptions:
    device: str
    logging_dir: str
    attn_implementation: str


@dataclass
class MossTrainingRuntimeOptions:
    device: str
    logging_dir: str


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
    runtime: MossTrainingRuntimeOptions

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
        )


@dataclass
class MossTtsParams:
    common: CommonTaskArgs
    init_model_path: str
    text: str
    language: str
    n_vq_for_inference: int
    output_path: str
    runtime: MossGenerationRuntimeOptions

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
    runtime: MossGenerationRuntimeOptions

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


def _load_json(path: str | Path) -> dict[str, object]:
    params_path = Path(path).expanduser().resolve()
    if not params_path.exists():
        raise FileNotFoundError(f"MOSS params file not found: {params_path}")

    with params_path.open("r", encoding="utf-8") as file:
        return json.load(file)


def _extract_task_args(payload: dict[str, object], task_name: str) -> dict[str, object]:
    raw_args = payload.get("args") or {}
    if not isinstance(raw_args, dict):
        raise TypeError("Malformed MOSS params payload: args must be an object")

    nested_args = raw_args.get(task_name)
    if not isinstance(nested_args, dict):
        raise TypeError(f"Malformed MOSS params payload: args.{task_name} must be an object")
    return nested_args


def _parse_common_task_args(args: dict[str, object]) -> CommonTaskArgs:
    raw_model_params = args.get("model_params_json")
    if raw_model_params is None:
        model_params_json: dict[str, object] = {}
    elif isinstance(raw_model_params, dict):
        model_params_json = raw_model_params
    else:
        raise TypeError("Malformed MOSS params payload: model_params_json must be an object")

    return CommonTaskArgs(
        model_root_path=str(args["model_root_path"]) if args.get("model_root_path") is not None else None,
        speaker_dir_name=str(args["speaker_dir_name"]) if args.get("speaker_dir_name") is not None else None,
        model_params_json=model_params_json,
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
    return str((root_path / leaf_name).resolve())


def _require_resolved_path(path: str | None, label: str) -> str:
    if path is None:
        raise ValueError(f"MOSS params payload is missing a resolvable {label}")
    return path


def _resolve_moss_inference_root(common: CommonTaskArgs) -> Path:
    if common.model_root_path is None:
        raise ValueError("MOSS params payload is missing a resolvable inference model path")

    root_path = Path(common.model_root_path).expanduser().resolve()
    if common.speaker_dir_name:
        speaker_root = (root_path / common.speaker_dir_name).resolve()
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


def _model_param(common: CommonTaskArgs, key: str):
    return common.model_params_json.get(key)


def _model_param_str(common: CommonTaskArgs, key: str, fallback: str | None) -> str | None:
    value = _model_param(common, key)
    if value is None:
        return fallback
    return str(value)


def _model_param_int(common: CommonTaskArgs, key: str, fallback: int | None) -> int | None:
    value = _model_param(common, key)
    if value is None:
        return fallback
    return int(value)


def _model_param_bool(common: CommonTaskArgs, key: str, fallback: bool) -> bool:
    value = _model_param(common, key)
    if value is None:
        return fallback
    return bool(value)


def load_training_params(path: str | Path) -> MossTrainingParams:
    payload = _load_json(path)
    if payload.get("kind") != "Training":
        raise ValueError(f"Expected Training params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "Training")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed MOSS training params payload")

    common = _parse_common_task_args(args)
    learning_rate = _model_param_str(common, "learningRate", args.get("lr"))
    weight_decay = _model_param_str(common, "weightDecay", None)
    warmup_ratio = _model_param_str(common, "warmupRatio", None)
    max_grad_norm = _model_param_str(common, "maxGradNorm", None)
    mixed_precision = _model_param_str(common, "mixedPrecision", None)
    channelwise_loss_weight = _model_param_str(
        common,
        "channelwiseLossWeight",
        None,
    )
    lr_scheduler_type = _model_param_str(common, "lrSchedulerType", None)

    return MossTrainingParams(
        common=common,
        init_model_path=_resolve_training_model_path(common),
        codec_path=_resolve_codec_path(common),
        input_jsonl=str(args["input_jsonl"]),
        output_model_path=str(args["output_model_path"]),
        batch_size=int(args["batch_size"]),
        gradient_accumulation_steps=int(args["gradient_accumulation_steps"]),
        num_epochs=int(args["num_epochs"]),
        learning_rate=_parse_float_with_default(learning_rate, 1e-5),
        weight_decay=_parse_float_with_default(weight_decay, 0.1),
        warmup_ratio=_parse_float_with_default(warmup_ratio, 0.03),
        warmup_steps=int(_model_param_int(common, "warmupSteps", None) or 0),
        max_grad_norm=_parse_float_with_default(max_grad_norm, 1.0),
        mixed_precision=str(mixed_precision or "bf16"),
        enable_gradient_checkpointing=_model_param_bool(
            common,
            "enableGradientCheckpointing",
            bool(args.get("enable_gradient_checkpointing", True)),
        ),
        skip_reference_audio_codes=_model_param_bool(
            common,
            "skipReferenceAudioCodes",
            True,
        ),
        prep_batch_size=int(_model_param_int(common, "prepBatchSize", None) or 16),
        prep_n_vq=_model_param_int(common, "prepNVq", None),
        channelwise_loss_weight=str(channelwise_loss_weight or "1,32"),
        lr_scheduler_type=str(lr_scheduler_type or "cosine"),
        runtime=MossTrainingRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
        ),
    )


def load_tts_params(path: str | Path) -> MossTtsParams:
    payload = _load_json(path)
    if payload.get("kind") != "TextToSpeech":
        raise ValueError(f"Expected TextToSpeech params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "TextToSpeech")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed MOSS tts params payload")

    common = _parse_common_task_args(args)
    n_vq_for_inference = _model_param_int(
        common,
        "nVqForInference",
        None,
    )

    return MossTtsParams(
        common=common,
        init_model_path=_resolve_inference_model_path(common),
        text=str(args["text"]),
        language=str(args.get("language") or "auto"),
        n_vq_for_inference=int(n_vq_for_inference or 8),
        output_path=str(args["output_path"]),
        runtime=MossGenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "auto"),
        ),
    )


def load_voice_clone_params(path: str | Path) -> MossVoiceCloneParams:
    payload = _load_json(path)
    if payload.get("kind") != "VoiceClone":
        raise ValueError(f"Expected VoiceClone params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "VoiceClone")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed MOSS voice clone params payload")

    common = _parse_common_task_args(args)
    n_vq_for_inference = _model_param_int(
        common,
        "nVqForInference",
        None,
    )

    return MossVoiceCloneParams(
        common=common,
        init_model_path=_resolve_inference_model_path(common),
        text=str(args["text"]),
        language=str(args.get("language") or "auto"),
        n_vq_for_inference=int(n_vq_for_inference or 8),
        output_path=str(args["output_path"]),
        ref_audio_path=str(args["ref_audio_path"]),
        ref_text=str(args.get("ref_text") or ""),
        runtime=MossGenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "auto"),
        ),
    )