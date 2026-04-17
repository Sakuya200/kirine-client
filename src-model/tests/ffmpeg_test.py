import importlib.util
import sys
import tempfile
import types
import unittest
import wave
from pathlib import Path
from unittest.mock import patch


SRC_MODEL_ROOT = Path(__file__).resolve().parents[1]


def load_ffmpeg_module(relative_path: str, module_name: str):
    module_path = SRC_MODEL_ROOT / relative_path
    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Unable to load module from {module_path}")

    module = importlib.util.module_from_spec(spec)
    fake_ffmpy = types.SimpleNamespace(FFmpeg=object)
    previous_ffmpy = sys.modules.get("ffmpy")
    sys.modules["ffmpy"] = fake_ffmpy
    try:
        spec.loader.exec_module(module)
    finally:
        if previous_ffmpy is None:
            sys.modules.pop("ffmpy", None)
        else:
            sys.modules["ffmpy"] = previous_ffmpy
    return module


def write_mono_wav(path: Path, sample_rate: int) -> None:
    with wave.open(str(path), "wb") as wav_file:
        wav_file.setnchannels(1)
        wav_file.setsampwidth(2)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(b"\x00\x00" * sample_rate)


class FfmpegHelperTests(unittest.TestCase):
    def test_parse_args_accepts_sample_rate_and_input_format(self):
        for relative_path, module_name in [
            ("qwen3_tts/ffmpeg.py", "qwen3_tts_ffmpeg"),
            ("vox_cpm2/ffmpeg.py", "vox_cpm2_ffmpeg"),
        ]:
            with self.subTest(module=module_name):
                module = load_ffmpeg_module(relative_path, module_name)
                args = module.parse_args(
                    [
                        "--input-path",
                        "input.wav",
                        "--output-path",
                        "output.wav",
                        "--format",
                        "wav",
                        "--input-format",
                        "wav",
                        "--sample-rate",
                        "12000",
                    ]
                )

                self.assertEqual(args.input_format, "wav")
                self.assertEqual(args.sample_rate, 12000)

    def test_resolve_output_options_appends_sample_rate(self):
        module = load_ffmpeg_module("qwen3_tts/ffmpeg.py", "qwen3_tts_ffmpeg_options")
        options = module.resolve_output_options("wav", 16000)

        self.assertIn("-acodec pcm_s16le", options)
        self.assertIn("-ac 1", options)
        self.assertIn("-ar 16000", options)

    def test_transcode_audio_copies_matching_wav_without_invoking_ffmpeg(self):
        for relative_path, module_name, sample_rate in [
            ("qwen3_tts/ffmpeg.py", "qwen3_tts_ffmpeg_copy", 12000),
            ("vox_cpm2/ffmpeg.py", "vox_cpm2_ffmpeg_copy", 16000),
        ]:
            with self.subTest(module=module_name):
                module = load_ffmpeg_module(relative_path, module_name)

                with tempfile.TemporaryDirectory() as temp_dir:
                    temp_root = Path(temp_dir)
                    input_path = temp_root / "input.wav"
                    output_path = temp_root / "output.wav"
                    write_mono_wav(input_path, sample_rate)

                    args = module.parse_args(
                        [
                            "--input-path",
                            str(input_path),
                            "--output-path",
                            str(output_path),
                            "--format",
                            "wav",
                            "--sample-rate",
                            str(sample_rate),
                            "--input-format",
                            "wav",
                        ]
                    )

                    with patch.object(module, "FFmpeg") as ffmpeg_class:
                        module.transcode_audio(args)

                    ffmpeg_class.assert_not_called()
                    self.assertTrue(output_path.exists())
                    self.assertEqual(input_path.read_bytes(), output_path.read_bytes())


if __name__ == "__main__":
    unittest.main()