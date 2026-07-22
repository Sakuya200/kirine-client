"""MOSS-TTS-Realtime 会话级流式语音合成入口。

对接 Rust 后端 ``run_streaming_session`` 契约：
- 读 ``streaming.params.json`` (kind=StreamingSpeech) 取路径与运行时；
- 读 ``context.json`` 的 ``basic.speakers``（trained 说话人带 speakerDirName）；
- 轮询 ``input.jsonl``，按 contextId 顺序合成，stdout 输出 started/chunk/finished/error 帧；
- chunk 字节：首 chunk = 44 字节 WAV 头（data size 哨兵 0xFFFFFFFF）+ PCM16，后续 chunk = PCM16，
  拼接为合法 WAV（前端 useStreamableAudioPlayer 累加整播）。

多轮语义：每消息独立合成（voice prompt + 文本 -> 音频），轮间不保 KV cache，不采集 user 音频
（超出"只实现流式语音生成"范围）。上游 API 用法对齐 ``example_multiturn_stream_to_tts.py``。
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
import time
from pathlib import Path
from typing import Iterator

from common import base_checkpoint, codec_path, resolve_trained_checkpoint
from params import load_streaming_params

SAMPLE_RATE = 24000
POLL_INTERVAL_SECONDS = 0.1


# --------------------------------------------------------------------------- #
# 帧协议
# --------------------------------------------------------------------------- #
def emit_frame(payload: dict) -> None:
    """向 stdout 输出一行 JSON 帧（Rust runner 逐行读取分帧）。"""
    sys.stdout.write(json.dumps(payload, ensure_ascii=False) + "\n")
    sys.stdout.flush()


def wav_header_sentinel(sample_rate: int = SAMPLE_RATE, channels: int = 1, bits: int = 16) -> bytes:
    """44 字节 WAV 头，data size 字段填 0xFFFFFFFF 哨兵。

    流式场景下总数据长度未知，哨兵值让浏览器按实际到达数据解码、支持半截播放。
    RIFF size 同样填哨兵（36 + 0xFFFFFFFF 在 u32 内回绕，浏览器容忍）。
    """
    byte_rate = sample_rate * channels * bits // 8
    block_align = channels * bits // 8
    sentinel = 0xFFFFFFFF
    return struct.pack(
        "<4sI4s4sIHHIIHH4sI",
        b"RIFF",
        sentinel,
        b"WAVE",
        b"fmt ",
        16,
        1,  # PCM
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bits,
        b"data",
        sentinel,
    )


def float32_to_pcm16(samples: bytes) -> bytes:
    """mono float32 little-endian -> int16 little-endian，clip 到 [-1, 1]。"""
    import numpy as np

    arr = np.frombuffer(samples, dtype="<f4")
    arr = np.clip(arr, -1.0, 1.0)
    return (arr * 32767).astype("<i2").tobytes()


def _float32_np_to_pcm16(arr) -> bytes:
    """numpy float32 (-1..1) -> PCM16 bytes。"""
    import numpy as np

    arr = np.asarray(arr, dtype="<f4").reshape(-1)
    arr = np.clip(arr, -1.0, 1.0)
    return (arr * 32767).astype("<i2").tobytes()


def emit_chunk(context_id: str, pcm: bytes, header_sent: bool) -> bool:
    """发一帧 chunk。首帧自动前置 WAV 头。返回（新的）header_sent 状态。"""
    if not pcm:
        return header_sent
    parts: list[bytes] = [] if header_sent else [wav_header_sentinel()]
    parts.append(pcm)
    data = b"".join(parts)
    emit_frame({"type": "chunk", "contextId": context_id, "bytes": list(data)})
    return True


def save_wav(path: Path, pcm: bytes, sample_rate: int = SAMPLE_RATE) -> None:
    """把累积 PCM16 写成合法 WAV（data size 为实际长度，供历史回放）。"""
    byte_rate = sample_rate * 1 * 16 // 8
    data_size = len(pcm)
    header = struct.pack(
        "<4sI4s4sIHHIIHH4sI",
        b"RIFF",
        36 + data_size,
        b"WAVE",
        b"fmt ",
        16,
        1,
        1,
        sample_rate,
        byte_rate,
        2,
        16,
        b"data",
        data_size,
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(header + pcm)


# --------------------------------------------------------------------------- #
# input.jsonl 轮询
# --------------------------------------------------------------------------- #
def poll_new_lines(path: Path, last_offset: int) -> tuple[list[str], int]:
    """自 last_offset 读 input.jsonl 新增的非空行，返回 (lines, new_offset)。

    文件被截断/重建（size < last_offset）时重置到 0，避免漏读首条。
    """
    if not path.exists():
        return [], last_offset
    size = path.stat().st_size
    if size < last_offset:
        last_offset = 0
    new_lines: list[str] = []
    with path.open("r", encoding="utf-8") as f:
        f.seek(last_offset)
        for line in f:
            stripped = line.strip()
            if stripped:
                new_lines.append(stripped)
        last_offset = f.tell()
    return new_lines, last_offset


# --------------------------------------------------------------------------- #
# 模型 / codec 加载与音频编解码（用法对齐 example_multiturn_stream_to_tts.py）
# --------------------------------------------------------------------------- #
def _load_audio_mono(path: str, target_sample_rate: int = SAMPLE_RATE):
    """加载音频为 mono torch.Tensor (1, T)，重采样到目标采样率。"""
    import torch
    import torchaudio

    wav, sr = torchaudio.load(path)
    if sr != target_sample_rate:
        wav = torchaudio.functional.resample(wav, sr, target_sample_rate)
    if wav.shape[0] > 1:
        wav = wav.mean(dim=0, keepdim=True)
    return wav  # (1, T)


def encode_voice_prompt(codec, ref_audio_path: str, device) -> object:
    """codec.encode 参考音频 -> audio_codes tokens（voice-clone 声音提示）。"""
    import torch

    wav = _load_audio_mono(ref_audio_path, SAMPLE_RATE)
    with torch.inference_mode():
        result = codec.encode(wav.unsqueeze(0).to(device))
    return result["audio_codes"].squeeze(1).cpu().numpy()


def select_model_path(params, speakers: list[dict]) -> tuple[str, str | None]:
    """决定加载哪个 checkpoint：存在 trained 说话人 -> 其 checkpoint_final；否则基座。

    返回 (model_path, trained_speaker_dir_name_or_None)。一会话至多一个 trained。
    """
    trained = [
        s for s in speakers
        if (s.get("category") or "voice-clone") == "trained" and s.get("speakerDirName")
    ]
    if len(trained) > 1:
        raise SystemExit("❌ 流式会话至多支持一个已训练说话人。")
    if trained:
        spk = trained[0]
        ckpt = resolve_trained_checkpoint(params.model_root_path, spk.get("speakerDirName"))
        if ckpt is None:
            raise SystemExit(
                f"❌ 未找到已训练说话人 checkpoint_final: {spk.get('speakerDirName')}"
            )
        return str(ckpt), spk.get("speakerDirName")
    return str(base_checkpoint()), None


def load_model_and_codec(params, speakers: list[dict]):
    """加载 model + tokenizer + processor + codec。返回 (model, tokenizer, processor, codec, device)。"""
    import torch
    from transformers import AutoModel, AutoTokenizer

    from mossttsrealtime.modeling_mossttsrealtime import MossTTSRealtime
    from mossttsrealtime.processing_mossttsrealtime import MossTTSRealtimeProcessor

    if not torch.cuda.is_available():
        raise SystemExit("❌ MOSS-TTS-Realtime 仅支持 CUDA，未检测到可用 GPU。")

    device = torch.device("cuda")
    dtype = torch.bfloat16 if torch.cuda.is_bf16_supported() else torch.float16

    model_path, trained_dir = select_model_path(params, speakers)
    print(
        f"[moss_tts_realtime] loading model from {model_path}"
        + (f" (trained speaker dir={trained_dir})" if trained_dir else " (base)"),
        flush=True,
    )

    tokenizer = AutoTokenizer.from_pretrained(model_path)
    processor = MossTTSRealtimeProcessor(tokenizer)
    model = MossTTSRealtime.from_pretrained(
        model_path, attn_implementation="sdpa", torch_dtype=dtype
    ).to(device)
    model.eval()

    codec = AutoModel.from_pretrained(str(codec_path()), trust_remote_code=True).eval().to(device)
    return model, tokenizer, processor, codec, device


def _sanitize_tokens(tokens, codebook_size: int, audio_eos_token: int):
    """截断到 EOS / 非法 token 之前（对齐上游 example）。"""
    if tokens.dim() == 1:
        tokens = tokens.unsqueeze(0)
    if tokens.numel() == 0:
        return tokens
    eos_rows = (tokens[:, 0] == audio_eos_token).nonzero(as_tuple=False)
    invalid_rows = ((tokens < 0) | (tokens >= codebook_size)).any(dim=1)
    stop_idx = None
    if eos_rows.numel() > 0:
        stop_idx = int(eos_rows[0].item())
    if invalid_rows.any():
        invalid_idx = int(invalid_rows.nonzero(as_tuple=False)[0].item())
        stop_idx = invalid_idx if stop_idx is None else min(stop_idx, invalid_idx)
    if stop_idx is not None:
        tokens = tokens[:stop_idx]
    return tokens


def _decode_audio_frames(audio_frames, decoder, codebook_size: int, audio_eos_token: int) -> Iterator:
    """把 session 产出的 audio token 帧喂入 decoder，yield float32 numpy wav 片段。"""
    import numpy as np

    for frame in audio_frames:
        tokens = frame
        if tokens.dim() == 3:
            tokens = tokens[0]
        if tokens.dim() != 2:
            raise ValueError(f"Expected [T, C] audio tokens, got {tuple(tokens.shape)}")
        tokens = _sanitize_tokens(tokens, codebook_size, audio_eos_token)
        if tokens.numel() == 0:
            continue
        decoder.push_tokens(tokens.detach())
        for wav in decoder.audio_chunks():
            if wav.numel() == 0:
                continue
            yield wav.detach().cpu().numpy().reshape(-1)


def synthesize_one(
    params,
    model,
    tokenizer,
    processor,
    codec,
    device,
    speaker: dict,
    text: str,
    context_id: str,
    output_audio_dir: Path,
) -> None:
    """单条消息合成：started -> chunk* -> finished（异常 -> error，不退出进程）。"""
    from mossttsrealtime.streaming_mossttsrealtime import (
        AudioStreamDecoder,
        MossTTSRealtimeInference,
        MossTTSRealtimeStreamingSession,
    )

    emit_frame({"type": "started", "contextId": context_id})
    accumulated = bytearray()
    header_sent = False
    try:
        inferencer = MossTTSRealtimeInference(model, tokenizer, max_length=params.max_length)
        inferencer.reset_generation_state(keep_cache=False)
        session = MossTTSRealtimeStreamingSession(
            inferencer,
            processor,
            codec=codec,
            codec_sample_rate=SAMPLE_RATE,
            codec_encode_kwargs={},
            prefill_text_len=processor.delay_tokens_len,
            temperature=params.temperature,
            top_p=params.top_p,
            top_k=params.top_k,
            do_sample=True,
            repetition_penalty=params.repetition_penalty,
            repetition_window=params.repetition_window or None,
        )

        category = speaker.get("category") or "voice-clone"
        if category == "voice-clone":
            session.set_voice_prompt_tokens(speaker["_prompt_tokens"])
        # trained 说话人：模型已是微调 checkpoint，不设 voice prompt。

        decoder = AudioStreamDecoder(
            codec,
            chunk_frames=3,
            overlap_frames=0,
            decode_kwargs={"chunk_duration": -1},
            device=device,
        )
        codebook_size = int(getattr(codec.config, "codebook_size", 1024))
        audio_eos_token = int(getattr(session.inferencer, "audio_eos_token", 1026))

        def _emit_wav_chunks(wav_iter) -> None:
            nonlocal header_sent
            for wav_np in wav_iter:
                pcm = _float32_np_to_pcm16(wav_np)
                if pcm:
                    header_sent = emit_chunk(context_id, pcm, header_sent)
                    accumulated.extend(pcm)

        with codec.streaming(batch_size=1):
            _emit_wav_chunks(_decode_audio_frames(session.push_text(text), decoder, codebook_size, audio_eos_token))
            _emit_wav_chunks(_decode_audio_frames(session.end_text(), decoder, codebook_size, audio_eos_token))
            while True:
                frames = session.drain(max_steps=1)
                if not frames:
                    break
                _emit_wav_chunks(_decode_audio_frames(frames, decoder, codebook_size, audio_eos_token))
                if session.inferencer.is_finished:
                    break
            # flush decoder
            final = decoder.flush()
            if final is not None and final.numel() > 0:
                pcm = _float32_np_to_pcm16(final.detach().cpu().numpy().reshape(-1))
                if pcm:
                    header_sent = emit_chunk(context_id, pcm, header_sent)
                    accumulated.extend(pcm)

        if accumulated:
            save_wav(output_audio_dir / f"{context_id}.wav", bytes(accumulated), SAMPLE_RATE)
        emit_frame({"type": "finished", "contextId": context_id})
    except Exception as exc:  # noqa: BLE001
        emit_frame({"type": "error", "contextId": context_id, "message": str(exc)})


# --------------------------------------------------------------------------- #
# 会话主循环
# --------------------------------------------------------------------------- #
def load_context_speakers(context_file_path: str) -> list[dict]:
    """读 context.json 的 basic.speakers。"""
    with Path(context_file_path).open("r", encoding="utf-8") as f:
        ctx = json.load(f)
    return ctx.get("basic", {}).get("speakers", [])


def run_session(params) -> None:
    import torch

    speakers = load_context_speakers(params.context_file_path)
    if not speakers:
        raise SystemExit("❌ context.json 无 speakers，无法启动流式会话。")

    model, tokenizer, processor, codec, device = load_model_and_codec(params, speakers)

    # 预编码 voice-clone 说话人 prompt（缓存到 speaker["_prompt_tokens"]）。
    for spk in speakers:
        if (spk.get("category") or "voice-clone") == "voice-clone":
            ref = spk.get("refAudioPath") or ""
            if not ref:
                raise SystemExit(f"❌ voice-clone 说话人 {spk.get('name')} 缺参考音频。")
            print(f"[moss_tts_realtime] encoding voice prompt for {spk.get('name')}", flush=True)
            spk["_prompt_tokens"] = encode_voice_prompt(codec, ref, device)

    speakers_by_name = {s["name"]: s for s in speakers if "name" in s}
    input_path = Path(params.input_cache_file_path)
    output_audio_dir = Path(params.output_audio_dir)
    last_offset = 0

    print("[moss_tts_realtime] session ready, polling input.jsonl", flush=True)
    while True:
        lines, last_offset = poll_new_lines(input_path, last_offset)
        for line in lines:
            try:
                entry = json.loads(line)
            except json.JSONDecodeError as exc:
                print(f"[moss_tts_realtime] skip bad input line: {exc}", file=sys.stderr, flush=True)
                continue
            context_id = str(entry.get("contextId", ""))
            speaker_name = str(entry.get("speakerName", ""))
            text = str(entry.get("text", ""))
            if not context_id or not speaker_name:
                print(f"[moss_tts_realtime] skip input lacking contextId/speakerName", file=sys.stderr, flush=True)
                continue
            speaker = speakers_by_name.get(speaker_name)
            if speaker is None:
                emit_frame({
                    "type": "error",
                    "contextId": context_id,
                    "message": f"unknown speaker: {speaker_name}",
                })
                continue
            with torch.inference_mode():
                synthesize_one(
                    params, model, tokenizer, processor, codec, device,
                    speaker, text, context_id, output_audio_dir,
                )
        time.sleep(POLL_INTERVAL_SECONDS)


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="MOSS-TTS-Realtime streaming session for kirine-client.")
    parser.add_argument("--params-file", dest="params_file", type=str, required=True)
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    params = load_streaming_params(args.params_file)
    run_session(params)


if __name__ == "__main__":
    main()
