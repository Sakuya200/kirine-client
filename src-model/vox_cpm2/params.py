from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
import json
from pathlib import Path


@dataclass
class VoxCpm2TrainingRuntimeOptions:
    device: str
    logging_dir: str


@dataclass
class VoxCpm2TrainingParams:
    base_model: str
    version: int
    init_model_path: str
    input_jsonl: str
    output_model_path: str
    batch_size: int
    lr: float
    num_epochs: int
    gradient_accumulation_steps: int
    enable_gradient_checkpointing: bool
    use_lora: bool
    training_mode: str
    lora_rank: int | None
    lora_alpha: int | None
    lora_dropout: str | None
    weight_decay: float | None
    warmup_steps: int | None
    runtime: VoxCpm2TrainingRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            train_jsonl=self.input_jsonl,
            output_model_path=self.output_model_path,
            init_model_path=self.init_model_path,
            logging_dir=self.runtime.logging_dir,
            batch_size=self.batch_size,
            num_epochs=self.num_epochs,
            device=self.runtime.device,
            gradient_accumulation_steps=self.gradient_accumulation_steps,
            training_mode=self.training_mode,
            lora_rank=self.lora_rank if self.lora_rank is not None else 32,
            lora_alpha=self.lora_alpha if self.lora_alpha is not None else 32,
            lora_dropout=self.lora_dropout if self.lora_dropout is not None else "0.0",
            learning_rate=self.lr,
            weight_decay=self.weight_decay,
            warmup_steps=self.warmup_steps,
            enable_gradient_checkpointing=self.enable_gradient_checkpointing,
            train_script_path="",
        )


def _load_json(path: str | Path) -> dict[str, object]:
    params_path = Path(path).expanduser().resolve()
    if not params_path.exists():
        raise FileNotFoundError(f"VoxCPM2 training params file not found: {params_path}")

    with params_path.open("r", encoding="utf-8") as file:
        return json.load(file)


def _extract_task_args(payload: dict[str, object], task_name: str) -> dict[str, object]:
    raw_args = payload.get("args") or {}
    if not isinstance(raw_args, dict):
        raise TypeError("Malformed VoxCPM2 params payload: args must be an object")

    nested_args = raw_args.get(task_name)
    if not isinstance(nested_args, dict):
        raise TypeError(f"Malformed VoxCPM2 params payload: args.{task_name} must be an object")
    return nested_args


def _parse_optional_float(value: str | None) -> float | None:
    if value is None:
        return None
    return float(value)


def _parse_float_with_default(value: str | None, default: float) -> float:
    return default if value is None else float(value)


def load_training_params(path: str | Path) -> VoxCpm2TrainingParams:
    payload = _load_json(path)
    if payload.get("kind") != "Training":
        raise ValueError(f"Expected Training params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "Training")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed VoxCPM2 training params payload")

    return VoxCpm2TrainingParams(
        base_model=str(payload.get("base_model") or "vox_cpm2"),
        version=int(payload.get("version") or 1),
        init_model_path=str(args["init_model_path"]),
        input_jsonl=str(args["input_jsonl"]),
        output_model_path=str(args["output_model_path"]),
        batch_size=int(args["batch_size"]),
        lr=_parse_float_with_default(args.get("lr"), 1e-4),
        num_epochs=int(args["num_epochs"]),
        gradient_accumulation_steps=int(args["gradient_accumulation_steps"]),
        enable_gradient_checkpointing=bool(args["enable_gradient_checkpointing"]),
        use_lora=bool(args.get("use_lora", False)),
        training_mode=str(args.get("training_mode") or ("lora" if args.get("use_lora") else "full")),
        lora_rank=int(args["lora_rank"]) if args.get("lora_rank") is not None else None,
        lora_alpha=int(args["lora_alpha"]) if args.get("lora_alpha") is not None else None,
        lora_dropout=str(args["lora_dropout"]) if args.get("lora_dropout") is not None else None,
        weight_decay=_parse_optional_float(args.get("weight_decay")),
        warmup_steps=int(args["warmup_steps"]) if args.get("warmup_steps") is not None else None,
        runtime=VoxCpm2TrainingRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
        ),
    )


@dataclass
class VoxCpm2GenerationRuntimeOptions:
    device: str
    logging_dir: str


@dataclass
class VoxCpm2TtsParams:
    init_model_path: str
    text: str
    output_path: str
    cfg_value: float
    inference_timesteps: int
    runtime: VoxCpm2GenerationRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            text=self.text,
            cfg_value=self.cfg_value,
            inference_timesteps=self.inference_timesteps,
            output_path=self.output_path,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
        )


@dataclass
class VoxCpm2VoiceCloneParams:
    init_model_path: str
    mode: str
    ref_audio_path: str
    ref_text: str
    text: str
    style_prompt: str
    cfg_value: float
    inference_timesteps: int
    language: str
    output_path: str
    runtime: VoxCpm2GenerationRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            mode=self.mode,
            ref_audio_path=self.ref_audio_path,
            ref_text=self.ref_text,
            text=self.text,
            style_prompt=self.style_prompt,
            cfg_value=self.cfg_value,
            inference_timesteps=self.inference_timesteps,
            language=self.language,
            output_path=self.output_path,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
        )


def load_tts_params(path: str | Path) -> VoxCpm2TtsParams:
    payload = _load_json(path)
    if payload.get("kind") != "TextToSpeech":
        raise ValueError(f"Expected TextToSpeech params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "TextToSpeech")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed VoxCPM2 tts params payload")

    return VoxCpm2TtsParams(
        init_model_path=str(args["init_model_path"]),
        text=str(args["text"]),
        output_path=str(args["output_path"]),
        cfg_value=_parse_float_with_default(args.get("cfg_value"), 2.0),
        inference_timesteps=int(args.get("inference_timesteps") or 10),
        runtime=VoxCpm2GenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
        ),
    )


def load_voice_clone_params(path: str | Path) -> VoxCpm2VoiceCloneParams:
    payload = _load_json(path)
    if payload.get("kind") != "VoiceClone":
        raise ValueError(f"Expected VoiceClone params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "VoiceClone")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed VoxCPM2 voice clone params payload")

    return VoxCpm2VoiceCloneParams(
        init_model_path=str(args["init_model_path"]),
        mode=str(args.get("mode") or "reference"),
        ref_audio_path=str(args["ref_audio_path"]),
        ref_text=str(args.get("ref_text") or ""),
        text=str(args["text"]),
        style_prompt=str(args.get("style_prompt") or ""),
        cfg_value=_parse_float_with_default(args.get("cfg_value"), 2.0),
        inference_timesteps=int(args.get("inference_timesteps") or 10),
        language=str(args.get("language") or "auto"),
        output_path=str(args["output_path"]),
        runtime=VoxCpm2GenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
        ),
    )