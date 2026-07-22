"""MOSS-TTS-Realtime 模型微调入口：统一 JSONL -> MOSS conversations 映射 + 调上游 finetuning。

流程（run_training，Task 10 追加）：
1. 统一 JSONL ``{fid, audio, text, language?}`` -> MOSS ``conversations`` JSONL（本文件 map_to_conversations）。
2. subprocess ``accelerate launch moss_tts_realtime/finetuning/prepare_data.py`` 预编码 audio_codes。
3. subprocess ``accelerate launch moss_tts_realtime/finetuning/sft.py`` SFT 训练。
4. 规整产物：最大的 ``checkpoint-epoch-{N}`` / ``checkpoint-{N}`` -> ``checkpoint_final/``（本文件 canonicalize_checkpoint）。

上游脚本经 ``common.run_upstream_script`` 以 cwd=仓库根运行，``mossttsrealtime`` 包经 sys.path[0] 可导入。
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from common import base_checkpoint, codec_path, run_upstream_script
from params import load_training_params

CHECKPOINT_FINAL_DIR = "checkpoint_final"


def map_to_conversations(input_jsonl: Path, output_jsonl: Path) -> int:
    """统一 JSONL -> MOSS conversations JSONL（单轮映射）。

    ``{fid, audio, text, language?}`` -> ``{"id": fid, "conversations": [{"role":"assistant", "text": text, "wav": audio}]}``
    可选 ``refWav`` / ``ref_wav`` -> ``ref_wav``（声音克隆式微调）。``language`` 丢弃（MOSS 自动语种检测）。
    缺 text/audio 的行跳过。返回写入条数；为 0 则 raise SystemExit。
    """
    written = 0
    with input_jsonl.open("r", encoding="utf-8") as fin, output_jsonl.open("w", encoding="utf-8") as fout:
        for line in fin:
            line = line.strip()
            if not line:
                continue
            try:
                entry = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not isinstance(entry, dict):
                continue
            fid = entry.get("fid") if entry.get("fid") is not None else entry.get("id", written)
            text = str(entry.get("text", "")).strip()
            audio = entry.get("audio") or entry.get("ref_audio") or ""
            if not text or not audio:
                continue
            conv_entry: dict = {"role": "assistant", "text": text, "wav": str(audio)}
            ref = entry.get("refWav") or entry.get("ref_wav")
            if ref:
                conv_entry["ref_wav"] = str(ref)
            fout.write(
                json.dumps({"id": str(fid), "conversations": [conv_entry]}, ensure_ascii=False) + "\n"
            )
            written += 1
    if written == 0:
        raise SystemExit("❌ 训练数据映射后无可用样本，请检查训练音频与文本清单。")
    return written


def canonicalize_checkpoint(output_model_path: Path) -> Path:
    """把最大的 ``checkpoint-epoch-{N}`` 或 ``checkpoint-{N}`` 重命名为 ``checkpoint_final/``。

    已存在 ``checkpoint_final/`` 则幂等返回。无候选则 raise SystemExit。
    """
    final = output_model_path / CHECKPOINT_FINAL_DIR
    if final.is_dir():
        return final
    candidates: list[tuple[int, Path]] = []
    for child in output_model_path.iterdir():
        if not child.is_dir():
            continue
        name = child.name
        n: int | None = None
        for prefix in ("checkpoint-epoch-", "checkpoint-"):
            if name.startswith(prefix):
                try:
                    n = int(name[len(prefix):])
                except ValueError:
                    n = None
                break
        if n is not None:
            candidates.append((n, child))
    if not candidates:
        raise SystemExit("❌ 未找到 checkpoint 目录，无法规整为 checkpoint_final。")
    candidates.sort(key=lambda x: x[0])
    candidates[-1][1].rename(final)
    return final


# --------------------------------------------------------------------------- #
# 微调主流程：映射 -> prepare_data -> sft -> 规整 checkpoint_final
# --------------------------------------------------------------------------- #
def run_training(params) -> None:
    output_dir = Path(params.output_model_path).expanduser().resolve()
    output_dir.mkdir(parents=True, exist_ok=True)
    mapped_jsonl = output_dir / "_conversations.jsonl"
    prepared_jsonl = output_dir / "_prepared.jsonl"

    print(
        f"[moss_tts_realtime] training device={params.device} "
        f"batch={params.batch_size} grad_accum={params.gradient_accumulation_steps} "
        f"epochs={params.num_epochs} lr={params.learning_rate}",
        flush=True,
    )
    print(f"[moss_tts_realtime] output_model_path={output_dir}", flush=True)

    # 1. 统一 JSONL -> MOSS conversations JSONL
    written = map_to_conversations(
        Path(params.input_jsonl).expanduser().resolve(), mapped_jsonl
    )
    print(f"[moss_tts_realtime] mapped {written} samples -> {mapped_jsonl}", flush=True)

    # 2. 预编码 audio_codes（上游脚本）。单卡直接 python 调用（Accelerator 单进程）；
    #    多卡需改用 accelerate launch（本期范围外，见 spec §10）。
    run_upstream_script(
        "moss_tts_realtime/finetuning/prepare_data.py",
        [
            "--codec-path", str(params.codec_path),
            "--device", params.device,
            "--input-jsonl", str(mapped_jsonl),
            "--output-jsonl", str(prepared_jsonl),
        ],
    )

    # 3. SFT 训练（上游脚本）。
    run_upstream_script(
        "moss_tts_realtime/finetuning/sft.py",
        [
            "--model-path", str(params.base_checkpoint),
            "--codec-path", str(params.codec_path),
            "--train-jsonl", str(prepared_jsonl),
            "--output-dir", str(output_dir),
            "--per-device-batch-size", str(params.batch_size),
            "--gradient-accumulation-steps", str(params.gradient_accumulation_steps),
            "--learning-rate", params.learning_rate,
            "--weight-decay", params.weight_decay,
            "--warmup-ratio", params.warmup_ratio,
            "--num-epochs", str(params.num_epochs),
            "--mixed-precision", params.mixed_precision,
            "--max-grad-norm", params.max_grad_norm,
        ],
    )

    # 4. 规整 canonical 产物（仅 sft.py 成功退出后执行）。
    final = canonicalize_checkpoint(output_dir)
    print(f"[moss_tts_realtime] canonical checkpoint: {final}", flush=True)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run MOSS-TTS-Realtime fine-tuning via kirine-client.")
    parser.add_argument(
        "--params-file",
        dest="params_file",
        type=str,
        required=True,
        help="Path to a JSON params file produced by the kirine-client UI.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    params = load_training_params(args.params_file)
    run_training(params)


if __name__ == "__main__":
    main()
