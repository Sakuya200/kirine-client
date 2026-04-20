import argparse
from dataclasses import dataclass
import json
from pathlib import Path
import re
import sys
from types import SimpleNamespace


def ensure_src_root_on_path() -> None:
    src_root = Path(__file__).resolve().parents[1]
    src_root_str = str(src_root)
    if src_root_str not in sys.path:
        sys.path.insert(0, src_root_str)


ensure_src_root_on_path()


@dataclass
class TtsRuntimeOptions:
    is_cpu: bool
    model_load_kwargs: dict[str, object]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--init_model_path",
        "--init-model-path",
        dest="init_model_path",
        type=str,
        default="Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
    )
    parser.add_argument("--text", type=str)
    parser.add_argument("--language", type=str, default="Auto")
    parser.add_argument("--speaker", type=str, default="")
    parser.add_argument("--instruct", type=str, default="")
    parser.add_argument("--output_path", "--output-path", dest="output_path", type=str)
    parser.add_argument("--logging_dir", "--logging-dir", dest="logging_dir", type=str, default="")
    parser.add_argument("--device", type=str, default="cuda:0")
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


def normalize_speaker_name(speaker_name: str) -> str:
    normalized = re.sub(r"[\s\-]+", "_", speaker_name.strip())
    normalized = re.sub(r"_+", "_", normalized)
    return normalized.casefold()


def load_speaker_mapping(model_path: str) -> dict[str, object]:
    config_path = Path(model_path).expanduser().resolve() / "config.json"
    if not config_path.exists():
        raise FileNotFoundError(f"config.json not found under model path: {config_path}")

    with config_path.open("r", encoding="utf-8") as file:
        config = json.load(file)

    talker_config = config.get("talker_config") or {}
    speaker_ids = talker_config.get("spk_id") or {}
    if not isinstance(speaker_ids, dict) or not speaker_ids:
        raise ValueError(
            f"No custom speaker mapping was found in model config: {config_path}"
        )

    return speaker_ids


def infer_speaker_name_from_model(model_path: str) -> str:
    speaker_ids = load_speaker_mapping(model_path)
    speaker_name = next(iter(speaker_ids.keys()), "").strip()
    if not speaker_name:
        raise ValueError(
            f"Resolved empty speaker name from model config: {Path(model_path).expanduser().resolve() / 'config.json'}"
        )

    return speaker_name


def resolve_explicit_speaker_name(speaker_name: str, model_path: str) -> str:
    explicit_speaker = normalize_speaker_name(speaker_name)
    if not explicit_speaker:
        return ""

    speaker_ids = load_speaker_mapping(model_path)
    speaker_pairs = {
        normalize_speaker_name(candidate).casefold(): candidate
        for candidate in speaker_ids.keys()
        if normalize_speaker_name(candidate)
    }
    resolved_speaker = speaker_pairs.get(explicit_speaker.casefold())
    if resolved_speaker:
        return resolved_speaker

    available_speakers = ", ".join(sorted(speaker_ids.keys()))
    raise ValueError(
        f"Unsupported speaker '{speaker_name}'. Available speakers: {available_speakers}"
    )


def resolve_speaker_name(args: argparse.Namespace) -> str:
    explicit_speaker = resolve_explicit_speaker_name(args.speaker, args.init_model_path)
    if explicit_speaker:
        return explicit_speaker

    return infer_speaker_name_from_model(args.init_model_path)


def build_runtime_options(args: argparse.Namespace, torch_module) -> TtsRuntimeOptions:
    if is_cpu_device(args.device):
        return TtsRuntimeOptions(
            is_cpu=True,
            model_load_kwargs={
                "device_map": args.device,
                "dtype": torch_module.float32,
            },
        )

    return TtsRuntimeOptions(
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


def generate_audio(args: argparse.Namespace, dependencies: SimpleNamespace | None = None):
    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    speaker_name = resolve_speaker_name(args)

    print(
        f"[tts] resolved speaker={speaker_name} language={args.language} device={args.device}",
        flush=True,
    )

    model, runtime, deps = load_model(args, dependencies)
    wavs, sr = model.generate_custom_voice(
        text=args.text,
        language=args.language,
        speaker=speaker_name,
        instruct=args.instruct,
    )
    deps.sf.write(str(output_path), wavs[0], sr)
    return runtime


def main(argv: list[str] | None = None):
    args = parse_args(argv)
    generate_audio(args)


if __name__ == "__main__":
    main()