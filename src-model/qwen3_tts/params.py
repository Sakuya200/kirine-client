from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
import json
from pathlib import Path


@dataclass
class Qwen3TrainingRuntimeOptions:
    device: str
    logging_dir: str
    attn_implementation: str


@dataclass
class Qwen3TrainingParams:
    base_model: str
    version: int
    init_model_path: str
    tokenizer_model_path: str
    input_jsonl: str
    output_jsonl: str
    output_model_path: str
    batch_size: int
    lr: float
    num_epochs: int
    speaker_name: str
    gradient_accumulation_steps: int
    enable_gradient_checkpointing: bool
    use_lora: bool
    runtime: Qwen3TrainingRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            output_model_path=self.output_model_path,
            train_jsonl=self.output_jsonl,
            logging_dir=self.runtime.logging_dir,
            batch_size=self.batch_size,
            lr=self.lr,
            num_epochs=self.num_epochs,
            speaker_name=self.speaker_name,
            gradient_accumulation_steps=self.gradient_accumulation_steps,
            enable_gradient_checkpointing=self.enable_gradient_checkpointing,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
            use_lora=self.use_lora,
        )


def _load_json(path: str | Path) -> dict[str, object]:
    params_path = Path(path).expanduser().resolve()
    if not params_path.exists():
        raise FileNotFoundError(f"Qwen3 training params file not found: {params_path}")

    with params_path.open("r", encoding="utf-8") as file:
        return json.load(file)


def _extract_task_args(payload: dict[str, object], task_name: str) -> dict[str, object]:
    raw_args = payload.get("args") or {}
    if not isinstance(raw_args, dict):
        raise TypeError("Malformed Qwen3 params payload: args must be an object")

    nested_args = raw_args.get(task_name)
    if not isinstance(nested_args, dict):
        raise TypeError(f"Malformed Qwen3 params payload: args.{task_name} must be an object")
    return nested_args


def _parse_learning_rate(value: str | None, default: float = 2e-5) -> float:
    if value is None:
        return default
    return float(value)


def load_training_params(path: str | Path) -> Qwen3TrainingParams:
    payload = _load_json(path)
    if payload.get("kind") != "Training":
        raise ValueError(f"Expected Training params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "Training")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed Qwen3 training params payload")

    return Qwen3TrainingParams(
        base_model=str(payload.get("base_model") or "qwen3_tts"),
        version=int(payload.get("version") or 1),
        init_model_path=str(args["init_model_path"]),
        tokenizer_model_path=str(args["tokenizer_model_path"]),
        input_jsonl=str(args["input_jsonl"]),
        output_jsonl=str(args["output_jsonl"]),
        output_model_path=str(args["output_model_path"]),
        batch_size=int(args["batch_size"]),
        lr=_parse_learning_rate(args.get("lr")),
        num_epochs=int(args["num_epochs"]),
        speaker_name=str(args["speaker_name"]),
        gradient_accumulation_steps=int(args["gradient_accumulation_steps"]),
        enable_gradient_checkpointing=bool(args["enable_gradient_checkpointing"]),
        use_lora=bool(args.get("use_lora", False)),
        runtime=Qwen3TrainingRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "flash_attention_2"),
        ),
    )


@dataclass
class Qwen3GenerationRuntimeOptions:
    device: str
    logging_dir: str
    attn_implementation: str


@dataclass
class Qwen3TtsParams:
    init_model_path: str
    text: str
    language: str
    speaker: str
    instruct: str
    output_path: str
    runtime: Qwen3GenerationRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            text=self.text,
            language=self.language,
            speaker=self.speaker,
            instruct=self.instruct,
            output_path=self.output_path,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
        )


@dataclass
class Qwen3VoiceCloneParams:
    ref_audio_path: str
    ref_text: str
    init_model_path: str
    language: str
    output_path: str
    text: str
    runtime: Qwen3GenerationRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            ref_audio_path=self.ref_audio_path,
            ref_text=self.ref_text,
            init_model_path=self.init_model_path,
            language=self.language,
            output_path=self.output_path,
            text=self.text,
            logging_dir=self.runtime.logging_dir,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
        )


def load_tts_params(path: str | Path) -> Qwen3TtsParams:
    payload = _load_json(path)
    if payload.get("kind") != "TextToSpeech":
        raise ValueError(f"Expected TextToSpeech params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "TextToSpeech")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed Qwen3 tts params payload")

    return Qwen3TtsParams(
        init_model_path=str(args["init_model_path"]),
        text=str(args["text"]),
        language=str(args.get("language") or "Auto"),
        speaker=str(args.get("speaker") or ""),
        instruct=str(args.get("instruct") or ""),
        output_path=str(args["output_path"]),
        runtime=Qwen3GenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "flash_attention_2"),
        ),
    )


def load_voice_clone_params(path: str | Path) -> Qwen3VoiceCloneParams:
    payload = _load_json(path)
    if payload.get("kind") != "VoiceClone":
        raise ValueError(f"Expected VoiceClone params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "VoiceClone")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed Qwen3 voice clone params payload")

    return Qwen3VoiceCloneParams(
        ref_audio_path=str(args["ref_audio_path"]),
        ref_text=str(args.get("ref_text") or ""),
        init_model_path=str(args["init_model_path"]),
        language=str(args.get("language") or "Auto"),
        output_path=str(args["output_path"]),
        text=str(args["text"]),
        runtime=Qwen3GenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "flash_attention_2"),
        ),
    )