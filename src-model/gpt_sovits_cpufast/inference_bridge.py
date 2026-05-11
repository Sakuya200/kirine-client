from __future__ import annotations

import argparse
import importlib
from pathlib import Path
import sys


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
	parser = argparse.ArgumentParser(description="GPT-SoVITS-CPUFast advanced CLI bridge")
	parser.add_argument("--gpt_model", required=True)
	parser.add_argument("--sovits_model", required=True)
	parser.add_argument("--ref_audio", required=True)
	parser.add_argument("--ref_text", required=True)
	parser.add_argument("--prompt_language", required=True)
	parser.add_argument("--target_text", required=True)
	parser.add_argument("--target_language", required=True)
	parser.add_argument("--output_path", required=True)
	parser.add_argument("--top_k", type=int, default=15)
	parser.add_argument("--top_p", type=float, default=1.0)
	parser.add_argument("--temperature", type=float, default=1.0)
	parser.add_argument("--speed_factor", type=float, default=1.0)
	parser.add_argument("--text_split_method", default="cut5")
	parser.add_argument("--batch_size", type=int, default=1)
	parser.add_argument("--split_bucket", action="store_true")
	parser.add_argument("--fragment_interval", type=float, default=0.3)
	parser.add_argument("--seed", type=int, default=-1)
	parser.add_argument("--parallel_infer", action="store_true")
	parser.add_argument("--device", default="cpu")
	return parser.parse_args(argv)


def ensure_cpufast_root_on_path() -> Path:
	cpufast_root = Path.cwd().resolve()
	cpufast_root_str = str(cpufast_root)
	gpt_sovits_root = cpufast_root / "GPT_SoVITS"
	gpt_sovits_root_str = str(gpt_sovits_root)

	if cpufast_root_str not in sys.path:
		sys.path.insert(0, cpufast_root_str)
	if gpt_sovits_root.exists() and gpt_sovits_root_str not in sys.path:
		sys.path.insert(0, gpt_sovits_root_str)

	return cpufast_root


def read_text_file(path: str) -> str:
	return Path(path).expanduser().resolve().read_text(encoding="utf-8").strip()


def build_tts_pipeline(args: argparse.Namespace):
	tts_module = importlib.import_module("GPT_SoVITS.TTS_infer_pack.TTS")
	TTS = getattr(tts_module, "TTS")
	TTS_Config = getattr(tts_module, "TTS_Config")

	tts_config = TTS_Config(None)
	tts_config.device = args.device
	tts_config.is_half = not args.device.strip().lower().startswith("cpu")
	tts_config.update_version("v2")
	tts_config.t2s_weights_path = str(Path(args.gpt_model).expanduser().resolve())
	tts_config.vits_weights_path = str(Path(args.sovits_model).expanduser().resolve())
	return TTS(tts_config)


def synthesize(args: argparse.Namespace) -> None:
	import soundfile as sf

	output_path = Path(args.output_path).expanduser().resolve()
	output_path.parent.mkdir(parents=True, exist_ok=True)

	prompt_text = read_text_file(args.ref_text)
	target_text = read_text_file(args.target_text)
	if not prompt_text:
		raise ValueError("Reference text cannot be empty.")
	if not target_text:
		raise ValueError("Target text cannot be empty.")

	tts_pipeline = build_tts_pipeline(args)
	inputs = {
		"text": target_text,
		"text_lang": args.target_language.lower(),
		"ref_audio_path": str(Path(args.ref_audio).expanduser().resolve()),
		"aux_ref_audio_paths": [],
		"prompt_text": prompt_text,
		"prompt_lang": args.prompt_language.lower(),
		"top_k": int(args.top_k),
		"top_p": float(args.top_p),
		"temperature": float(args.temperature),
		"text_split_method": str(args.text_split_method),
		"batch_size": int(args.batch_size),
		"batch_threshold": 0.75,
		"split_bucket": bool(args.split_bucket),
		"speed_factor": float(args.speed_factor),
		"fragment_interval": float(args.fragment_interval),
		"seed": int(args.seed),
		"media_type": "wav",
		"streaming_mode": False,
		"parallel_infer": bool(args.parallel_infer),
		"vits_parallel_infer": bool(args.parallel_infer),
		"repetition_penalty": 1.35,
	}

	result = None
	for result in tts_pipeline.run(inputs):
		pass

	if result is None:
		raise RuntimeError("GPT-SoVITS-CPUFast inference returned no audio data")

	sample_rate, audio_data = result
	sf.write(output_path, audio_data, sample_rate)


def main(argv: list[str] | None = None) -> None:
	ensure_cpufast_root_on_path()
	args = parse_args(argv)
	synthesize(args)


if __name__ == "__main__":
	main()
