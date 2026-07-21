import json

from params import load_streaming_params


def _write(tmp_path, payload):
    p = tmp_path / "p.json"
    p.write_text(json.dumps(payload), encoding="utf-8")
    return p


def test_load_streaming_params_uses_defaults(tmp_path):
    p = _write(
        tmp_path,
        {
            "kind": "StreamingSpeech",
            "runtime": {"device": "cuda"},
            "args": {
                "Streaming": {
                    "context_file_path": "/c",
                    "input_cache_file_path": "/i",
                    "output_audio_dir": "/o",
                    "model_root_path": "/m",
                }
            },
        },
    )
    sp = load_streaming_params(p)
    assert sp.device == "cuda"
    assert sp.context_file_path == "/c"
    assert sp.model_root_path == "/m"
    assert sp.temperature == 0.8
    assert sp.top_p == 0.6
    assert sp.top_k == 30
    assert sp.repetition_penalty == 1.1
    assert sp.repetition_window == 50
    assert sp.max_length == 32768


def test_load_streaming_params_overrides(tmp_path):
    p = _write(
        tmp_path,
        {
            "kind": "StreamingSpeech",
            "runtime": {"device": "cuda:0"},
            "args": {
                "Streaming": {
                    "context_file_path": "/c",
                    "input_cache_file_path": "/i",
                    "output_audio_dir": "/o",
                    "model_params_json": {
                        "temperature": 0.5,
                        "topK": 20,
                        "repetitionWindow": 10,
                    },
                }
            },
        },
    )
    sp = load_streaming_params(p)
    assert sp.device == "cuda"
    assert sp.temperature == 0.5
    assert sp.top_k == 20
    assert sp.repetition_window == 10
    assert sp.model_root_path == ""


def test_load_streaming_params_cpu_normalized(tmp_path):
    p = _write(
        tmp_path,
        {
            "kind": "StreamingSpeech",
            "runtime": {"device": "cpu"},
            "args": {
                "Streaming": {
                    "context_file_path": "/c",
                    "input_cache_file_path": "/i",
                    "output_audio_dir": "/o",
                }
            },
        },
    )
    assert load_streaming_params(p).device == "cpu"
