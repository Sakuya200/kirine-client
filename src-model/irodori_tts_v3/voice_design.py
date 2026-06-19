"""音色设计入口：转译 --params-file 后调用上游 infer.py。

使用音色设计专用权重 Irodori-TTS-600M-v3-VoiceDesign + caption（来自后端 instruct 字段）。
若模型参数 refAudioPath 非空，启用 text+ref+caption 三分支模式（--ref-wav）；
否则纯 caption 模式（--no-ref）。
"""

from __future__ import annotations

import argparse

from common import run_infer
from params import load_voice_design_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Irodori-TTS voice design inference.")
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
        "--checkpoint", str(params.voicedesign_checkpoint),
        "--text", params.text,
        "--output-wav", params.output_path,
        "--model-device", params.device,
        "--codec-device", params.device,
        "--model-precision", sampling.model_precision,
        "--num-steps", str(sampling.num_steps),
        "--cfg-scale-text", str(sampling.cfg_scale_text),
        "--cfg-scale-caption", str(sampling.cfg_scale_caption),
        "--cfg-scale-speaker", str(sampling.cfg_scale_speaker),
        "--duration-scale", str(sampling.duration_scale),
        "--num-candidates", str(sampling.num_candidates),
    ]

    if params.caption:
        args += ["--caption", params.caption]

    if params.ref_audio_path:
        args += ["--ref-wav", params.ref_audio_path]
    else:
        args += ["--no-ref"]

    if sampling.seed is not None:
        args += ["--seed", str(sampling.seed)]
    return args


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_voice_design_params(cli_args.params_file)
    print(
        f"[irodori_tts] voice-design text_len={len(params.text)} "
        f"caption_len={len(params.caption)} ref_wav={params.ref_audio_path} "
        f"device={params.device}",
        flush=True,
    )
    run_infer(build_infer_args(params))


if __name__ == "__main__":
    main()
