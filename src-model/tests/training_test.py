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
		self.sync_gradients = True
		self.clip_calls = []
		self.is_main_process = True

	def prepare(self, *items):
		return items

	def clip_grad_norm_(self, parameters, max_norm):
		self.clip_calls.append(max_norm)

	def unwrap_model(self, model):
		return model


class FakeParameter:
	def __init__(self):
		self.device = "cpu"
		self.dtype = "float32"
		self.requires_grad = True

	def numel(self):
		return 1


class FakeModel:
	def __init__(self):
		self.gradient_checkpointing_enabled = False
		self.input_require_grads_enabled = False
		self.config = SimpleNamespace(use_cache=True)
		self.talker = SimpleNamespace(
			config=SimpleNamespace(use_cache=True),
			model=SimpleNamespace(config=SimpleNamespace(use_cache=True)),
		)

	def parameters(self):
		return iter([FakeParameter()])

	def gradient_checkpointing_enable(self):
		self.gradient_checkpointing_enabled = True

	def enable_input_require_grads(self):
		self.input_require_grads_enabled = True


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
		self.step_calls = 0
		self.zero_grad_calls = 0

	def step(self):
		self.step_calls += 1

	def zero_grad(self):
		self.zero_grad_calls += 1


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
		module = load_module("training_full")
		torch_module = SimpleNamespace(float32="float32", bfloat16="bfloat16")
		cpu_args = module.parse_args(["--train_jsonl", "train.jsonl", "--device", "cpu"])
		gpu_args = module.parse_args(
			["--train_jsonl", "train.jsonl", "--device", "cuda:0", "--gradient-accumulation-steps", "8"]
		)

		cpu_runtime = module.build_runtime_options(cpu_args, torch_module)
		gpu_runtime = module.build_runtime_options(gpu_args, torch_module)

		self.assertTrue(cpu_runtime.is_cpu)
		self.assertEqual(cpu_runtime.accelerator_kwargs["mixed_precision"], "no")
		self.assertEqual(cpu_runtime.model_load_kwargs["torch_dtype"], "float32")
		self.assertNotIn("attn_implementation", cpu_runtime.model_load_kwargs)
		self.assertEqual(cpu_runtime.accelerator_kwargs["gradient_accumulation_steps"], 4)

		self.assertFalse(gpu_runtime.is_cpu)
		self.assertEqual(gpu_runtime.accelerator_kwargs["mixed_precision"], "bf16")
		self.assertEqual(gpu_runtime.accelerator_kwargs["gradient_accumulation_steps"], 8)
		self.assertEqual(gpu_runtime.model_load_kwargs["torch_dtype"], "bfloat16")
		self.assertEqual(gpu_runtime.model_load_kwargs["attn_implementation"], "flash_attention_2")

	def test_initialize_training_pipeline_uses_cpu_safe_configuration(self):
		module = load_module("training_full")
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

	def test_initialize_training_pipeline_enables_gradient_checkpointing(self):
		module = load_module("training_full")
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
					"--enable-gradient-checkpointing",
				]
			)

			context = module.initialize_training_pipeline(args, dependencies)

		self.assertTrue(context.model.gradient_checkpointing_enabled)
		self.assertFalse(context.model.input_require_grads_enabled)
		self.assertFalse(context.model.config.use_cache)
		self.assertFalse(context.model.talker.config.use_cache)
		self.assertFalse(context.model.talker.model.config.use_cache)

	def test_build_checkpoint_state_dict_drops_speaker_encoder_and_updates_speaker_embedding(self):
		module = load_module("training_full")

		class FakeTensorRow:
			def __init__(self, values, dtype="float32"):
				self.values = list(values)
				self.dtype = dtype

			def detach(self):
				return self

			def to(self, value):
				if value != "cpu":
					self.dtype = value
				return self

			def __eq__(self, other):
				return self.values == other

		class FakeTensor:
			def __init__(self, values, dtype="float32"):
				self.values = [list(row) for row in values]
				self.dtype = dtype

			def detach(self):
				return self

			def to(self, value):
				if value == "cpu":
					return self
				self.dtype = value
				return self

			def clone(self):
				return FakeTensor(self.values, self.dtype)

			def __getitem__(self, index):
				return FakeTensorRow(self.values[index], self.dtype)

			def __setitem__(self, index, value):
				self.values[index] = list(getattr(value, "values", value))

		class FakeCheckpointModel:
			def state_dict(self):
				return {
					"speaker_encoder.weight": FakeTensor([[9.0, 9.0]]),
					"talker.model.codec_embedding.weight": FakeTensor([[0.0, 0.0] for _ in range(3001)]),
					"talker.layer.weight": FakeTensor([[1.0, 2.0]]),
				}

		target_speaker_embedding = FakeTensor([[3.0, 4.0]], dtype="bfloat16")
		state_dict = module.build_checkpoint_state_dict(FakeCheckpointModel(), target_speaker_embedding)

		self.assertNotIn("speaker_encoder.weight", state_dict)
		self.assertEqual(state_dict["talker.model.codec_embedding.weight"][3000], [3.0, 4.0])
		self.assertEqual(state_dict["talker.layer.weight"][0], [1.0, 2.0])

	def test_save_training_checkpoint_uses_sharded_safe_serialization(self):
		module = load_module("training_full")

		class FakeTensorRow:
			def __init__(self, values, dtype="float32"):
				self.values = list(values)
				self.dtype = dtype

			def detach(self):
				return self

			def to(self, value):
				if value != "cpu":
					self.dtype = value
				return self

		class FakeTensor:
			def __init__(self, values, dtype="float32"):
				self.values = [list(row) for row in values]
				self.dtype = dtype

			def detach(self):
				return self

			def to(self, value):
				if value == "cpu":
					return self
				self.dtype = value
				return self

			def clone(self):
				return FakeTensor(self.values, self.dtype)

			def __getitem__(self, index):
				return FakeTensorRow(self.values[index], self.dtype)

			def __setitem__(self, index, value):
				self.values[index] = list(getattr(value, "values", value))

		class FakeCheckpointModel:
			_tied_weights_keys = ["talker.model.codec_embedding.weight"]

			def state_dict(self):
				return {
					"talker.model.codec_embedding.weight": FakeTensor([[0.0, 0.0] for _ in range(3001)]),
					"talker.layer.weight": FakeTensor([[1.0, 2.0]]),
				}

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "model"
			output_dir = temp_root / "output"
			model_dir.mkdir(parents=True)
			output_dir.mkdir(parents=True)
			(model_dir / "config.json").write_text("{}", encoding="utf-8")

			args = SimpleNamespace(
				init_model_path=str(model_dir),
				output_model_path=str(output_dir),
				speaker_name="speaker-a",
			)
			accelerator = FakeAccelerator()
			model = FakeCheckpointModel()
			target_speaker_embedding = FakeTensor([[7.0, 8.0]], dtype="bfloat16")

			with patch.object(module, "save_torch_state_dict") as save_state_dict:
				module.save_training_checkpoint(args, accelerator, model, target_speaker_embedding, epoch=0)

			save_state_dict.assert_called_once()
			_, save_dir = save_state_dict.call_args.args[:2]
			self.assertEqual(Path(save_dir), output_dir / "checkpoint-epoch-0")
			self.assertEqual(save_state_dict.call_args.kwargs["max_shard_size"], module.CHECKPOINT_MAX_SHARD_SIZE)
			self.assertTrue(save_state_dict.call_args.kwargs["safe_serialization"])

	def test_training_entry_routes_full_training_when_lora_disabled(self):
		full_module = load_module("training_full")
		entry_module = load_module("training")

		with patch.object(full_module, "train") as full_train:
			entry_module.train(["--train_jsonl", "train.jsonl", "--no-use-lora"])

		full_train.assert_called_once_with(["--train_jsonl", "train.jsonl"])

	def test_training_entry_routes_lora_requests_to_placeholder_module(self):
		lora_module = load_module("training_lora")
		entry_module = load_module("training")

		with patch.object(lora_module, "train") as lora_train:
			entry_module.train(["--train_jsonl", "train.jsonl", "--use-lora", "--lora-r", "8"])

		lora_train.assert_called_once_with(["--train_jsonl", "train.jsonl", "--lora-r", "8"])


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

	def write_tts_model_config(self, model_dir: Path, speakers: dict[str, int]):
		(model_dir / "config.json").write_text(
			json.dumps(
				{
					"talker_config": {
						"spk_id": speakers,
						"spk_is_dialect": {speaker_name: False for speaker_name in speakers},
					}
				},
				ensure_ascii=False,
			),
			encoding="utf-8",
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
			self.write_tts_model_config(model_dir, {"speaker-a": 3000})
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

	def test_resolve_speaker_name_matches_model_config_case_insensitively(self):
		module = load_module("tts")

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "checkpoint"
			model_dir.mkdir(parents=True)
			self.write_tts_model_config(model_dir, {"Vivian": 3000, "Ryan": 3001})

			args = module.parse_args(
				[
					"--init_model_path",
					str(model_dir),
					"--speaker",
					"vivian",
				]
			)

			self.assertEqual(module.resolve_speaker_name(args), "Vivian")

	def test_resolve_speaker_name_normalizes_separator_variants(self):
		module = load_module("tts")

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "checkpoint"
			model_dir.mkdir(parents=True)
			self.write_tts_model_config(model_dir, {"uncle_fu": 3000})

			args = module.parse_args(
				[
					"--init_model_path",
					str(model_dir),
					"--speaker",
					"Uncle-Fu",
				]
			)

			self.assertEqual(module.resolve_speaker_name(args), "uncle_fu")

	def test_resolve_speaker_name_rejects_unknown_explicit_speaker(self):
		module = load_module("tts")

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "checkpoint"
			model_dir.mkdir(parents=True)
			self.write_tts_model_config(model_dir, {"Vivian": 3000, "Ryan": 3001})

			args = module.parse_args(
				[
					"--init_model_path",
					str(model_dir),
					"--speaker",
					"unknown speaker",
				]
			)

			with self.assertRaisesRegex(ValueError, "Unsupported speaker"):
				module.resolve_speaker_name(args)

	def test_resolve_speaker_name_falls_back_to_first_model_speaker_when_missing(self):
		module = load_module("tts")

		with tempfile.TemporaryDirectory() as temp_dir:
			temp_root = Path(temp_dir)
			model_dir = temp_root / "checkpoint"
			model_dir.mkdir(parents=True)
			self.write_tts_model_config(model_dir, {"Vivian": 3000, "Ryan": 3001})

			args = module.parse_args(["--init_model_path", str(model_dir)])

			self.assertEqual(module.resolve_speaker_name(args), "Vivian")

if __name__ == "__main__":
	unittest.main()
