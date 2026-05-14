from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
from pathlib import Path

from vox_cpm2.params_entity import CommonTaskArgs, ParamsEntity, RuntimeOptions


VOX_CPM2_MODEL_NAME = "VoxCPM2"


@dataclass
class VoxCpm2TrainingParams:
    base_model: str
    version: str
    common: CommonTaskArgs
    init_model_path: str
    input_jsonl: str
    output_model_path: str
    batch_size: int
    lr: float
    num_epochs: int
    gradient_accumulation_steps: int
    use_lora: bool
    training_mode: str
    lora_rank: int | None
    lora_alpha: int | None
    lora_dropout: str | None
    weight_decay: float | None
    warmup_steps: int | None
    runtime: RuntimeOptions

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
            train_script_path="",
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
        raise ValueError(f"VoxCPM2 params payload is missing a resolvable {label}")
    return path


def _resolve_nested_training_checkpoint_path(model_root_path: Path) -> Path | None:
    checkpoints_root = model_root_path / "checkpoints"
    if not checkpoints_root.is_dir():
        return None

    latest_dirs: list[Path] = []
    step_dirs: list[Path] = []

    for mode_entry in checkpoints_root.iterdir():
        if not mode_entry.is_dir():
            continue

        latest_path = (mode_entry / "latest").resolve()
        if latest_path.is_dir():
            latest_dirs.append(latest_path)

        for checkpoint_entry in mode_entry.iterdir():
            checkpoint_path = checkpoint_entry.resolve()
            if not checkpoint_entry.is_dir() or checkpoint_path == latest_path:
                continue
            step_dirs.append(checkpoint_path)

    latest_dirs.sort()
    if latest_dirs:
        return latest_dirs[-1]

    step_dirs.sort()
    if step_dirs:
        return step_dirs[-1]

    return None


def _resolve_vox_inference_checkpoint_path(model_root_path: Path) -> Path:
    nested_checkpoint_path = _resolve_nested_training_checkpoint_path(model_root_path)
    if nested_checkpoint_path is not None:
        return nested_checkpoint_path

    if not model_root_path.is_dir():
        return model_root_path.resolve()

    checkpoint_dirs: list[Path] = []
    for entry in model_root_path.iterdir():
        if not entry.is_dir():
            continue
        if entry.name.startswith("checkpoint") or entry.name == "latest":
            checkpoint_dirs.append(entry.resolve())

    checkpoint_dirs.sort()
    if checkpoint_dirs:
        return checkpoint_dirs[-1]

    return model_root_path.resolve()


def _resolve_model_path(common: CommonTaskArgs) -> str:
    candidate = _resolve_locator_candidate(
        common,
        VOX_CPM2_MODEL_NAME,
        prefer_speaker_dir_name=True,
    )
    inference_root = Path(_require_resolved_path(candidate, "inference model path"))
    return str(_resolve_vox_inference_checkpoint_path(inference_root))


def _resolve_training_model_path(common: CommonTaskArgs) -> str:
    candidate = _resolve_locator_candidate(
        common,
        VOX_CPM2_MODEL_NAME,
        prefer_speaker_dir_name=False,
    )
    return _require_resolved_path(candidate, "training model path")


def _parse_optional_float(value: str | None) -> float | None:
    if value is None:
        return None
    return float(value)


def _parse_float_with_default(value: str | None, default: float) -> float:
    return default if value is None else float(value)
def _normalize_runtime(runtime: RuntimeOptions) -> RuntimeOptions:
    return RuntimeOptions(
        device=runtime.device or "cuda:0",
        logging_dir=runtime.logging_dir or "",
        attn_implementation=runtime.attn_implementation or "auto",
    )


def load_training_params(path: str | Path) -> VoxCpm2TrainingParams:
    params = ParamsEntity.from_file(path)
    args = params.training_args()
    use_lora = params.model_param_bool("useLora", False)
    training_mode = params.model_param_str(
        "trainingMode",
        "lora" if use_lora else "full",
    )

    return VoxCpm2TrainingParams(
        base_model=params.base_model or "vox_cpm2",
        version=params.version,
        common=args.common,
        init_model_path=_resolve_training_model_path(args.common),
        input_jsonl=args.input_jsonl,
        output_model_path=args.output_model_path,
        batch_size=args.batch_size,
        lr=_parse_float_with_default(params.model_param_str("learningRate", args.lr), 1e-4),
        num_epochs=args.num_epochs,
        gradient_accumulation_steps=args.gradient_accumulation_steps,
        use_lora=use_lora,
        training_mode=str(training_mode),
        lora_rank=params.model_param_int("loraRank", None),
        lora_alpha=params.model_param_int("loraAlpha", None),
        lora_dropout=params.model_param_str("loraDropout", None),
        weight_decay=_parse_optional_float(params.model_param_str("weightDecay", None)),
        warmup_steps=params.model_param_int("warmupSteps", None),
        runtime=_normalize_runtime(params.runtime),
    )


@dataclass
class VoxCpm2TtsParams:
    common: CommonTaskArgs
    init_model_path: str
    text: str
    output_path: str
    cfg_value: float
    inference_timesteps: int
    runtime: RuntimeOptions

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
    common: CommonTaskArgs
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
    runtime: RuntimeOptions

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
    params = ParamsEntity.from_file(path)
    args = params.tts_args()

    return VoxCpm2TtsParams(
        common=args.common,
        init_model_path=_resolve_model_path(args.common),
        text=args.text,
        output_path=args.output_path,
        cfg_value=_parse_float_with_default(params.model_param_str("cfgValue", None), 2.0),
        inference_timesteps=int(params.model_param_int("inferenceTimesteps", None) or 10),
        runtime=_normalize_runtime(params.runtime),
    )


def load_voice_clone_params(path: str | Path) -> VoxCpm2VoiceCloneParams:
    params = ParamsEntity.from_file(path)
    args = params.voice_clone_args()

    return VoxCpm2VoiceCloneParams(
        common=args.common,
        init_model_path=_resolve_model_path(args.common),
        mode=str(params.model_param_str("mode", "reference") or "reference"),
        ref_audio_path=args.ref_audio_path,
        ref_text=args.ref_text or "",
        text=args.text,
        style_prompt=str(params.model_param_str("stylePrompt", "") or ""),
        cfg_value=_parse_float_with_default(params.model_param_str("cfgValue", None), 2.0),
        inference_timesteps=int(params.model_param_int("inferenceTimesteps", None) or 10),
        language=args.language or "auto",
        output_path=args.output_path,
        runtime=_normalize_runtime(params.runtime),
    )