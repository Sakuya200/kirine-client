import importlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch


SRC_ROOT = Path(__file__).resolve().parents[1] / "src"
MODEL_SCRIPT_PACKAGE = "qwen3_tts"

if str(SRC_ROOT) not in sys.path:
	sys.path.insert(0, str(SRC_ROOT))


def load_module(module_name: str):
	qualified_name = f"{MODEL_SCRIPT_PACKAGE}.{module_name}"
	sys.modules.pop(qualified_name, None)
	return importlib.import_module(qualified_name)


class FakeCode:
	def __init__(self, payload):
		self.payload = payload

	def cpu(self):
		return self

	def tolist(self):
		return self.payload


class FakeTokenizer:
	def __init__(self):
		self.calls = []

	def encode(self, audio_paths):
		self.calls.append(list(audio_paths))
		return SimpleNamespace(audio_codes=[FakeCode([1, 2, 3]) for _ in audio_paths])


class EncodeAudioTests(unittest.TestCase):
	def test_build_tokenizer_kwargs_switches_with_device(self):
		module = load_module("encode_audio")

		self.assertEqual(module.build_tokenizer_kwargs("cpu"), {"device_map": "cpu"})
		self.assertEqual(
			module.build_tokenizer_kwargs("cuda:0"),
			{"device_map": "cuda:0", "attn_implementation": "flash_attention_2"},
		)

	def test_main_writes_encoded_jsonl_in_cpu_mode(self):
		module = load_module("encode_audio")
		fake_tokenizer = FakeTokenizer()

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			input_path = temp_root / "input.jsonl"
			output_path = temp_root / "output.jsonl"
			input_rows = [
				{"audio": "sample-a.wav", "text": "hello"},
				{"audio": "sample-b.wav", "text": "world"},
			]
			with open(input_path, "w", encoding="utf-8") as file:
				for row in input_rows:
					file.write(json.dumps(row, ensure_ascii=False) + "\n")

			with patch.object(module, "load_tokenizer", return_value=fake_tokenizer):
				module.main(
					[
						"--device",
						"cpu",
						"--input_jsonl",
						str(input_path),
						"--output_jsonl",
						str(output_path),
						"--tokenizer_model_path",
						"fake-tokenizer",
					]
				)

			with open(output_path, "r", encoding="utf-8") as file:
				output_rows = [json.loads(line) for line in file if line.strip()]

			self.assertEqual(fake_tokenizer.calls, [["sample-a.wav", "sample-b.wav"]])
			self.assertEqual(output_rows[0]["audio_codes"], [1, 2, 3])
			self.assertEqual(output_rows[1]["audio_codes"], [1, 2, 3])


class FakeAccelerator:
	def __init__(self, **kwargs):
		self.kwargs = kwargs

	def prepare(self, model, optimizer, train_dataloader):
		return model, optimizer, train_dataloader


class FakeParameter:
	def __init__(self):
		self.device = "cpu"
		self.dtype = "float32"


class FakeModel:
	def parameters(self):
		return iter([FakeParameter()])


class FakeQwen3TTSModel:
	last_call = None

	@classmethod
	def from_pretrained(cls, model_path, **kwargs):
		cls.last_call = {"model_path": model_path, "kwargs": kwargs}
		return SimpleNamespace(processor=object(), model=FakeModel())


class FakeTTSDataset:
	def __init__(self, train_data, processor, config):
		self.train_data = train_data
		self.processor = processor
		self.config = config
		self.collate_fn = object()


class FakeDataLoader:
	def __init__(self, dataset, batch_size, shuffle, collate_fn):
		self.dataset = dataset
		self.batch_size = batch_size
		self.shuffle = shuffle
		self.collate_fn = collate_fn


class FakeAdamW:
	def __init__(self, parameters, lr, weight_decay):
		self.parameters = list(parameters)
		self.lr = lr
		self.weight_decay = weight_decay


class TrainingTests(unittest.TestCase):
	def build_dependencies(self):
		return SimpleNamespace(
			torch=SimpleNamespace(float32="float32", bfloat16="bfloat16"),
			Accelerator=FakeAccelerator,
			TTSDataset=FakeTTSDataset,
			Qwen3TTSModel=FakeQwen3TTSModel,
			save_file=lambda state_dict, save_path: None,
			AdamW=FakeAdamW,
			DataLoader=FakeDataLoader,
			AutoConfig=SimpleNamespace(from_pretrained=lambda _: {"config": True}),
		)

	def write_train_jsonl(self, temp_root: Path) -> Path:
		train_jsonl = temp_root / "train.jsonl"
		with open(train_jsonl, "w", encoding="utf-8") as file:
			file.write(
				json.dumps(
					{
						"audio": "sample.wav",
						"text": "hello",
						"audio_codes": [[1] * 16],
						"ref_audio": "ref.wav",
					},
					ensure_ascii=False,
				)
				+ "\n"
			)
		return train_jsonl

	def test_build_runtime_options_switches_between_cpu_and_gpu(self):
		module = load_module("training")
		torch_module = SimpleNamespace(float32="float32", bfloat16="bfloat16")
		cpu_args = module.parse_args(["--train_jsonl", "train.jsonl", "--device", "cpu"])
		gpu_args = module.parse_args(["--train_jsonl", "train.jsonl", "--device", "cuda:0"])

		cpu_runtime = module.build_runtime_options(cpu_args, torch_module)
		gpu_runtime = module.build_runtime_options(gpu_args, torch_module)

		self.assertTrue(cpu_runtime.is_cpu)
		self.assertEqual(cpu_runtime.accelerator_kwargs["mixed_precision"], "no")
		self.assertEqual(cpu_runtime.model_load_kwargs["torch_dtype"], "float32")
		self.assertNotIn("attn_implementation", cpu_runtime.model_load_kwargs)

		self.assertFalse(gpu_runtime.is_cpu)
		self.assertEqual(gpu_runtime.accelerator_kwargs["mixed_precision"], "bf16")
		self.assertEqual(gpu_runtime.model_load_kwargs["torch_dtype"], "bfloat16")
		self.assertEqual(gpu_runtime.model_load_kwargs["attn_implementation"], "flash_attention_2")

	def test_initialize_training_pipeline_uses_cpu_safe_configuration(self):
		module = load_module("training")
		dependencies = self.build_dependencies()

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			train_jsonl = self.write_train_jsonl(temp_root)
			args = module.parse_args(
				[
					"--train_jsonl",
					str(train_jsonl),
					"--device",
					"cpu",
					"--init_model_path",
					"fake-model",
				]
			)

			context = module.initialize_training_pipeline(args, dependencies)

		self.assertTrue(context.runtime.is_cpu)
		self.assertTrue(context.accelerator.kwargs["cpu"])
		self.assertEqual(context.accelerator.kwargs["mixed_precision"], "no")
		self.assertEqual(FakeQwen3TTSModel.last_call["kwargs"], {"torch_dtype": "float32"})
		self.assertEqual(len(context.train_data), 1)


class FakeTtsModel:
	last_call = None

	@classmethod
	def from_pretrained(cls, model_path, **kwargs):
		cls.last_call = {"model_path": model_path, "kwargs": kwargs}
		return cls()

	def generate_custom_voice(self, text, language, speaker, instruct):
		self.call = {
			"text": text,
			"language": language,
			"speaker": speaker,
			"instruct": instruct,
		}
		return [[0.1, 0.2]], 24000


class TtsTests(unittest.TestCase):
	def build_dependencies(self):
		writes = []

		def fake_write(path, wav, sr):
			writes.append({"path": path, "wav": wav, "sr": sr})

		return SimpleNamespace(
			torch=SimpleNamespace(float32="float32", bfloat16="bfloat16"),
			sf=SimpleNamespace(write=fake_write),
			Qwen3TTSModel=FakeTtsModel,
			writes=writes,
		)

	def test_build_runtime_options_switches_between_cpu_and_gpu(self):
		module = load_module("tts")
		torch_module = SimpleNamespace(float32="float32", bfloat16="bfloat16")
		cpu_args = module.parse_args(["--device", "cpu"])
		gpu_args = module.parse_args(["--device", "cuda:0"])

		cpu_runtime = module.build_runtime_options(cpu_args, torch_module)
		gpu_runtime = module.build_runtime_options(gpu_args, torch_module)

		self.assertTrue(cpu_runtime.is_cpu)
		self.assertEqual(cpu_runtime.model_load_kwargs, {"device_map": "cpu", "dtype": "float32"})
		self.assertFalse(gpu_runtime.is_cpu)
		self.assertEqual(
			gpu_runtime.model_load_kwargs,
			{"device_map": "cuda:0", "dtype": "bfloat16", "attn_implementation": "flash_attention_2"},
		)

	def test_generate_audio_uses_cpu_safe_model_config(self):
		module = load_module("tts")
		dependencies = self.build_dependencies()

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "checkpoint"
			model_dir.mkdir(parents=True)
			(model_dir / "config.json").write_text(
				json.dumps(
					{
						"talker_config": {
							"spk_id": {"speaker-a": 3000},
							"spk_is_dialect": {"speaker-a": False},
						}
					},
					ensure_ascii=False,
				),
				encoding="utf-8",
			)
			output_path = str(temp_root / "sample.wav")
			args = module.parse_args(
				[
					"--init_model_path",
					str(model_dir),
					"--text",
					"hello",
					"--speaker",
					"speaker-a",
					"--output_path",
					output_path,
					"--device",
					"cpu",
				]
			)

			runtime = module.generate_audio(args, dependencies)

		self.assertTrue(runtime.is_cpu)
		self.assertEqual(FakeTtsModel.last_call["kwargs"], {"device_map": "cpu", "dtype": "float32"})
		self.assertEqual(dependencies.writes[0]["path"], output_path)
		self.assertEqual(dependencies.writes[0]["sr"], 24000)

if __name__ == "__main__":
	unittest.main()
