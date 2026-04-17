import argparse
from dataclasses import dataclass
from pathlib import Path
from types import SimpleNamespace


@dataclass
class VoiceCloneRuntimeOptions:
    is_cpu: bool
    model_load_kwargs: dict[str, object]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--ref_audio_path", "--ref-audio-path", dest="ref_audio_path", type=str, required=True, help="Reference audio local path for voice cloning.")
    parser.add_argument("--ref_text", "--ref-text", dest="ref_text", type=str, required=True, help="Reference text corresponding to the reference audio.")
    parser.add_argument("--init_model_path", "--init-model-path", dest="init_model_path", type=str, default="Qwen/Qwen3-TTS-12Hz-1.7B-Base")
    parser.add_argument("--language", type=str, default="Auto")
    parser.add_argument("--output_path", "--output-path", dest="output_path", type=str, required=True)
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--device", type=str, default="cuda:0")
    parser.add_argument("--text", type=str, required=True, help="Input text to be synthesized.")
    parser.add_argument(
        "--attn_implementation",
        "--attn-implementation",
        dest="attn_implementation",
        type=str,
        default="flash_attention_2",
    )
    return parser.parse_args(argv)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def load_dependencies() -> SimpleNamespace:
    import torch
    import soundfile as sf
    from qwen_tts import Qwen3TTSModel

    return SimpleNamespace(torch=torch, sf=sf, Qwen3TTSModel=Qwen3TTSModel)


def build_runtime_options(args: argparse.Namespace, torch_module) -> VoiceCloneRuntimeOptions:
    if is_cpu_device(args.device):
        return VoiceCloneRuntimeOptions(
            is_cpu=True,
            model_load_kwargs={
                "device_map": args.device,
                "dtype": torch_module.float32,
            },
        )

    return VoiceCloneRuntimeOptions(
        is_cpu=False,
        model_load_kwargs={
            "device_map": args.device,
            "dtype": torch_module.bfloat16,
            "attn_implementation": args.attn_implementation,
        },
    )


def load_model(args: argparse.Namespace, dependencies: SimpleNamespace | None = None):
    deps = dependencies or load_dependencies()
    runtime = build_runtime_options(args, deps.torch)
    model = deps.Qwen3TTSModel.from_pretrained(
        args.init_model_path,
        **runtime.model_load_kwargs,
    )
    return model, runtime, deps


def generate_voice_clone_audio(
    args: argparse.Namespace,
    dependencies: SimpleNamespace | None = None,
):
    ref_audio_path = Path(args.ref_audio_path).expanduser().resolve()
    if not ref_audio_path.exists():
        raise FileNotFoundError(f"Reference audio file not found: {ref_audio_path}")
    if not args.ref_text.strip():
        raise ValueError("Reference text cannot be empty.")
    if not args.text.strip():
        raise ValueError("Target text cannot be empty.")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    model, runtime, deps = load_model(args, dependencies)
    wavs, sr = model.generate_voice_clone(
        text=args.text,
        language=args.language,
        ref_audio=str(ref_audio_path),
        ref_text=args.ref_text,
    )
    deps.sf.write(str(output_path), wavs[0], sr)
    return runtime


def main(argv: list[str] | None = None):
    args = parse_args(argv)
    generate_voice_clone_audio(args)


if __name__ == "__main__":
    main()