from __future__ import annotations

import json
import os
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace

RUNTIME_METADATA_FILE_NAME = "voxcpm_runtime.json"
SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]
VOX_CPM2_BASE_MODEL_RELATIVE_PATH = Path("base-models") / "VoxCPM2"


@dataclass
class RuntimeTarget:
    model_path: str
    load_kwargs: dict[str, object]


def load_dependencies() -> SimpleNamespace:
    import soundfile as sf
    from voxcpm import VoxCPM

    return SimpleNamespace(sf=sf, VoxCPM=VoxCPM)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def prepare_runtime_environment(device: str) -> None:
    if is_cpu_device(device):
        os.environ["CUDA_VISIBLE_DEVICES"] = ""


def resolve_optimize_flag(device: str) -> bool:
    return not is_cpu_device(device)


def compose_generation_text(text: str, style_prompt: str) -> str:
    clean_text = text.strip()
    clean_style = style_prompt.strip()
    if not clean_style:
        return clean_text
    return f"({clean_style}){clean_text}"


def normalize_metadata_path(value: str) -> str:
    return value.strip().replace("\\", "/")


def resolve_existing_path(path: Path) -> Path | None:
    candidate = path.expanduser()
    if candidate.exists():
        return candidate.resolve()
    return None


def resolve_absolute_or_relative_path(
    absolute_value: str,
    relative_value: str,
    relative_root: Path,
) -> Path | None:
    normalized_absolute = normalize_metadata_path(absolute_value)
    if normalized_absolute:
        resolved = resolve_existing_path(Path(normalized_absolute))
        if resolved is not None:
            return resolved

    normalized_relative = normalize_metadata_path(relative_value)
    if normalized_relative:
        resolved = resolve_existing_path(relative_root / normalized_relative)
        if resolved is not None:
            return resolved

    return None


def resolve_latest_checkpoint_from_root(model_root: Path, training_mode: str) -> Path | None:
    checkpoint_root = model_root / "checkpoints" / training_mode
    latest_path = checkpoint_root / "latest"
    if latest_path.exists():
        return latest_path.resolve()

    step_dirs = sorted(path for path in checkpoint_root.glob("step_*") if path.is_dir())
    if step_dirs:
        return step_dirs[-1].resolve()

    return None


def resolve_base_model_path(metadata_path: Path, base_model_path: str, base_model_relative_path: str) -> Path:
    resolved = resolve_absolute_or_relative_path(
        base_model_path,
        base_model_relative_path,
        SRC_MODEL_ROOT,
    )
    if resolved is not None:
        return resolved

    fallback = resolve_existing_path(SRC_MODEL_ROOT / VOX_CPM2_BASE_MODEL_RELATIVE_PATH)
    if fallback is not None:
        return fallback

    raise FileNotFoundError(
        "Unable to resolve VoxCPM2 base model directory from runtime metadata: "
        f"{metadata_path}"
    )


def resolve_checkpoint_path(
    model_root: Path,
    metadata_path: Path,
    training_mode: str,
    latest_checkpoint_path: str,
    latest_checkpoint_relative_path: str,
) -> Path:
    resolved = resolve_absolute_or_relative_path(
        latest_checkpoint_path,
        latest_checkpoint_relative_path,
        model_root,
    )
    if resolved is not None:
        return resolved

    fallback = resolve_latest_checkpoint_from_root(model_root, training_mode)
    if fallback is not None:
        return fallback

    raise FileNotFoundError(
        "Unable to resolve VoxCPM2 checkpoint directory from runtime metadata: "
        f"{metadata_path}"
    )


def resolve_runtime_target(init_model_path: str) -> RuntimeTarget:
    model_root = Path(init_model_path).expanduser().resolve()
    metadata_path = model_root / RUNTIME_METADATA_FILE_NAME
    if not metadata_path.exists():
        return RuntimeTarget(model_path=str(model_root), load_kwargs={})

    with metadata_path.open("r", encoding="utf-8") as file:
        metadata = json.load(file)

    training_mode = str(metadata.get("trainingMode", "")).strip().lower()
    latest_checkpoint_path = str(metadata.get("latestCheckpointPath", "")).strip()
    latest_checkpoint_relative_path = str(metadata.get("latestCheckpointRelativePath", "")).strip()
    base_model_path = str(metadata.get("baseModelPath", "")).strip()
    base_model_relative_path = str(metadata.get("baseModelRelativePath", "")).strip()

    if training_mode == "lora":
        if (not latest_checkpoint_path and not latest_checkpoint_relative_path) or (
            not base_model_path and not base_model_relative_path
        ):
            raise ValueError(f"Invalid VoxCPM runtime metadata: {metadata_path}")
        return RuntimeTarget(
            model_path=str(
                resolve_base_model_path(
                    metadata_path,
                    base_model_path,
                    base_model_relative_path,
                )
            ),
            load_kwargs={
                "lora_weights_path": str(
                    resolve_checkpoint_path(
                        model_root,
                        metadata_path,
                        training_mode,
                        latest_checkpoint_path,
                        latest_checkpoint_relative_path,
                    )
                )
            },
        )

    if training_mode == "full":
        if not latest_checkpoint_path and not latest_checkpoint_relative_path:
            raise ValueError(f"Invalid VoxCPM runtime metadata: {metadata_path}")
        return RuntimeTarget(
            model_path=str(
                resolve_checkpoint_path(
                    model_root,
                    metadata_path,
                    training_mode,
                    latest_checkpoint_path,
                    latest_checkpoint_relative_path,
                )
            ),
            load_kwargs={},
        )

    raise ValueError(f"Unsupported VoxCPM runtime training mode: {training_mode}")


def load_model_and_dependencies(init_model_path: str, device: str):
    prepare_runtime_environment(device)
    deps = load_dependencies()
    runtime_target = resolve_runtime_target(init_model_path)
    model = deps.VoxCPM.from_pretrained(
        runtime_target.model_path,
        load_denoiser=False,
        optimize=resolve_optimize_flag(device),
        **runtime_target.load_kwargs,
    )
    return model, deps
