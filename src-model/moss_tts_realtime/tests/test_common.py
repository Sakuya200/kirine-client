from pathlib import Path

from common import (
    CHECKPOINT_FINAL_DIR,
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
