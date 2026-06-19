"""说话人文本转语音入口：转译 --params-file 后调用上游 infer.py。

说话人来源：训练产物（Speaker Inversion 产出 .speaker.safetensors → --ref-embed，
或 LoRA 产出适配器目录 → --lora-adapter --no-ref）。无训练产物时退化为 --no-ref 纯文本生成。
"""

from __future__ import annotations

import argparse

from common import run_infer
from params import load_tts_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Irodori-TTS text-to-speech inference.")
    parser.add_argument(
        "--params-file",
        dest="params_file",
        type=str,
        required=True,
        help="Path to a JSON params file produced by the kirine-client UI.",
    )
    return parser.parse_args(argv)


def build_infer_args(params) -> list[str]:
    sampling = params.sampling
    args = [
        "--checkpoint", str(params.base_checkpoint),
        "--text", params.text,
        "--output-wav", params.output_path,
        "--model-device", params.device,
        "--codec-device", params.device,
        "--model-precision", sampling.model_precision,
        "--num-steps", str(sampling.num_steps),
        "--cfg-scale-text", str(sampling.cfg_scale_text),
        "--cfg-scale-speaker", str(sampling.cfg_scale_speaker),
        "--duration-scale", str(sampling.duration_scale),
        "--num-candidates", str(sampling.num_candidates),
    ]

    if params.speaker_kind == "ref_embed" and params.speaker_path is not None:
        args += ["--ref-embed", str(params.speaker_path)]
    elif params.speaker_kind == "lora_adapter" and params.speaker_path is not None:
        args += ["--lora-adapter", str(params.speaker_path), "--no-ref"]
    else:
        # 无训练产物：基座 500M-v3 为 speaker-conditioned，需 --no-ref 退化为纯文本生成。
        args += ["--no-ref"]

    if sampling.seed is not None:
        args += ["--seed", str(sampling.seed)]
    return args


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_tts_params(cli_args.params_file)
    print(
        f"[irodori_tts] TTS text_len={len(params.text)} "
        f"speaker_kind={params.speaker_kind} device={params.device}",
        flush=True,
    )
    run_infer(build_infer_args(params))


if __name__ == "__main__":
    main()
