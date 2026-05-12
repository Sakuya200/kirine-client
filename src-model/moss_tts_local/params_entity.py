from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
import json
from pathlib import Path


class TaskKind(str, Enum):
    TRAINING = "Training"
    TEXT_TO_SPEECH = "TextToSpeech"
    VOICE_CLONE = "VoiceClone"

    @classmethod
    def from_value(cls, value: object) -> "TaskKind":
        normalized = "" if value is None else str(value).strip()
        for member in cls:
            if member.value == normalized:
                return member

        raise ValueError(f"Unsupported params payload kind: {value}")


def _expect_mapping(value: object, label: str, *, allow_none: bool = False) -> dict[str, object]:
    if value is None and allow_none:
        return {}
    if isinstance(value, dict):
        return value
    raise TypeError(f"Malformed params payload: {label} must be an object")


def _coerce_optional_str(value: object) -> str | None:
    if value is None:
        return None
    return str(value)


def _coerce_required_str(value: object, label: str) -> str:
    if value is None:
        raise ValueError(f"Malformed params payload: {label} is required")
    return str(value)


def _coerce_required_int(value: object, label: str) -> int:
    if value is None:
        raise ValueError(f"Malformed params payload: {label} is required")
    return int(value)


def _coerce_optional_bool(value: object, fallback: bool = False) -> bool:
    if value is None:
        return fallback
    if isinstance(value, bool):
        return value
    if isinstance(value, (int, float)):
        return bool(value)
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in {"", "0", "false", "no", "off"}:
            return False
        if normalized in {"1", "true", "yes", "on"}:
            return True
    raise TypeError("Malformed params payload: boolean field has invalid value")


@dataclass(frozen=True)
class RuntimeOptions:
    device: str | None
    logging_dir: str | None
    attn_implementation: str | None

    @classmethod
    def from_mapping(cls, value: object) -> "RuntimeOptions":
        runtime = _expect_mapping(value, "runtime", allow_none=True)
        return cls(
            device=_coerce_optional_str(runtime.get("device")),
            logging_dir=_coerce_optional_str(runtime.get("logging_dir")),
            attn_implementation=_coerce_optional_str(runtime.get("attn_implementation")),
        )


@dataclass(frozen=True)
class CommonTaskArgs:
    model_root_path: str | None
    speaker_dir_name: str | None
    model_params_json: dict[str, object]

    @classmethod
    def from_mapping(cls, value: dict[str, object]) -> "CommonTaskArgs":
        raw_model_params = value.get("model_params_json")
        if raw_model_params is None:
            model_params_json: dict[str, object] = {}
        elif isinstance(raw_model_params, dict):
            model_params_json = raw_model_params
        else:
            raise TypeError(
                "Malformed params payload: model_params_json must be an object"
            )

        return cls(
            model_root_path=_coerce_optional_str(value.get("model_root_path")),
            speaker_dir_name=_coerce_optional_str(value.get("speaker_dir_name")),
            model_params_json=model_params_json,
        )


@dataclass(frozen=True)
class TrainingArgs:
    common: CommonTaskArgs
    input_jsonl: str
    output_jsonl: str
    output_model_path: str
    batch_size: int
    lr: str | None
    num_epochs: int
    speaker_name: str
    gradient_accumulation_steps: int
    enable_gradient_checkpointing: bool

    @classmethod
    def from_mapping(cls, value: dict[str, object]) -> "TrainingArgs":
        return cls(
            common=CommonTaskArgs.from_mapping(value),
            input_jsonl=_coerce_required_str(value.get("input_jsonl"), "args.Training.input_jsonl"),
            output_jsonl=_coerce_required_str(value.get("output_jsonl"), "args.Training.output_jsonl"),
            output_model_path=_coerce_required_str(
                value.get("output_model_path"),
                "args.Training.output_model_path",
            ),
            batch_size=_coerce_required_int(value.get("batch_size"), "args.Training.batch_size"),
            lr=_coerce_optional_str(value.get("lr")),
            num_epochs=_coerce_required_int(value.get("num_epochs"), "args.Training.num_epochs"),
            speaker_name=_coerce_required_str(value.get("speaker_name"), "args.Training.speaker_name"),
            gradient_accumulation_steps=_coerce_required_int(
                value.get("gradient_accumulation_steps"),
                "args.Training.gradient_accumulation_steps",
            ),
            enable_gradient_checkpointing=_coerce_optional_bool(
                value.get("enable_gradient_checkpointing")
            ),
        )


@dataclass(frozen=True)
class TextToSpeechArgs:
    common: CommonTaskArgs
    text: str
    language: str | None
    speaker: str | None
    output_path: str

    @classmethod
    def from_mapping(cls, value: dict[str, object]) -> "TextToSpeechArgs":
        return cls(
            common=CommonTaskArgs.from_mapping(value),
            text=_coerce_required_str(value.get("text"), "args.TextToSpeech.text"),
            language=_coerce_optional_str(value.get("language")),
            speaker=_coerce_optional_str(value.get("speaker")),
            output_path=_coerce_required_str(value.get("output_path"), "args.TextToSpeech.output_path"),
        )


@dataclass(frozen=True)
class VoiceCloneArgs:
    common: CommonTaskArgs
    ref_audio_path: str
    ref_text: str | None
    language: str | None
    output_path: str
    text: str

    @classmethod
    def from_mapping(cls, value: dict[str, object]) -> "VoiceCloneArgs":
        return cls(
            common=CommonTaskArgs.from_mapping(value),
            ref_audio_path=_coerce_required_str(
                value.get("ref_audio_path"),
                "args.VoiceClone.ref_audio_path",
            ),
            ref_text=_coerce_optional_str(value.get("ref_text")),
            language=_coerce_optional_str(value.get("language")),
            output_path=_coerce_required_str(value.get("output_path"), "args.VoiceClone.output_path"),
            text=_coerce_required_str(value.get("text"), "args.VoiceClone.text"),
        )


@dataclass(frozen=True)
class ParamsEntity:
    version: str
    base_model: str
    model_version: str
    kind: TaskKind
    runtime: RuntimeOptions
    args: TrainingArgs | TextToSpeechArgs | VoiceCloneArgs

    @classmethod
    def from_dict(cls, payload: dict[str, object]) -> "ParamsEntity":
        kind = TaskKind.from_value(payload.get("kind"))
        raw_args = _expect_mapping(payload.get("args"), "args")
        nested_args = _expect_mapping(raw_args.get(kind.value), f"args.{kind.value}")

        if kind is TaskKind.TRAINING:
            parsed_args: TrainingArgs | TextToSpeechArgs | VoiceCloneArgs = TrainingArgs.from_mapping(
                nested_args
            )
        elif kind is TaskKind.TEXT_TO_SPEECH:
            parsed_args = TextToSpeechArgs.from_mapping(nested_args)
        else:
            parsed_args = VoiceCloneArgs.from_mapping(nested_args)

        return cls(
            version=str(payload.get("version") or "1.0.0"),
            base_model=str(payload.get("base_model") or ""),
            model_version=str(payload.get("model_version") or ""),
            kind=kind,
            runtime=RuntimeOptions.from_mapping(payload.get("runtime")),
            args=parsed_args,
        )

    @classmethod
    def from_json(cls, args_json: str) -> "ParamsEntity":
        payload = json.loads(args_json)
        if not isinstance(payload, dict):
            raise TypeError("Malformed params payload: root must be an object")
        return cls.from_dict(payload)

    @classmethod
    def from_file(cls, path: str | Path) -> "ParamsEntity":
        params_path = Path(path).expanduser().resolve()
        if not params_path.exists():
            raise FileNotFoundError(f"Params file not found: {params_path}")

        with params_path.open("r", encoding="utf-8") as file:
            return cls.from_json(file.read())

    def from_args_json(self, args_json: str) -> "ParamsEntity":
        parsed = self.from_json(args_json)
        object.__setattr__(self, "version", parsed.version)
        object.__setattr__(self, "base_model", parsed.base_model)
        object.__setattr__(self, "model_version", parsed.model_version)
        object.__setattr__(self, "kind", parsed.kind)
        object.__setattr__(self, "runtime", parsed.runtime)
        object.__setattr__(self, "args", parsed.args)
        return self

    @property
    def common(self) -> CommonTaskArgs:
        return self.args.common

    def ensure_kind(self, expected: TaskKind) -> "ParamsEntity":
        if self.kind is not expected:
            raise ValueError(
                f"Expected {expected.value} params payload, got: {self.kind.value}"
            )
        return self

    def training_args(self) -> TrainingArgs:
        self.ensure_kind(TaskKind.TRAINING)
        if not isinstance(self.args, TrainingArgs):
            raise TypeError("Malformed params payload: training args were not parsed")
        return self.args

    def tts_args(self) -> TextToSpeechArgs:
        self.ensure_kind(TaskKind.TEXT_TO_SPEECH)
        if not isinstance(self.args, TextToSpeechArgs):
            raise TypeError("Malformed params payload: text-to-speech args were not parsed")
        return self.args

    def voice_clone_args(self) -> VoiceCloneArgs:
        self.ensure_kind(TaskKind.VOICE_CLONE)
        if not isinstance(self.args, VoiceCloneArgs):
            raise TypeError("Malformed params payload: voice-clone args were not parsed")
        return self.args

    def model_param(self, key: str, default: object = None) -> object:
        return self.common.model_params_json.get(key, default)

    def model_param_str(self, key: str, default: str | None = None) -> str | None:
        value = self.model_param(key, default)
        if value is None:
            return None
        return str(value)

    def model_param_int(self, key: str, default: int | None = None) -> int | None:
        value = self.model_param(key, default)
        if value is None:
            return None
        return int(value)

    def model_param_bool(self, key: str, default: bool = False) -> bool:
        value = self.model_param(key, default)
        return _coerce_optional_bool(value, default)