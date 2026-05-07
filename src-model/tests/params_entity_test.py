import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path


SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]

if str(SRC_MODEL_ROOT) not in sys.path:
    sys.path.insert(0, str(SRC_MODEL_ROOT))


def load_module(module_name: str):
    sys.modules.pop(module_name, None)
    return importlib.import_module(module_name)


class ParamsEntityParsingTests(unittest.TestCase):
    def test_params_entity_parses_training_payload(self):
        module = load_module("qwen3_tts.params_entity")
        payload = {
            "version": "1.0.0",
            "base_model": "qwen3_tts",
            "model_scale": "0.6B",
            "kind": "Training",
            "runtime": {
                "device": "cpu",
                "logging_dir": "logs",
                "attn_implementation": "sdpa",
            },
            "args": {
                "Training": {
                    "model_root_path": "models",
                    "speaker_dir_name": None,
                    "model_params_json": {"learningRate": "1e-5"},
                    "input_jsonl": "input.jsonl",
                    "output_jsonl": "output.jsonl",
                    "output_model_path": "out",
                    "batch_size": 2,
                    "lr": "3e-5",
                    "num_epochs": 4,
                    "speaker_name": "speaker_a",
                    "gradient_accumulation_steps": 8,
                    "enable_gradient_checkpointing": True,
                }
            },
        }

        params = module.ParamsEntity.from_json(json.dumps(payload, ensure_ascii=False))

        self.assertEqual(params.kind, module.TaskKind.TRAINING)
        self.assertEqual(params.base_model, "qwen3_tts")
        self.assertEqual(params.model_scale, "0.6B")
        self.assertEqual(params.runtime.device, "cpu")
        self.assertEqual(params.model_param_str("learningRate"), "1e-5")
        self.assertEqual(params.training_args().gradient_accumulation_steps, 8)


class ModelParamsLoaderTests(unittest.TestCase):
    def test_qwen3_tts_loader_uses_shared_params_entity(self):
        module = load_module("qwen3_tts.params")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            custom_model_dir = temp_root / "models" / "speaker_a"
            custom_model_dir.mkdir(parents=True, exist_ok=True)
            payload_path = temp_root / "qwen3_tts.json"
            payload = {
                "version": "1.0.0",
                "base_model": "qwen3_tts",
                "model_scale": "0.6B",
                "kind": "TextToSpeech",
                "runtime": {"device": "cpu"},
                "args": {
                    "TextToSpeech": {
                        "model_root_path": str(temp_root / "models"),
                        "speaker_dir_name": "speaker_a",
                        "model_params_json": {"voicePrompt": "calm"},
                        "text": "hello",
                        "language": "english",
                        "speaker": "speaker_a",
                        "output_path": str(temp_root / "out.wav"),
                    }
                },
            }
            payload_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

            params = module.load_tts_params(payload_path)

        self.assertEqual(params.text, "hello")
        self.assertEqual(params.runtime.device, "cpu")
        self.assertEqual(params.instruct, "calm")
        self.assertEqual(params.init_model_path, str(custom_model_dir.resolve()))

    def test_vox_tts_loader_resolves_latest_checkpoint(self):
        module = load_module("vox_cpm2.params")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            latest_checkpoint = temp_root / "models" / "speaker_b" / "checkpoints" / "lora" / "latest"
            latest_checkpoint.mkdir(parents=True, exist_ok=True)
            payload_path = temp_root / "vox.json"
            payload = {
                "version": "1.0.0",
                "base_model": "vox_cpm2",
                "model_scale": "2B",
                "kind": "TextToSpeech",
                "runtime": {},
                "args": {
                    "TextToSpeech": {
                        "model_root_path": str(temp_root / "models"),
                        "speaker_dir_name": "speaker_b",
                        "model_params_json": {"cfgValue": "2.5", "inferenceTimesteps": 12},
                        "text": "hello",
                        "language": "chinese",
                        "speaker": "speaker_b",
                        "output_path": str(temp_root / "out.wav"),
                    }
                },
            }
            payload_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

            params = module.load_tts_params(payload_path)

        self.assertEqual(params.cfg_value, 2.5)
        self.assertEqual(params.inference_timesteps, 12)
        self.assertEqual(params.init_model_path, str(latest_checkpoint.resolve()))
        self.assertEqual(params.runtime.attn_implementation, "auto")

    def test_moss_voice_clone_loader_resolves_bundled_model(self):
        module = load_module("moss_tts_local.params")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            speaker_root = temp_root / "models" / "speaker_c"
            bundled_model_root = speaker_root / module.MOSS_MODEL_NAME
            bundled_model_root.mkdir(parents=True, exist_ok=True)
            ref_audio_path = temp_root / "ref.wav"
            ref_audio_path.write_bytes(b"RIFF")
            payload_path = temp_root / "moss.json"
            payload = {
                "version": "1.0.0",
                "base_model": "moss_tts_local",
                "model_scale": "1.7B",
                "kind": "VoiceClone",
                "runtime": {"device": "cpu"},
                "args": {
                    "VoiceClone": {
                        "model_root_path": str(temp_root / "models"),
                        "speaker_dir_name": "speaker_c",
                        "model_params_json": {"nVqForInference": 6},
                        "ref_audio_path": str(ref_audio_path),
                        "ref_text": "demo",
                        "language": "english",
                        "output_path": str(temp_root / "out.wav"),
                        "text": "target",
                    }
                },
            }
            payload_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

            params = module.load_voice_clone_params(payload_path)

        self.assertEqual(params.n_vq_for_inference, 6)
        self.assertEqual(params.runtime.device, "cpu")
        self.assertEqual(params.init_model_path, str(bundled_model_root.resolve()))

    def test_moss_training_loader_keeps_generic_runtime_attn_implementation(self):
        module = load_module("moss_tts_local.params")

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_root = Path(temp_dir)
            (temp_root / "models" / module.MOSS_MODEL_NAME).mkdir(parents=True, exist_ok=True)
            (temp_root / "models" / module.MOSS_AUDIO_TOKENIZER_NAME).mkdir(parents=True, exist_ok=True)
            payload_path = temp_root / "moss-train.json"
            payload = {
                "version": "1.0.0",
                "base_model": "moss_tts_local",
                "model_scale": "1.7B",
                "kind": "Training",
                "runtime": {"device": "cpu", "attn_implementation": "sdpa"},
                "args": {
                    "Training": {
                        "model_root_path": str(temp_root / "models"),
                        "speaker_dir_name": None,
                        "model_params_json": {},
                        "input_jsonl": str(temp_root / "train.jsonl"),
                        "output_jsonl": str(temp_root / "encoded.jsonl"),
                        "output_model_path": str(temp_root / "out"),
                        "batch_size": 2,
                        "lr": "1e-5",
                        "num_epochs": 3,
                        "speaker_name": "speaker_moss",
                        "gradient_accumulation_steps": 4,
                        "enable_gradient_checkpointing": True,
                    }
                },
            }
            payload_path.write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")

            params = module.load_training_params(payload_path)

        self.assertEqual(params.runtime.attn_implementation, "sdpa")
        self.assertEqual(params.to_namespace().attn_implementation, "sdpa")


if __name__ == "__main__":
    unittest.main()