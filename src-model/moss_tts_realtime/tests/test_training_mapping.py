import json
from pathlib import Path

from training import canonicalize_checkpoint, map_to_conversations


def _write_jsonl(p: Path, rows):
    p.write_text("\n".join(json.dumps(r) for r in rows) + "\n", encoding="utf-8")


def test_map_single_turn(tmp_path):
    src = tmp_path / "in.jsonl"
    out = tmp_path / "out.jsonl"
    _write_jsonl(src, [{"fid": "a1", "audio": "/x.wav", "text": "你好", "language": "chinese"}])
    n = map_to_conversations(src, out)
    assert n == 1
    row = json.loads(out.read_text(encoding="utf-8"))
    assert row["id"] == "a1"
    assert row["conversations"][0]["role"] == "assistant"
    assert row["conversations"][0]["text"] == "你好"
    assert row["conversations"][0]["wav"] == "/x.wav"
    assert "language" not in row["conversations"][0]


def test_map_skips_empty_and_supports_ref_wav(tmp_path):
    src = tmp_path / "in.jsonl"
    out = tmp_path / "out.jsonl"
    _write_jsonl(
        src,
        [
            {"audio": "", "text": "x"},
            {"audio": "/r.wav", "text": "hi", "refWav": "/ref.wav"},
            "not-json",
            {"audio": "/r2.wav", "text": "yo", "id": "b2"},
        ],
    )
    n = map_to_conversations(src, out)
    assert n == 2
    rows = [json.loads(l) for l in out.read_text(encoding="utf-8").splitlines()]
    assert rows[0]["conversations"][0]["ref_wav"] == "/ref.wav"
    assert rows[1]["id"] == "b2"


def test_map_raises_when_no_samples(tmp_path):
    src = tmp_path / "in.jsonl"
    out = tmp_path / "out.jsonl"
    _write_jsonl(src, [{"audio": "", "text": ""}])
    try:
        map_to_conversations(src, out)
    except SystemExit:
        return
    raise AssertionError("expected SystemExit when no samples")


def test_canonicalize_picks_largest_epoch(tmp_path):
    for n in (1, 3, 2):
        (tmp_path / f"checkpoint-epoch-{n}").mkdir()
    final = canonicalize_checkpoint(tmp_path)
    assert final.name == "checkpoint_final"
    assert (tmp_path / "checkpoint_final").is_dir()
    assert not (tmp_path / "checkpoint-epoch-3").exists()


def test_canonicalize_supports_plain_checkpoint_prefix(tmp_path):
    (tmp_path / "checkpoint-100").mkdir()
    (tmp_path / "checkpoint-20").mkdir()
    final = canonicalize_checkpoint(tmp_path)
    assert final.name == "checkpoint_final"
    assert (tmp_path / "checkpoint-100").exists() is False  # renamed
    assert (tmp_path / "checkpoint-20").exists() is True  # smaller kept


def test_canonicalize_idempotent(tmp_path):
    (tmp_path / "checkpoint_final").mkdir()
    (tmp_path / "checkpoint-epoch-1").mkdir()
    assert canonicalize_checkpoint(tmp_path).name == "checkpoint_final"
    # 第二次仍幂等，不会动 checkpoint-epoch-1
    assert (tmp_path / "checkpoint-epoch-1").exists()


def test_canonicalize_raises_when_no_checkpoint(tmp_path):
    (tmp_path / "logs").mkdir()
    try:
        canonicalize_checkpoint(tmp_path)
    except SystemExit:
        return
    raise AssertionError("expected SystemExit when no checkpoint dirs")
