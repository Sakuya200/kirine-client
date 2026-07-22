"""MOSS-TTS-Realtime 会话级流式语音合成入口。

对接 Rust 后端 ``run_streaming_session`` 契约：
- 读 ``streaming.params.json`` (kind=StreamingSpeech) 取路径与运行时；
- 读 ``context.json`` 的 ``basic.speakers``（trained 说话人带 speakerDirName）；
- 轮询 ``input.jsonl``，按 contextId 顺序合成，stdout 输出 started/chunk/finished/error 帧；
- chunk 字节：首 chunk = 44 字节 WAV 头（data size 哨兵 0xFFFFFFFF）+ PCM16，后续 chunk = PCM16，
  拼接为合法 WAV（前端 useStreamableAudioPlayer 累加整播）。

本文件先定义帧协议与 WAV 工具（可单测），会话主循环与合成在后续追加。
"""

from __future__ import annotations

import json
import struct
import sys
from pathlib import Path

SAMPLE_RATE = 24000


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
