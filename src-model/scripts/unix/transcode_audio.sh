#!/usr/bin/env sh
set -eu

INPUT_PATH=""
OUTPUT_PATH=""
OUTPUT_FORMAT=""
INPUT_FORMAT=""
SAMPLE_RATE=""
TASK_LOG_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --input-path)
            INPUT_PATH=$2
            shift 2
            ;;
        --output-path)
            OUTPUT_PATH=$2
            shift 2
            ;;
        --format)
            OUTPUT_FORMAT=$2
            shift 2
            ;;
        --input-format)
            INPUT_FORMAT=$2
            shift 2
            ;;
        --sample-rate)
            SAMPLE_RATE=$2
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        *)
            echo "Unknown transcode-audio argument: $1" >&2
            exit 64
            ;;
    esac
done

if [ -z "$TASK_LOG_FILE" ]; then
    echo "Missing --task-log-file argument." >&2
    exit 64
fi

mkdir -p "$(dirname "$TASK_LOG_FILE")"

if [ -z "$INPUT_PATH" ]; then
    echo "Missing --input-path argument." >&2
    exit 64
fi
if [ -z "$OUTPUT_PATH" ]; then
    echo "Missing --output-path argument." >&2
    exit 64
fi
if [ -z "$OUTPUT_FORMAT" ]; then
    echo "Missing --format argument." >&2
    exit 64
fi

if [ ! -f "$INPUT_PATH" ]; then
    echo "Transcode input path does not exist: $INPUT_PATH" >&2
    exit 66
fi

mkdir -p "$(dirname "$OUTPUT_PATH")"

normalized_format=$(printf '%s' "$OUTPUT_FORMAT" | tr '[:upper:]' '[:lower:]')
if [ "$normalized_format" = "wave" ]; then
    normalized_format="wav"
fi

set -- -y -nostdin
if [ -n "$INPUT_FORMAT" ]; then
    normalized_input_format=$(printf '%s' "$INPUT_FORMAT" | tr '[:upper:]' '[:lower:]')
    set -- "$@" -f "$normalized_input_format"
fi
set -- "$@" -i "$INPUT_PATH" -vn -sn -dn

case "$normalized_format" in
    mp3)
        set -- "$@" -codec:a libmp3lame
        ;;
    flac)
        set -- "$@" -codec:a flac
        ;;
    wav)
        set -- "$@" -acodec pcm_s16le -ac 1
        ;;
    *)
        echo "Unsupported transcode format: $OUTPUT_FORMAT" >&2
        exit 64
        ;;
esac

if [ -n "$SAMPLE_RATE" ]; then
    case "$SAMPLE_RATE" in
        ''|*[!0-9]*)
            echo "Sample rate must be positive: $SAMPLE_RATE" >&2
            exit 64
            ;;
        0)
            echo "Sample rate must be positive: $SAMPLE_RATE" >&2
            exit 64
            ;;
        *)
            set -- "$@" -ar "$SAMPLE_RATE"
            ;;
    esac
fi

set -- "$@" "$OUTPUT_PATH"
printf '%s\n' "[transcode-audio] ffmpeg $*" >>"$TASK_LOG_FILE"
ffmpeg "$@" >>"$TASK_LOG_FILE" 2>&1