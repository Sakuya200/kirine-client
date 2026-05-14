from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
from pathlib import Path

from qwen3_tts.params_entity import CommonTaskArgs, ParamsEntity, RuntimeOptions


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
class Qwen3TrainingParams:
    base_model: str
    version: str
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
    runtime: RuntimeOptions

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
        if leaf_name.strip().casefold() == "base-models":
            leaf_name = default_leaf_name
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
def _normalize_runtime(runtime: RuntimeOptions) -> RuntimeOptions:
    return RuntimeOptions(
        device=runtime.device or "cuda:0",
        logging_dir=runtime.logging_dir or "",
        attn_implementation=runtime.attn_implementation or "flash_attention_2",
    )


def _infer_qwen3_model_version(params: ParamsEntity) -> str:
    raw_model_version = params.model_version
    if raw_model_version:
        model_version = str(raw_model_version).strip()
        if model_version in QWEN3_VARIANT_MODEL_NAMES:
            return model_version

    raw_model_version = params.model_param_str("modelVersion")
    if raw_model_version is not None:
        model_version = str(raw_model_version).strip()
        if model_version in QWEN3_VARIANT_MODEL_NAMES:
            return model_version

    raise ValueError("Qwen3 params payload is missing a supported modelVersion value")


def _resolve_qwen3_inference_model_path(
    params: ParamsEntity,
) -> str:
    common = params.common
    model_version = _infer_qwen3_model_version(params)
    candidate = _resolve_locator_candidate(
        common,
        QWEN3_VARIANT_MODEL_NAMES[model_version]["custom"],
        prefer_speaker_dir_name=True,
    )
    inference_root = Path(_require_resolved_path(candidate, "inference model path"))
    return str(_resolve_latest_qwen3_checkpoint(inference_root))


def _resolve_qwen3_training_model_path(
    params: ParamsEntity,
) -> str:
    common = params.common
    model_version = _infer_qwen3_model_version(params)
    candidate = _resolve_locator_candidate(
        common,
        QWEN3_VARIANT_MODEL_NAMES[model_version]["base"],
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


def load_training_params(path: str | Path) -> Qwen3TrainingParams:
    params = ParamsEntity.from_file(path)
    args = params.training_args()
    learning_rate = params.model_param_str("learningRate", args.lr or "") or None

    return Qwen3TrainingParams(
        base_model=params.base_model or "qwen3_tts",
        version=params.version,
        common=args.common,
        init_model_path=_resolve_qwen3_training_model_path(params),
        tokenizer_model_path=_resolve_qwen3_tokenizer_model_path(args.common),
        input_jsonl=args.input_jsonl,
        output_jsonl=args.output_jsonl,
        output_model_path=args.output_model_path,
        batch_size=args.batch_size,
        lr=_parse_learning_rate(learning_rate),
        num_epochs=args.num_epochs,
        speaker_name=args.speaker_name,
        gradient_accumulation_steps=args.gradient_accumulation_steps,
        enable_gradient_checkpointing=params.model_param_bool(
            "enableGradientCheckpointing",
            False,
        ),
        runtime=_normalize_runtime(params.runtime),
    )


@dataclass
class Qwen3TtsParams:
    common: CommonTaskArgs
    init_model_path: str
    text: str
    language: str
    speaker: str
    instruct: str
    output_path: str
    runtime: RuntimeOptions

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
    runtime: RuntimeOptions

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
    params = ParamsEntity.from_file(path)
    args = params.tts_args()

    return Qwen3TtsParams(
        common=args.common,
        init_model_path=_resolve_qwen3_inference_model_path(params),
        text=args.text,
        language=args.language or "Auto",
        speaker=args.speaker or "",
        instruct=params.model_param_str("voicePrompt", "") or "",
        output_path=args.output_path,
        runtime=_normalize_runtime(params.runtime),
    )


def load_voice_clone_params(path: str | Path) -> Qwen3VoiceCloneParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_clone_args()

    return Qwen3VoiceCloneParams(
        common=args.common,
        ref_audio_path=args.ref_audio_path,
        ref_text=args.ref_text or "",
        init_model_path=_resolve_qwen3_training_model_path(params),
        language=args.language or "Auto",
        output_path=args.output_path,
        text=args.text,
        runtime=_normalize_runtime(params.runtime),
    )