from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
import json
from pathlib import Path


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


def _parse_float_with_default(value: str | None, default: float) -> float:
    if value is None:
        return default
    return float(value)


def load_training_params(path: str | Path) -> MossTrainingParams:
    payload = _load_json(path)
    if payload.get("kind") != "Training":
        raise ValueError(f"Expected Training params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "Training")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed MOSS training params payload")

    return MossTrainingParams(
        init_model_path=str(args["init_model_path"]),
        codec_path=str(args["codec_path"]),
        input_jsonl=str(args["input_jsonl"]),
        output_model_path=str(args["output_model_path"]),
        batch_size=int(args["batch_size"]),
        gradient_accumulation_steps=int(args["gradient_accumulation_steps"]),
        num_epochs=int(args["num_epochs"]),
        learning_rate=_parse_float_with_default(args.get("lr"), 1e-5),
        weight_decay=_parse_float_with_default(args.get("weight_decay"), 0.1),
        warmup_ratio=_parse_float_with_default(args.get("warmup_ratio"), 0.03),
        warmup_steps=int(args.get("warmup_steps") or 0),
        max_grad_norm=_parse_float_with_default(args.get("max_grad_norm"), 1.0),
        mixed_precision=str(args.get("mixed_precision") or "bf16"),
        enable_gradient_checkpointing=bool(args.get("enable_gradient_checkpointing", True)),
        skip_reference_audio_codes=bool(args.get("skip_reference_audio_codes", True)),
        prep_batch_size=int(args.get("prep_batch_size") or 16),
        prep_n_vq=int(args["prep_n_vq"]) if args.get("prep_n_vq") is not None else None,
        channelwise_loss_weight=str(args.get("channelwise_loss_weight") or "1,32"),
        lr_scheduler_type=str(args.get("lr_scheduler_type") or "cosine"),
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

    return MossTtsParams(
        init_model_path=str(args["init_model_path"]),
        text=str(args["text"]),
        language=str(args.get("language") or "auto"),
        n_vq_for_inference=int(args.get("n_vq_for_inference") or 8),
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

    return MossVoiceCloneParams(
        init_model_path=str(args["init_model_path"]),
        text=str(args["text"]),
        language=str(args.get("language") or "auto"),
        n_vq_for_inference=int(args.get("n_vq_for_inference") or 8),
        output_path=str(args["output_path"]),
        ref_audio_path=str(args["ref_audio_path"]),
        ref_text=str(args.get("ref_text") or ""),
        runtime=MossGenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "auto"),
        ),
    )