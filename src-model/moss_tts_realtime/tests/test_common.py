from pathlib import Path

import pytest

import common
from common import (
    CHECKPOINT_FINAL_DIR,
    FFMPEG_DIR_NAME,
    ensure_ffmpeg_dlls,
    normalize_device,
    resolve_speaker_dir,
    resolve_trained_checkpoint,
)


def test_resolve_speaker_dir_none_when_missing_args():
    assert resolve_speaker_dir(None, "1") is None
    assert resolve_speaker_dir("/tmp", None) is None


def test_resolve_speaker_dir_none_when_absent(tmp_path):
    assert resolve_speaker_dir(str(tmp_path), "nope") is None


def test_resolve_speaker_dir_returns_path_when_present(tmp_path):
    spk = tmp_path / "42"
    spk.mkdir()
    result = resolve_speaker_dir(str(tmp_path), "42")
    assert result is not None
    assert result.name == "42"


def test_resolve_trained_checkpoint_none_when_no_final(tmp_path):
    (tmp_path / "42").mkdir()
    assert resolve_trained_checkpoint(str(tmp_path), "42") is None


def test_resolve_trained_checkpoint_finds_final(tmp_path):
    spk = tmp_path / "42"
    (spk / CHECKPOINT_FINAL_DIR).mkdir(parents=True)
    ckpt = resolve_trained_checkpoint(str(tmp_path), "42")
    assert ckpt is not None
    assert ckpt.name == CHECKPOINT_FINAL_DIR


def test_normalize_device():
    assert normalize_device(None) == "cuda"
    assert normalize_device("") == "cuda"
    assert normalize_device("cuda") == "cuda"
    assert normalize_device("cuda:0") == "cuda"
    assert normalize_device("CUDA") == "cuda"
    assert normalize_device("cpu") == "cpu"


def test_ensure_ffmpeg_dlls_raises_when_missing(monkeypatch, tmp_path):
    # ffmpeg 位于 src-model 同级目录；缺失时应报错（SystemExit），而非静默跳过。
    monkeypatch.setattr(common, "_SRC_MODEL_ROOT", tmp_path / "src-model")
    with pytest.raises(SystemExit):
        ensure_ffmpeg_dlls()


def test_ensure_ffmpeg_dlls_registers_sibling_of_src_model(monkeypatch, tmp_path):
    # ffmpeg bin 应解析到 src-model 的同级目录（_SRC_MODEL_ROOT.parent/ffmpeg-8.1.2/bin）。
    src_model = tmp_path / "src-model"
    src_model.mkdir()
    ffmpeg_bin = tmp_path / FFMPEG_DIR_NAME / "bin"
    ffmpeg_bin.mkdir(parents=True)
    monkeypatch.setattr(common, "_SRC_MODEL_ROOT", src_model)

    calls = []
    monkeypatch.setattr(common.os, "add_dll_directory", lambda p: calls.append(p))

    result = ensure_ffmpeg_dlls()
    assert result == ffmpeg_bin
    assert calls == [str(ffmpeg_bin)]
