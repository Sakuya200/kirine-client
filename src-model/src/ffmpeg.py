import argparse
from pathlib import Path

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
    return parser.parse_args(argv)


def resolve_output_options(audio_format: str) -> str:
    normalized = audio_format.strip().lower()
    if normalized == "mp3":
        return "-codec:a libmp3lame"
    if normalized == "flac":
        return ""
    if normalized == "wav":
        return "-acodec pcm_s16le -ar 24000 -ac 1"

    raise ValueError(f"Unsupported transcode format: {audio_format}")


def transcode_audio(args: argparse.Namespace) -> None:
    input_path = Path(args.input_path).expanduser().resolve()
    if not input_path.exists():
        raise FileNotFoundError(f"Transcode input path does not exist: {input_path}")

    output_path = Path(args.output_path).expanduser().resolve()
    output_path.parent.mkdir(parents=True, exist_ok=True)

    output_options = resolve_output_options(args.format)
    ffmpeg = FFmpeg(
        executable="ffmpeg",
        global_options=["-y"],
        inputs={str(input_path): None},
        outputs={str(output_path): output_options or None},
    )
    ffmpeg.run()


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    transcode_audio(args)


if __name__ == "__main__":
    main()