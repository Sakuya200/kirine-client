from __future__ import annotations

import argparse
import importlib.util
import subprocess
import sys
import warnings
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace
from typing import Sequence

def ensure_src_root_on_path() -> Path:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)
    return src_root


ensure_src_root_on_path()


def install_torchaudio_load_fallback() -> None:
    import soundfile as sf
    import torch
    import torchaudio

    if getattr(torchaudio.load, "_kirine_moss_fallback", False):
        return

    original_load = torchaudio.load

    def load_with_soundfile_fallback(
        uri,
        frame_offset: int = 0,
        num_frames: int = -1,
        normalize: bool = True,
        channels_first: bool = True,
        format=None,
        buffer_size: int = 4096,
        backend=None,
    ):
        try:
            return original_load(
                uri,
                frame_offset=frame_offset,
                num_frames=num_frames,
                normalize=normalize,
                channels_first=channels_first,
                format=format,
                buffer_size=buffer_size,
                backend=backend,
            )
        except RuntimeError as exc:
            if "Could not load libtorchcodec" not in str(exc):
                raise

            warnings.warn(
                "torchaudio.load fell back to soundfile because torchcodec could not be loaded.",
                RuntimeWarning,
                stacklevel=2,
            )

            read_kwargs = {"dtype": "float32", "always_2d": True}
            if frame_offset:
                read_kwargs["start"] = int(frame_offset)
            if num_frames is not None and int(num_frames) >= 0:
                read_kwargs["frames"] = int(num_frames)

            waveform, sample_rate = sf.read(str(uri), **read_kwargs)
            tensor = torch.from_numpy(waveform.T.copy())
            if not channels_first:
                tensor = tensor.transpose(0, 1)
            return tensor, sample_rate

    load_with_soundfile_fallback._kirine_moss_fallback = True  # type: ignore[attr-defined]
    torchaudio.load = load_with_soundfile_fallback


def install_torchaudio_save_fallback() -> None:
    import soundfile as sf
    import torchaudio

    if getattr(torchaudio.save, "_kirine_moss_fallback", False):
        return

    original_save = torchaudio.save

    def save_with_soundfile_fallback(
        uri,
        src,
        sample_rate: int,
        channels_first: bool = True,
        format=None,
        encoding=None,
        bits_per_sample=None,
        buffer_size: int = 4096,
        backend=None,
        compression=None,
    ):
        try:
            return original_save(
                uri,
                src,
                int(sample_rate),
                channels_first=channels_first,
                format=format,
                encoding=encoding,
                bits_per_sample=bits_per_sample,
                buffer_size=buffer_size,
                backend=backend,
                compression=compression,
            )
        except RuntimeError as exc:
            if "Could not load libtorchcodec" not in str(exc):
                raise

            warnings.warn(
                "torchaudio.save fell back to soundfile because torchcodec could not be loaded.",
                RuntimeWarning,
                stacklevel=2,
            )

            waveform = src.detach().cpu()
            if waveform.ndim == 1:
                waveform = waveform.unsqueeze(0)
            if channels_first:
                waveform = waveform.transpose(0, 1)

            sf.write(str(uri), waveform.numpy(), int(sample_rate), format=format)
            return None

    save_with_soundfile_fallback._kirine_moss_fallback = True  # type: ignore[attr-defined]
    torchaudio.save = save_with_soundfile_fallback


@dataclass(frozen=True)
class RuntimeOptions:
    device: str
    torch_dtype_name: str
    attn_implementation: str

    @property
    def is_cpu(self) -> bool:
        return self.device == "cpu"


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def configure_torch_backends(torch_module) -> None:
    if not hasattr(torch_module, "backends") or not hasattr(torch_module.backends, "cuda"):
        return

    torch_module.backends.cuda.enable_cudnn_sdp(False)
    torch_module.backends.cuda.enable_flash_sdp(True)
    torch_module.backends.cuda.enable_mem_efficient_sdp(True)
    torch_module.backends.cuda.enable_math_sdp(True)


def resolve_runtime_options(device: str, requested_attn_implementation: str, torch_module) -> RuntimeOptions:
    normalized_device = "cpu" if is_cpu_device(device) or not torch_module.cuda.is_available() else "cuda"
    dtype_name = "float32" if normalized_device == "cpu" else "bfloat16"
    requested = requested_attn_implementation.strip().lower()

    if requested in {"", "auto"}:
        if (
            normalized_device == "cuda"
            and importlib.util.find_spec("flash_attn") is not None
            and dtype_name in {"float16", "bfloat16"}
        ):
            major, _ = torch_module.cuda.get_device_capability()
            if major >= 8:
                requested = "flash_attention_2"
        if requested in {"", "auto"}:
            requested = "sdpa" if normalized_device == "cuda" else "eager"

    resolved_device = device if normalized_device == "cpu" else device.strip() or "cuda:0"
    return RuntimeOptions(
        device=resolved_device,
        torch_dtype_name=dtype_name,
        attn_implementation=requested,
    )


def load_backend(model_path: str, device: str, requested_attn_implementation: str) -> tuple[object, object, SimpleNamespace, RuntimeOptions]:
    import torch
    import torchaudio
    from transformers import AutoModel, AutoProcessor

    install_torchaudio_load_fallback()
    configure_torch_backends(torch)
    runtime = resolve_runtime_options(device, requested_attn_implementation, torch)
    torch_dtype = getattr(torch, runtime.torch_dtype_name)

    processor = AutoProcessor.from_pretrained(model_path, trust_remote_code=True)
    if hasattr(processor, "audio_tokenizer"):
        processor.audio_tokenizer = processor.audio_tokenizer.to(runtime.device)

    model_kwargs: dict[str, object] = {
        "trust_remote_code": True,
        "torch_dtype": torch_dtype,
    }
    if runtime.attn_implementation:
        model_kwargs["attn_implementation"] = runtime.attn_implementation

    model = AutoModel.from_pretrained(model_path, **model_kwargs).to(runtime.device)
    model.eval()

    deps = SimpleNamespace(torch=torch, torchaudio=torchaudio)
    return model, processor, deps, runtime


def save_generated_audio(output_path: str, audio, sample_rate: int) -> None:
    import torchaudio

    install_torchaudio_save_fallback()
    resolved_output = Path(output_path).expanduser().resolve()
    resolved_output.parent.mkdir(parents=True, exist_ok=True)

    if getattr(audio, "ndim", 0) == 1:
        audio = audio.unsqueeze(0)

    torchaudio.save(str(resolved_output), audio.detach().cpu(), int(sample_rate))


def run_subprocess(command: Sequence[str], cwd: Path | None = None) -> None:
    subprocess.run(list(command), cwd=str(cwd) if cwd else None, check=True)


def add_shared_generation_args(parser: argparse.ArgumentParser) -> None:
    parser.add_argument("--init_model_path", "--init-model-path", dest="init_model_path", type=str, required=True)
    parser.add_argument("--output_path", "--output-path", dest="output_path", type=str, required=True)
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="auto",
    )
    parser.add_argument("--max_new_tokens", "--max-new-tokens", dest="max_new_tokens", type=int, default=4096)
    parser.add_argument(
        "--n_vq_for_inference",
        "--n-vq-for-inference",
        dest="n_vq_for_inference",
        type=int,
        default=32,
    )
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--language", type=str, default="Auto")


def current_python_executable() -> str:
    return sys.executable