import argparse
from pathlib import Path
import shutil
import wave

from ffmpy import FFmpeg


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--input_path",
        "--input-path",
        dest="input_path",
        type=str,
        required=True,
    )
    parser.add_argument(
        "--output_path",
        "--output-path",
        dest="output_path",
        type=str,
        required=True,
    )
    parser.add_argument("--format", type=str, required=True)
    parser.add_argument("--input-format", dest="input_format", type=str, default="")
    parser.add_argument("--sample-rate", dest="sample_rate", type=int, default=None)
    return parser.parse_args(argv)


def normalize_audio_format(audio_format: str) -> str:
    normalized = audio_format.strip().lower()
    if normalized == "wave":
        return "wav"
    return normalized


def resolve_input_format(input_path: Path, input_format: str) -> str:
    normalized = normalize_audio_format(input_format)
    if normalized:
        return normalized
    return normalize_audio_format(input_path.suffix.lstrip("."))


def resolve_input_options(input_path: Path, input_format: str) -> str:
    normalized = resolve_input_format(input_path, input_format)
    if normalized in {"wav", "mp3", "flac", "ogg"}:
        return f"-f {normalized}"
    return ""


def resolve_output_options(audio_format: str, sample_rate: int | None = None) -> str:
    normalized = normalize_audio_format(audio_format)
    options = ["-vn", "-sn", "-dn"]
    if normalized == "mp3":
        options.append("-codec:a libmp3lame")
    elif normalized == "flac":
        options.append("-codec:a flac")
    elif normalized == "wav":
        options.extend(["-acodec pcm_s16le", "-ac 1"])
    else:
        raise ValueError(f"Unsupported transcode format: {audio_format}")

    if sample_rate is not None:
        if sample_rate <= 0:
            raise ValueError(f"Sample rate must be positive: {sample_rate}")
        options.append(f"-ar {sample_rate}")

    return " ".join(options)


def can_copy_wav_without_transcode(input_path: Path, sample_rate: int | None) -> bool:
    try:
        with wave.open(str(input_path), "rb") as wav_file:
            if wav_file.getnchannels() != 1:
                return False
            if wav_file.getsampwidth() != 2:
                return False
            if wav_file.getcomptype() != "NONE":
                return False
            if sample_rate is not None and wav_file.getframerate() != sample_rate:
                return False
            return True
    except (wave.Error, FileNotFoundError, OSError):
        return False


def should_copy_audio(
    input_path: Path,
    output_path: Path,
    audio_format: str,
    sample_rate: int | None,
) -> bool:
    normalized = normalize_audio_format(audio_format)
    input_format = normalize_audio_format(input_path.suffix.lstrip("."))
    if normalized == "wav":
        return input_format == "wav" and can_copy_wav_without_transcode(input_path, sample_rate)
    return sample_rate is None and input_format == normalized


def copy_audio(input_path: Path, output_path: Path) -> None:
    if input_path == output_path:
        return
    shutil.copy2(input_path, output_path)


def transcode_audio(args: argparse.Namespace) -> None:
    input_path = Path(args.input_path).expanduser().resolve()
    if not input_path.exists():
        raise FileNotFoundError(f"Transcode input path does not exist: {input_path}")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    if should_copy_audio(input_path, output_path, args.format, args.sample_rate):
        copy_audio(input_path, output_path)
        return

    input_options = resolve_input_options(input_path, args.input_format)
    output_options = resolve_output_options(args.format, args.sample_rate)
    ffmpeg = FFmpeg(
        executable="ffmpeg",
        global_options=["-y", "-nostdin"],
        inputs={str(input_path): input_options or None},
        outputs={str(output_path): output_options or None},
    )
    ffmpeg.run()


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    transcode_audio(args)


if __name__ == "__main__":
    main()