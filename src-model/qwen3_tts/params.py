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


QWEN3_TOKENIZER_NAME = "Qwen3-TTS-Tokenizer-12Hz"
QWEN3_VARIANT_MODEL_NAMES = {
    "1.7B": {
        "base": "Qwen3-TTS-12Hz-1.7B-Base",
        "custom": "Qwen3-TTS-12Hz-1.7B-CustomVoice",
    },
    "0.6B": {
        "base": "Qwen3-TTS-12Hz-0.6B-Base",
        "custom": "Qwen3-TTS-12Hz-0.6B-CustomVoice",
    },
}


@dataclass
class Qwen3TrainingRuntimeOptions:
    device: str
    logging_dir: str
    attn_implementation: str


@dataclass
class Qwen3TrainingParams:
    base_model: str
    version: int
    common: CommonTaskArgs
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
    runtime: Qwen3TrainingRuntimeOptions

    def to_namespace(self) -> Namespace:
        return Namespace(
            init_model_path=self.init_model_path,
            output_model_path=self.output_model_path,
            output_jsonl=self.output_jsonl,
            logging_dir=self.runtime.logging_dir,
            batch_size=self.batch_size,
            lr=self.lr,
            num_epochs=self.num_epochs,
            speaker_name=self.speaker_name,
            gradient_accumulation_steps=self.gradient_accumulation_steps,
            enable_gradient_checkpointing=self.enable_gradient_checkpointing,
            device=self.runtime.device,
            attn_implementation=self.runtime.attn_implementation,
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


def _parse_common_task_args(args: dict[str, object]) -> CommonTaskArgs:
    raw_model_params = args.get("model_params_json")
    if raw_model_params is None:
        model_params_json: dict[str, object] = {}
    elif isinstance(raw_model_params, dict):
        model_params_json = raw_model_params
    else:
        raise TypeError("Malformed Qwen3 params payload: model_params_json must be an object")

    return CommonTaskArgs(
        model_root_path=str(args["model_root_path"]) if args.get("model_root_path") is not None else None,
        speaker_dir_name=str(args["speaker_dir_name"]) if args.get("speaker_dir_name") is not None else None,
        model_params_json=model_params_json,
    )


def _resolve_path_value(path: str | None) -> str | None:
    if path is None:
        return None
    return str(Path(path).expanduser().resolve())


def _resolve_locator_candidate(
    common: CommonTaskArgs,
    default_leaf_name: str | None,
    *,
    prefer_speaker_dir_name: bool,
) -> str | None:
    if common.model_root_path is None:
        return None

    root_path = Path(common.model_root_path).expanduser().resolve()
    leaf_name = default_leaf_name
    if prefer_speaker_dir_name and common.speaker_dir_name:
        leaf_name = common.speaker_dir_name
    if leaf_name is None:
        return None

    return str((root_path / leaf_name).resolve())


def _require_resolved_path(path: str | None, label: str) -> str:
    if path is None:
        raise ValueError(f"Qwen3 params payload is missing a resolvable {label}")
    return path


def _resolve_latest_qwen3_checkpoint(model_root_path: Path) -> Path:
    if not model_root_path.is_dir():
        return model_root_path.resolve()

    checkpoint_dirs: list[tuple[int, Path]] = []
    for entry in model_root_path.iterdir():
        if not entry.is_dir():
            continue

        name = entry.name
        if not name.startswith("checkpoint-epoch-"):
            continue

        try:
            epoch = int(name.removeprefix("checkpoint-epoch-"))
        except ValueError:
            continue

        checkpoint_dirs.append((epoch, entry.resolve()))

    checkpoint_dirs.sort(key=lambda item: item[0])
    if checkpoint_dirs:
        return checkpoint_dirs[-1][1]

    return model_root_path.resolve()


def _infer_qwen3_model_scale(common: CommonTaskArgs) -> str:
    raw_model_scale = common.model_params_json.get("modelScale")
    if raw_model_scale is not None:
        model_scale = str(raw_model_scale).strip()
        if model_scale in QWEN3_VARIANT_MODEL_NAMES:
            return model_scale

    raise ValueError("Qwen3 params payload is missing a supported modelScale value")


def _resolve_qwen3_inference_model_path(common: CommonTaskArgs) -> str:
    model_scale = _infer_qwen3_model_scale(common)
    candidate = _resolve_locator_candidate(
        common,
        QWEN3_VARIANT_MODEL_NAMES[model_scale]["custom"],
        prefer_speaker_dir_name=True,
    )
    inference_root = Path(_require_resolved_path(candidate, "inference model path"))
    return str(_resolve_latest_qwen3_checkpoint(inference_root))


def _resolve_qwen3_training_model_path(common: CommonTaskArgs) -> str:
    model_scale = _infer_qwen3_model_scale(common)
    candidate = _resolve_locator_candidate(
        common,
        QWEN3_VARIANT_MODEL_NAMES[model_scale]["base"],
        prefer_speaker_dir_name=False,
    )
    return _require_resolved_path(candidate, "training model path")


def _resolve_qwen3_tokenizer_model_path(common: CommonTaskArgs) -> str:
    candidate = _resolve_locator_candidate(
        common,
        QWEN3_TOKENIZER_NAME,
        prefer_speaker_dir_name=False,
    )
    return _require_resolved_path(candidate, "tokenizer model path")


def _parse_learning_rate(value: str | None, default: float = 2e-5) -> float:
    if value is None:
        return default
    return float(value)


def _model_param_str(common: CommonTaskArgs, key: str, fallback: str = "") -> str:
    value = common.model_params_json.get(key)
    if value is None:
        return fallback
    return str(value)


def load_training_params(path: str | Path) -> Qwen3TrainingParams:
    payload = _load_json(path)
    if payload.get("kind") != "Training":
        raise ValueError(f"Expected Training params payload, got: {payload.get('kind')}")

    runtime = payload.get("runtime") or {}
    args = _extract_task_args(payload, "Training")
    if not isinstance(runtime, dict) or not isinstance(args, dict):
        raise TypeError("Malformed Qwen3 training params payload")

    common = _parse_common_task_args(args)
    learning_rate = _model_param_str(common, "learningRate", str(args.get("lr") or "")) or None

    return Qwen3TrainingParams(
        base_model=str(payload.get("base_model") or "qwen3_tts"),
        version=int(payload.get("version") or 1),
        common=common,
        init_model_path=_resolve_qwen3_training_model_path(common),
        tokenizer_model_path=_resolve_qwen3_tokenizer_model_path(common),
        input_jsonl=str(args["input_jsonl"]),
        output_jsonl=str(args["output_jsonl"]),
        output_model_path=str(args["output_model_path"]),
        batch_size=int(args["batch_size"]),
        lr=_parse_learning_rate(learning_rate),
        num_epochs=int(args["num_epochs"]),
        speaker_name=str(args["speaker_name"]),
        gradient_accumulation_steps=int(args["gradient_accumulation_steps"]),
        enable_gradient_checkpointing=bool(args["enable_gradient_checkpointing"]),
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
    common: CommonTaskArgs
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
    common: CommonTaskArgs
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

    common = _parse_common_task_args(args)

    return Qwen3TtsParams(
        common=common,
        init_model_path=_resolve_qwen3_inference_model_path(common),
        text=str(args["text"]),
        language=str(args.get("language") or "Auto"),
        speaker=str(args.get("speaker") or ""),
        instruct=_model_param_str(common, "voicePrompt", ""),
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

    common = _parse_common_task_args(args)

    return Qwen3VoiceCloneParams(
        common=common,
        ref_audio_path=str(args["ref_audio_path"]),
        ref_text=str(args.get("ref_text") or ""),
        init_model_path=_resolve_qwen3_training_model_path(common),
        language=str(args.get("language") or "Auto"),
        output_path=str(args["output_path"]),
        text=str(args["text"]),
        runtime=Qwen3GenerationRuntimeOptions(
            device=str(runtime.get("device") or "cuda:0"),
            logging_dir=str(runtime.get("logging_dir") or ""),
            attn_implementation=str(runtime.get("attn_implementation") or "flash_attention_2"),
        ),
    )