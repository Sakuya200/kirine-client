"""声音克隆入口：转译 --params-file 后调用上游 infer.py。

使用基座 500M-v3 权重 + 参考音频（--ref-wav）进行音色克隆。
上游 infer.py 无 --ref-text 参数，参考文本不参与克隆。
"""

from __future__ import annotations

import argparse

from common import run_infer
from params import load_voice_clone_params


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Irodori-TTS voice cloning inference.")
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
        "--ref-wav", params.ref_audio_path,
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
    if sampling.seed is not None:
        args += ["--seed", str(sampling.seed)]
    return args


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    params = load_voice_clone_params(cli_args.params_file)
    print(
        f"[irodori_tts] voice-clone text_len={len(params.text)} "
        f"ref_wav={params.ref_audio_path} device={params.device}",
        flush=True,
    )
    run_infer(build_infer_args(params))


if __name__ == "__main__":
    main()
