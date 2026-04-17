from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace

RUNTIME_METADATA_FILE_NAME = "voxcpm_runtime.json"


@dataclass
class RuntimeTarget:
    model_path: str
    load_kwargs: dict[str, object]


def load_dependencies() -> SimpleNamespace:
    import soundfile as sf
    from voxcpm import VoxCPM

    return SimpleNamespace(sf=sf, VoxCPM=VoxCPM)


def compose_generation_text(text: str, style_prompt: str) -> str:
    clean_text = text.strip()
    clean_style = style_prompt.strip()
    if not clean_style:
        return clean_text
    return f"({clean_style}){clean_text}"


def resolve_runtime_target(init_model_path: str) -> RuntimeTarget:
    model_root = Path(init_model_path).expanduser().resolve()
    metadata_path = model_root / RUNTIME_METADATA_FILE_NAME
    if not metadata_path.exists():
        return RuntimeTarget(model_path=str(model_root), load_kwargs={})

    with metadata_path.open("r", encoding="utf-8") as file:
        metadata = json.load(file)

    training_mode = str(metadata.get("trainingMode", "")).strip().lower()
    latest_checkpoint_path = str(metadata.get("latestCheckpointPath", "")).strip()
    base_model_path = str(metadata.get("baseModelPath", "")).strip()

    if training_mode == "lora":
        if not latest_checkpoint_path or not base_model_path:
            raise ValueError(f"Invalid VoxCPM runtime metadata: {metadata_path}")
        return RuntimeTarget(
            model_path=base_model_path,
            load_kwargs={"lora_weights_path": latest_checkpoint_path},
        )

    if training_mode == "full":
        if not latest_checkpoint_path:
            raise ValueError(f"Invalid VoxCPM runtime metadata: {metadata_path}")
        return RuntimeTarget(model_path=latest_checkpoint_path, load_kwargs={})

    raise ValueError(f"Unsupported VoxCPM runtime training mode: {training_mode}")


def load_model_and_dependencies(init_model_path: str, device: str):
    deps = load_dependencies()
    runtime_target = resolve_runtime_target(init_model_path)
    model = deps.VoxCPM.from_pretrained(
        runtime_target.model_path,
        device=device,
        load_denoiser=False,
        **runtime_target.load_kwargs,
    )
    return model, deps
