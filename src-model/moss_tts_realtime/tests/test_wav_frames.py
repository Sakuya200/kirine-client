import struct
from pathlib import Path

import streaming


def test_wav_header_is_44_bytes_with_sentinel():
    h = streaming.wav_header_sentinel()
    assert len(h) == 44
    assert h[:4] == b"RIFF"
    assert h[8:12] == b"WAVE"
    assert h[12:16] == b"fmt "
    assert h[36:40] == b"data"
    riff_size = struct.unpack("<I", h[4:8])[0]
    data_size = struct.unpack("<I", h[40:44])[0]
    assert riff_size == 0xFFFFFFFF
    assert data_size == 0xFFFFFFFF


def test_float32_to_pcm16_clips_and_converts():
    raw = struct.pack("<3f", 0.5, -0.5, 1.5)
    pcm = streaming.float32_to_pcm16(raw)
    vals = struct.unpack("<3h", pcm)
    assert vals[0] == 16383
    assert vals[1] == -16383
    assert vals[2] == 32767


def test_header_plus_pcm_concatenates_to_valid_wav():
    h = streaming.wav_header_sentinel()
    pcm = struct.pack("<2h", 100, -100)
    blob = h + pcm
    assert blob[:4] == b"RIFF"
    assert blob[36:40] == b"data"
    # 紧跟头后的就是 PCM 数据
    assert blob[44:] == pcm


def test_save_wav_writes_valid_header_with_real_data_size(tmp_path):
    pcm = struct.pack("<2h", 100, -100)
    out = tmp_path / "msg-1.wav"
    streaming.save_wav(out, pcm)
    blob = out.read_bytes()
    assert blob[:4] == b"RIFF"
    data_size = struct.unpack("<I", blob[40:44])[0]
    assert data_size == len(pcm)
    assert blob[44:] == pcm


def test_poll_new_lines_appends_only_new(tmp_path):
    f = tmp_path / "in.jsonl"
    f.write_text('{"a":1}\n', encoding="utf-8")
    lines, off = streaming.poll_new_lines(f, 0)
    assert len(lines) == 1
    assert off > 0
    with f.open("a", encoding="utf-8") as fh:
        fh.write('{"a":2}\n')
    lines2, off2 = streaming.poll_new_lines(f, off)
    assert len(lines2) == 1
    assert off2 > off


def test_poll_handles_truncation(tmp_path):
    f = tmp_path / "in.jsonl"
    f.write_text("xxxxxxxxxx\n", encoding="utf-8")
    _, off = streaming.poll_new_lines(f, 0)
    f.write_text('{"a":1}\n', encoding="utf-8")  # 文件缩短
    lines, _ = streaming.poll_new_lines(f, off)
    assert len(lines) == 1


def test_poll_returns_empty_when_file_absent(tmp_path):
    lines, off = streaming.poll_new_lines(tmp_path / "nope.jsonl", 0)
    assert lines == []
    assert off == 0
