import argparse
from dataclasses import dataclass
from pathlib import Path
import sys
from types import SimpleNamespace


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()

from qwen3_tts.params import load_voice_design_params


@dataclass
class VoiceDesignRuntimeOptions:
    is_cpu: bool
    model_load_kwargs: dict[str, object]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def is_cpu_device(device: str) -> bool:
    return device.strip().lower().startswith("cpu")


def load_dependencies() -> SimpleNamespace:
    import torch
    import soundfile as sf
    from qwen_tts import Qwen3TTSModel

    return SimpleNamespace(torch=torch, sf=sf, Qwen3TTSModel=Qwen3TTSModel)


def build_runtime_options(args: argparse.Namespace, torch_module) -> VoiceDesignRuntimeOptions:
    if is_cpu_device(args.device):
        return VoiceDesignRuntimeOptions(
            is_cpu=True,
            model_load_kwargs={
                "device_map": args.device,
                "dtype": torch_module.float32,
            },
        )

    return VoiceDesignRuntimeOptions(
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


def generate_voice_design_audio(args: argparse.Namespace, dependencies: SimpleNamespace | None = None):
    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(
        f"[voice_design] language={args.language} device={args.device}",
        flush=True,
    )

    model, runtime, deps = load_model(args, dependencies)
    wavs, sr = model.generate_voice_design(
        text=args.text,
        language=args.language,
        instruct=args.instruct,
    )
    deps.sf.write(str(output_path), wavs[0], sr)
    return runtime


def main(argv: list[str] | None = None):
    cli_args = parse_args(argv)
    params = load_voice_design_params(cli_args.params_file)
    generate_voice_design_audio(params.to_namespace())


if __name__ == "__main__":
    main()
