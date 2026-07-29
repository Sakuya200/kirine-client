import json
from pathlib import Path

CONFIGS = Path(__file__).resolve().parent.parent / "configs"


def test_model_config_declares_streaming_only():
    cfg = json.loads((CONFIGS / "model-config.json").read_text(encoding="utf-8"))
    m = cfg["models"][0]
    assert m["baseModel"] == "moss_tts_realtime"
    assert m["supportedDevices"] == ["cuda", "cpu"]
    assert m["supportedFeatureList"] == ["streaming-speech"]


def test_params_config_has_streaming_block_only():
    cfg = json.loads((CONFIGS / "params-config.json").read_text(encoding="utf-8"))
    tasks = {b["task"] for b in cfg}
    assert tasks == {"streaming-speech"}


def test_streaming_params_names():
    cfg = json.loads((CONFIGS / "params-config.json").read_text(encoding="utf-8"))
    block = next(b for b in cfg if b["task"] == "streaming-speech")
    names = {p["name"] for p in block["params"]}
    assert {"temperature", "topP", "topK", "repetitionPenalty", "repetitionWindow", "maxLength"} <= names
