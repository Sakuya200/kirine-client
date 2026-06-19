"""模型微调入口：manifest 预处理 + 调用上游 train.py。

支持两种微调模式（由 model_params_json.trainingMode 选择）：
- speaker_inversion：冻结模型、训练 16 个说话人嵌入 token，产出
  ``<output_model_path>/checkpoint_final.speaker.safetensors``，可用于说话人 TTS。
- lora：低秩适配模型权重，产出 ``<output_model_path>/checkpoint_final/`` 适配器目录。

manifest 预处理：上游 ``prepare_manifest.py`` 仅接受 HF ``--dataset``，不接受后端提供的
原始 JSONL（``{audio, text, ...}``）。故此处内联等价的 DACVAE 隐变量预计算逻辑，
读取后端 JSONL，编码每条音频为 ``.pt`` 隐变量，写出 Irodori manifest
``{text, latent_path, num_frames, speaker_id}``。
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

from common import repo_root, base_checkpoint, run_train
from params import load_training_params

CODEC_REPO_ID = "Aratako/Semantic-DACVAE-Japanese-32dim"
MANIFEST_FILENAME = "_manifest.jsonl"
LATENTS_DIRNAME = "_latents"

CONFIG_BY_MODE = {
    "speaker_inversion": "configs/train_500m_v3_speaker_inversion.yaml",
    "lora": "configs/train_500m_v3_lora.yaml",
}


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run Irodori-TTS fine-tuning via kirine-client.")
    parser.add_argument(
        "--params-file",
        dest="params_file",
        type=str,
        required=True,
        help="Path to a JSON params file produced by the kirine-client UI.",
    )
    return parser.parse_args(argv)


def _load_audio_mono(path: Path) -> tuple["torch.Tensor", int]:
    import soundfile as sf
    import torch

    data, sr = sf.read(str(path), dtype="float32", always_2d=False)
    if data.ndim == 2:
        # 多声道 → 单声道（取均值）
        data = data.mean(axis=1)
    wav = torch.from_numpy(data).float().unsqueeze(0)  # (1, T)
    if wav.numel() == 0:
        raise ValueError(f"Decoded audio is empty: {path}")
    return wav, int(sr)


def build_manifest(params) -> Path:
    """读取后端 input_jsonl，预计算 DACVAE 隐变量，写出 Irodori manifest。

    返回 manifest 文件路径。隐变量写入 ``<output_model_path>/_latents/``，
    manifest 写入 ``<output_model_path>/_manifest.jsonl``。
    """
    # 上游 irodori_tts 包位于克隆仓库根目录，需加入 sys.path 才能导入 codec。
    repo = repo_root()
    if str(repo) not in sys.path:
        sys.path.append(str(repo))

    import torch  # noqa: E402
    from irodori_tts.codec import DACVAECodec  # noqa: E402

    output_dir = Path(params.output_model_path).expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    latent_dir = output_dir / LATENTS_DIRNAME
    latent_dir.mkdir(parents=True, exist_ok=True)
    manifest_path = output_dir / MANIFEST_FILENAME

    print(f"[irodori_tts] loading DACVAE codec ({CODEC_REPO_ID}) on {params.device}", flush=True)
    codec = DACVAECodec.load(
        repo_id=CODEC_REPO_ID,
        device=params.device,
        deterministic_encode=True,
        deterministic_decode=True,
        normalize_db=-16.0,
    )

    input_jsonl = Path(params.input_jsonl).expanduser().resolve()
    if not input_jsonl.exists():
        raise FileNotFoundError(f"Training input jsonl not found: {input_jsonl}")

    speaker_id = params.speaker_name or "speaker"
    written = 0
    skipped = 0
    with manifest_path.open("w", encoding="utf-8") as out_f, \
            input_jsonl.open("r", encoding="utf-8") as in_f:
        for line_no, raw in enumerate(in_f):
            raw = raw.strip()
            if not raw:
                continue
            try:
                entry = json.loads(raw)
            except json.JSONDecodeError as exc:
                print(f"[irodori_tts] skip line {line_no}: invalid json ({exc})", flush=True)
                skipped += 1
                continue

            text = str(entry.get("text", "")).strip()
            audio_field = entry.get("audio") or entry.get("ref_audio")
            if not text or not audio_field:
                print(f"[irodori_tts] skip line {line_no}: missing text/audio", flush=True)
                skipped += 1
                continue

            audio_path = Path(str(audio_field)).expanduser().resolve()
            if not audio_path.exists():
                print(f"[irodori_tts] skip line {line_no}: audio not found {audio_path}", flush=True)
                skipped += 1
                continue

            try:
                wav, sr = _load_audio_mono(audio_path)
                with torch.inference_mode():
                    latent = codec.encode_waveform(wav, sample_rate=sr)[0].cpu()
            except Exception as exc:  # noqa: BLE001
                print(f"[irodori_tts] skip line {line_no}: encode failed ({exc})", flush=True)
                skipped += 1
                continue

            latent_name = f"{written:08d}.pt"
            latent_path = latent_dir / latent_name
            torch.save(latent, latent_path)
            payload = {
                "text": text,
                "latent_path": str(latent_path.relative_to(output_dir)),
                "num_frames": int(latent.shape[0]),
                "speaker_id": speaker_id,
            }
            out_f.write(json.dumps(payload, ensure_ascii=False) + "\n")
            written += 1
            if written % 50 == 0:
                print(f"[irodori_tts] manifest: written={written} skipped={skipped}", flush=True)

    if written == 0:
        raise SystemExit(
            "❌ 训练数据预处理后无可用样本，请检查训练音频与文本清单。"
        )

    print(
        f"[irodori_tts] manifest ready: {manifest_path} "
        f"(written={written}, skipped={skipped})",
        flush=True,
    )
    return manifest_path


def compute_max_steps(params, num_samples: int) -> int:
    if params.max_steps is not None and params.max_steps > 0:
        return int(params.max_steps)
    denom = max(1, params.batch_size * params.gradient_accumulation_steps)
    return max(1, math.ceil(params.num_epochs * num_samples / denom))


def count_manifest_samples(manifest_path: Path) -> int:
    count = 0
    with manifest_path.open("r", encoding="utf-8") as f:
        for line in f:
            if line.strip():
                count += 1
    return count


def build_train_args(params, manifest_path: Path, max_steps: int) -> list[str]:
    config_rel = CONFIG_BY_MODE.get(params.training_mode)
    if config_rel is None:
        raise ValueError(f"Unsupported training mode: {params.training_mode}")

    is_cpu = params.device == "cpu"
    precision = "fp32" if is_cpu else "bf16"

    args = [
        "--config", config_rel,
        "--manifest", str(manifest_path),
        "--output-dir", str(Path(params.output_model_path).expanduser().resolve()),
        "--init-checkpoint", str(params.base_checkpoint),
        "--batch-size", str(params.batch_size),
        "--gradient-accumulation-steps", str(params.gradient_accumulation_steps),
        "--max-steps", str(max_steps),
        "--device", params.device,
        "--precision", precision,
        "--no-wandb",
    ]

    if params.enable_gradient_checkpointing:
        args.append("--gradient-checkpointing")
    else:
        args.append("--no-gradient-checkpointing")

    if params.lr:
        args += ["--lr", params.lr]

    if params.training_mode == "lora":
        args += ["--lora-r", str(params.lora_r), "--lora-alpha", str(params.lora_alpha)]

    # seed 由 yaml 默认 0；此处不覆盖，保持可复现。
    return args


def run_training(args: argparse.Namespace) -> None:
    params = load_training_params(args.params_file)
    print(
        f"[irodori_tts] training mode={params.training_mode} "
        f"device={params.device} batch_size={params.batch_size} "
        f"grad_accum={params.gradient_accumulation_steps} epochs={params.num_epochs} "
        f"lr={params.lr}",
        flush=True,
    )
    print(f"[irodori_tts] output_model_path={params.output_model_path}", flush=True)

    manifest_path = build_manifest(params)
    num_samples = count_manifest_samples(manifest_path)
    max_steps = compute_max_steps(params, num_samples)
    print(f"[irodori_tts] samples={num_samples} max_steps={max_steps}", flush=True)

    train_args = build_train_args(params, manifest_path, max_steps)
    run_train(train_args)


def main(argv: list[str] | None = None) -> None:
    cli_args = parse_args(argv)
    run_training(cli_args)


if __name__ == "__main__":
    main()
