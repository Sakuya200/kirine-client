import json
from pathlib import Path

CONFIGS = Path(__file__).resolve().parent.parent / "configs"


def test_model_config_declares_streaming_and_training():
    cfg = json.loads((CONFIGS / "model-config.json").read_text(encoding="utf-8"))
    m = cfg["models"][0]
    assert m["baseModel"] == "moss_tts_realtime"
    assert m["supportedDevices"] == ["cuda"]
    assert "streaming-speech" in m["supportedFeatureList"]
    assert "model-training" in m["supportedFeatureList"]


def test_params_config_has_streaming_and_training_blocks():
    cfg = json.loads((CONFIGS / "params-config.json").read_text(encoding="utf-8"))
    tasks = {b["task"] for b in cfg}
    assert "streaming-speech" in tasks
    assert "model-training" in tasks


def test_streaming_params_names():
    cfg = json.loads((CONFIGS / "params-config.json").read_text(encoding="utf-8"))
    block = next(b for b in cfg if b["task"] == "streaming-speech")
    names = {p["name"] for p in block["params"]}
    assert {"temperature", "topP", "topK", "repetitionPenalty", "repetitionWindow", "maxLength"} <= names


def test_training_params_names():
    cfg = json.loads((CONFIGS / "params-config.json").read_text(encoding="utf-8"))
    block = next(b for b in cfg if b["task"] == "model-training")
    names = {p["name"] for p in block["params"]}
    assert {
        "epochCount",
        "batchSize",
        "gradientAccumulationSteps",
        "learningRate",
        "weightDecay",
        "warmupRatio",
        "mixedPrecision",
        "maxGradNorm",
    } <= names
