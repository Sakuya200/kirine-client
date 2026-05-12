from __future__ import annotations

from argparse import Namespace
from dataclasses import dataclass
from pathlib import Path
import subprocess
import sys
import tempfile

from gpt_sovits_cpufast.params_entity import ParamsEntity, RuntimeOptions


GPT_SOVITS_CPUFAST_BASE_MODEL = "gpt_sovits_cpufast"

MODEL_VERSION_CHECKPOINTS = {
	"v1": (
		"GPT_SoVITS/pretrained_models/s1bert25hz-2kh-longer-epoch=68e-step=50232.ckpt",
		"GPT_SoVITS/pretrained_models/s2G488k.pth",
	),
	"v2": (
		"GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
		"GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth",
	),
	"v2pro": (
		"GPT_SoVITS/pretrained_models/s1v3.ckpt",
		"GPT_SoVITS/pretrained_models/v2Pro/s2Gv2Pro.pth",
	),
	"v2proplus": (
		"GPT_SoVITS/pretrained_models/s1v3.ckpt",
		"GPT_SoVITS/pretrained_models/v2Pro/s2Gv2ProPlus.pth",
	),
	"v2pp": (
		"GPT_SoVITS/pretrained_models/s1v3.ckpt",
		"GPT_SoVITS/pretrained_models/v2Pro/s2Gv2ProPlus.pth",
	),
}


def _require_path(path: str | None, label: str) -> Path:
	if path is None or not str(path).strip():
		raise ValueError(f"GPT-SoVITS-CPUFast params payload is missing {label}")
	return Path(path).expanduser().resolve()


def _resolve_bridge_script_path() -> Path:
	bridge_script_path = Path(__file__).resolve().with_name("inference_bridge.py")
	if not bridge_script_path.exists():
		raise FileNotFoundError(f"GPT-SoVITS-CPUFast bridge script not found: {bridge_script_path}")
	return bridge_script_path


def _normalize_runtime(runtime: RuntimeOptions) -> RuntimeOptions:
	return RuntimeOptions(
		device=runtime.device or "cpu",
		logging_dir=runtime.logging_dir or "",
		attn_implementation=runtime.attn_implementation or "sdpa",
	)


def _resolve_cpufast_root(model_root_path: str | None) -> Path:
	root_path = _require_path(model_root_path, "model_root_path")
	candidate_roots = [
		root_path,
		root_path / GPT_SOVITS_CPUFAST_BASE_MODEL,
		root_path / "base-models" / GPT_SOVITS_CPUFAST_BASE_MODEL,
		root_path / "GPT-SoVITS-CPUFast",
	]

	for candidate_root in candidate_roots:
		if (candidate_root / "GPT_SoVITS" / "inference_webui_fast.py").exists():
			return candidate_root

	raise FileNotFoundError(f"GPT-SoVITS-CPUFast root not found under: {root_path}")


def _normalize_model_version(model_version: str | None) -> str:
	normalized = (model_version or "").strip().lower()
	if normalized in MODEL_VERSION_CHECKPOINTS:
		return normalized
	if normalized == "v2pro+":
		return "v2proplus"
	raise ValueError(
		"Unsupported GPT-SoVITS-CPUFast model_version: "
		f"{model_version}. Supported values include V1, V2, V2Pro, V2ProPlus (or v2pp)."
	)


def _resolve_checkpoint_paths(cpufast_root: Path, model_version: str | None) -> tuple[Path, Path]:
	scale_key = _normalize_model_version(model_version)
	gpt_rel, sovits_rel = MODEL_VERSION_CHECKPOINTS[scale_key]
	gpt_path = cpufast_root / gpt_rel
	sovits_path = cpufast_root / sovits_rel

	if not gpt_path.exists():
		raise FileNotFoundError(f"GPT checkpoint not found for {scale_key}: {gpt_path}")
	if not sovits_path.exists():
		raise FileNotFoundError(f"SoVITS checkpoint not found for {scale_key}: {sovits_path}")

	return gpt_path, sovits_path


def _resolve_ref_audio_path(ref_audio_path: str | None, label: str = "refAudioPath") -> Path:
	path = _require_path(ref_audio_path, label)
	if not path.exists():
		raise FileNotFoundError(f"Reference audio file not found: {path}")
	return path


def _resolve_ref_text_path(ref_text_path: str | None) -> Path:
	path = _require_path(ref_text_path, "refTextPath")
	if not path.exists():
		raise FileNotFoundError(f"Reference text file not found: {path}")
	return path


def _resolve_optional_ref_text_path(ref_text_path: str | None) -> Path | None:
	if ref_text_path is None or not str(ref_text_path).strip():
		return None
	path = Path(ref_text_path).expanduser().resolve()
	if not path.exists():
		raise FileNotFoundError(f"Reference text file not found: {path}")
	return path


def _resolve_output_path(output_path: str) -> Path:
	resolved = Path(output_path).expanduser().resolve()
	resolved.parent.mkdir(parents=True, exist_ok=True)
	return resolved


def _map_runtime_language(language: str | None) -> str:
	mapping = {
		"chinese": "zh",
		"english": "en",
		"japanese": "ja",
	}
	normalized = (language or "chinese").strip().lower()
	if normalized not in mapping:
		raise ValueError(f"Unsupported GPT-SoVITS-CPUFast language: {language}")
	return mapping[normalized]


def _parse_model_param_int(params: ParamsEntity, key: str, default: int) -> int:
	value = params.model_param(key, default)
	return int(value)


def _parse_model_param_float(params: ParamsEntity, key: str, default: float) -> float:
	value = params.model_param(key, default)
	return float(value)


def _parse_model_param_bool(params: ParamsEntity, key: str, default: bool) -> bool:
	return params.model_param_bool(key, default)


@dataclass
class GptSovitsCpufastTtsParams:
	bridge_script_path: Path
	cpufast_root: Path
	gpt_model_path: Path
	sovits_model_path: Path
	ref_audio_path: Path
	ref_text_path: Path
	target_text: str
	target_language: str
	prompt_language: str
	top_k: int
	top_p: float
	temperature: float
	text_split_method: str
	batch_size: int
	speed_factor: float
	split_bucket: bool
	fragment_interval: float
	parallel_infer: bool
	seed: int
	output_path: Path
	runtime: RuntimeOptions

	def to_namespace(self) -> Namespace:
		return Namespace(
			bridge_script_path=str(self.bridge_script_path),
			cpufast_root=str(self.cpufast_root),
			gpt_model_path=str(self.gpt_model_path),
			sovits_model_path=str(self.sovits_model_path),
			ref_audio_path=str(self.ref_audio_path),
			ref_text_path=str(self.ref_text_path),
			target_text=self.target_text,
			target_language=self.target_language,
			prompt_language=self.prompt_language,
			top_k=self.top_k,
			top_p=self.top_p,
			temperature=self.temperature,
			text_split_method=self.text_split_method,
			batch_size=self.batch_size,
			speed_factor=self.speed_factor,
			split_bucket=self.split_bucket,
			fragment_interval=self.fragment_interval,
			parallel_infer=self.parallel_infer,
			seed=self.seed,
			output_path=str(self.output_path),
			device=self.runtime.device,
		)


@dataclass
class GptSovitsCpufastVoiceCloneParams:
	bridge_script_path: Path
	cpufast_root: Path
	gpt_model_path: Path
	sovits_model_path: Path
	ref_audio_path: Path
	ref_text_path: Path | None
	ref_text: str
	target_text: str
	target_language: str
	prompt_language: str
	top_k: int
	top_p: float
	temperature: float
	text_split_method: str
	batch_size: int
	speed_factor: float
	split_bucket: bool
	fragment_interval: float
	parallel_infer: bool
	seed: int
	output_path: Path
	runtime: RuntimeOptions

	def to_namespace(self) -> Namespace:
		return Namespace(
			bridge_script_path=str(self.bridge_script_path),
			cpufast_root=str(self.cpufast_root),
			gpt_model_path=str(self.gpt_model_path),
			sovits_model_path=str(self.sovits_model_path),
			ref_audio_path=str(self.ref_audio_path),
			ref_text_path=str(self.ref_text_path) if self.ref_text_path else None,
			ref_text=self.ref_text,
			target_text=self.target_text,
			target_language=self.target_language,
			prompt_language=self.prompt_language,
			top_k=self.top_k,
			top_p=self.top_p,
			temperature=self.temperature,
			text_split_method=self.text_split_method,
			batch_size=self.batch_size,
			speed_factor=self.speed_factor,
			split_bucket=self.split_bucket,
			fragment_interval=self.fragment_interval,
			parallel_infer=self.parallel_infer,
			seed=self.seed,
			output_path=str(self.output_path),
			device=self.runtime.device,
		)


def load_tts_params(path: str | Path) -> GptSovitsCpufastTtsParams:
	params = ParamsEntity.from_file(path)
	args = params.tts_args()
	cpufast_root = _resolve_cpufast_root(args.common.model_root_path)
	gpt_model_path, sovits_model_path = _resolve_checkpoint_paths(cpufast_root, params.model_version)
	runtime = _normalize_runtime(params.runtime)

	return GptSovitsCpufastTtsParams(
		bridge_script_path=_resolve_bridge_script_path(),
		cpufast_root=cpufast_root,
		gpt_model_path=gpt_model_path,
		sovits_model_path=sovits_model_path,
		ref_audio_path=_resolve_ref_audio_path(params.model_param_str("refAudioPath", None)),
		ref_text_path=_resolve_ref_text_path(params.model_param_str("refTextPath", None)),
		target_text=args.text.strip(),
		target_language=_map_runtime_language(args.language),
		prompt_language=_map_runtime_language(params.model_param_str("promptLang", args.language)),
		top_k=_parse_model_param_int(params, "topK", 15),
		top_p=_parse_model_param_float(params, "topP", 1.0),
		temperature=_parse_model_param_float(params, "temperature", 1.0),
		text_split_method=params.model_param_str("textSplitMethod", "cut5") or "cut5",
		batch_size=_parse_model_param_int(params, "batchSize", 1),
		speed_factor=_parse_model_param_float(params, "speedFactor", 1.0),
		split_bucket=_parse_model_param_bool(params, "splitBucket", True),
		fragment_interval=_parse_model_param_float(params, "fragmentInterval", 0.3),
		parallel_infer=_parse_model_param_bool(params, "parallelInfer", True),
		seed=_parse_model_param_int(params, "seed", -1),
		output_path=_resolve_output_path(args.output_path),
		runtime=runtime,
	)


def load_voice_clone_params(path: str | Path) -> GptSovitsCpufastVoiceCloneParams:
	params = ParamsEntity.from_file(path)
	args = params.voice_clone_args()
	cpufast_root = _resolve_cpufast_root(args.common.model_root_path)
	gpt_model_path, sovits_model_path = _resolve_checkpoint_paths(cpufast_root, params.model_version)
	runtime = _normalize_runtime(params.runtime)

	return GptSovitsCpufastVoiceCloneParams(
		bridge_script_path=_resolve_bridge_script_path(),
		cpufast_root=cpufast_root,
		gpt_model_path=gpt_model_path,
		sovits_model_path=sovits_model_path,
		ref_audio_path=_resolve_ref_audio_path(
			params.model_param_str("refAudioPath", args.ref_audio_path)
		),
		ref_text_path=_resolve_optional_ref_text_path(params.model_param_str("refTextPath", None)),
		ref_text=(args.ref_text or "").strip(),
		target_text=args.text.strip(),
		target_language=_map_runtime_language(args.language),
		prompt_language=_map_runtime_language(params.model_param_str("promptLang", args.language)),
		top_k=_parse_model_param_int(params, "topK", 15),
		top_p=_parse_model_param_float(params, "topP", 1.0),
		temperature=_parse_model_param_float(params, "temperature", 1.0),
		text_split_method=params.model_param_str("textSplitMethod", "cut5") or "cut5",
		batch_size=_parse_model_param_int(params, "batchSize", 1),
		speed_factor=_parse_model_param_float(params, "speedFactor", 1.0),
		split_bucket=_parse_model_param_bool(params, "splitBucket", True),
		fragment_interval=_parse_model_param_float(params, "fragmentInterval", 0.3),
		parallel_infer=_parse_model_param_bool(params, "parallelInfer", True),
		seed=_parse_model_param_int(params, "seed", -1),
		output_path=_resolve_output_path(args.output_path),
		runtime=runtime,
	)


def run_inference_cli(args: Namespace) -> None:
	if not args.target_text.strip():
		raise ValueError("Input text cannot be empty.")

	with tempfile.TemporaryDirectory(prefix="gpt-sovits-cpufast-") as temp_dir:
		temp_root = Path(temp_dir)
		target_text_path = temp_root / "target.txt"
		target_text_path.write_text(args.target_text, encoding="utf-8")

		ref_text_path = args.ref_text_path
		if not ref_text_path:
			ref_text_path = temp_root / "ref_text.txt"
			Path(ref_text_path).write_text((args.ref_text or "").strip(), encoding="utf-8")

		command = [
			sys.executable,
			str(args.bridge_script_path),
			"--gpt_model",
			str(args.gpt_model_path),
			"--sovits_model",
			str(args.sovits_model_path),
			"--ref_audio",
			str(args.ref_audio_path),
			"--ref_text",
			str(ref_text_path),
			"--prompt_language",
			str(args.prompt_language),
			"--target_text",
			str(target_text_path),
			"--target_language",
			str(args.target_language),
			"--output_path",
			str(args.output_path),
			"--top_k",
			str(args.top_k),
			"--top_p",
			str(args.top_p),
			"--temperature",
			str(args.temperature),
			"--speed_factor",
			str(args.speed_factor),
			"--text_split_method",
			str(args.text_split_method),
			"--batch_size",
			str(args.batch_size),
			"--fragment_interval",
			str(args.fragment_interval),
			"--seed",
			str(args.seed),
			"--device",
			str(args.device),
		]

		if args.split_bucket:
			command.append("--split_bucket")
		if args.parallel_infer:
			command.append("--parallel_infer")

		subprocess.run(command, check=True, cwd=args.cpufast_root)

		final_output_path = Path(args.output_path).expanduser().resolve()
		if not final_output_path.exists():
			raise FileNotFoundError(
				f"GPT-SoVITS-CPUFast output file not found: {final_output_path}"
			)
