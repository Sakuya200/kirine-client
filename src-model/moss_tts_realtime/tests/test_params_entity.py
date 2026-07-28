import json

from params_entity import ParamsEntity, TaskKind


def _write(tmp_path, payload):
    p = tmp_path / "streaming.params.json"
    p.write_text(json.dumps(payload), encoding="utf-8")
    return p


def test_streaming_kind_enum():
    assert TaskKind.from_value("StreamingSpeech") is TaskKind.STREAMING_SPEECH


def test_streaming_args_parsed(tmp_path):
    p = _write(
        tmp_path,
        {
            "version": "1.0.0",
            "base_model": "moss_tts_realtime",
            "model_version": "1.7B",
            "kind": "StreamingSpeech",
            "runtime": {"device": "cuda"},
            "args": {
                "Streaming": {
                    "context_file_path": "/ctx.json",
                    "input_cache_file_path": "/in.jsonl",
                    "output_audio_dir": "/out",
                    "model_root_path": "/models",
                }
            },
        },
    )
    args = ParamsEntity.from_file(p).streaming_args()
    assert args.context_file_path == "/ctx.json"
    assert args.input_cache_file_path == "/in.jsonl"
    assert args.output_audio_dir == "/out"
    assert args.model_root_path == "/models"


def test_streaming_args_defaults_model_root_path(tmp_path):
    p = _write(
        tmp_path,
        {
            "kind": "StreamingSpeech",
            "args": {
                "Streaming": {
                    "context_file_path": "/c",
                    "input_cache_file_path": "/i",
                    "output_audio_dir": "/o",
                }
            },
        },
    )
    args = ParamsEntity.from_file(p).streaming_args()
    assert args.model_root_path == ""


def test_streaming_args_rejects_wrong_kind(tmp_path):
    p = _write(
        tmp_path,
        {
            "kind": "TextToSpeech",
            "args": {
                "TextToSpeech": {
                    "text": "hi",
                    "output_path": "/o",
                }
            },
        },
    )
    entity = ParamsEntity.from_file(p)
    try:
        entity.streaming_args()
    except ValueError:
        return
    raise AssertionError("expected ValueError when kind is not StreamingSpeech")
