import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import streaming


class _WritableBuffer:
    def __init__(self):
        self._data = []

    def write(self, data):
        self._data.append(data)
        return len(data)

    def flush(self):
        return None

    def read_text(self):
        return b"".join(self._data).decode("utf-8")


def test_emit_error_frame_prints_message_to_stderr(capsys):
    frames_file = _WritableBuffer()

    streaming.emit_error_frame(frames_file, "ctx-1", "boom")

    captured = capsys.readouterr()
    assert "ctx-1" in captured.err
    assert "boom" in captured.err
    assert json.loads(frames_file.read_text().strip()) == {
        "type": "error",
        "contextId": "ctx-1",
        "message": "boom",
    }
