#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH='' cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH='' cd -- "$SCRIPT_DIR/../.." && pwd)
BASE_MODEL=""
MODEL_ROOT=""
VENV_PYTHON=""
MODE=""
TASK_LOG_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base-model)
            BASE_MODEL=$2
            shift 2
            ;;
        --mode)
            MODE=$2
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        *)
            echo "Unknown toggle-lora-dependencies argument: $1" >&2
            exit 64
            ;;
    esac
done

if [ -z "$BASE_MODEL" ]; then
    echo "toggle-lora-dependencies requires --base-model." >&2
    exit 64
fi

MODEL_ROOT="$SRC_MODEL_ROOT/$BASE_MODEL"
VENV_PYTHON="$MODEL_ROOT/venv/bin/python"

if [ -z "$TASK_LOG_FILE" ]; then
    echo "toggle-lora-dependencies requires --task-log-file." >&2
    exit 64
fi

case "$MODE" in
    enable|disable)
        ;;
    *)
        echo "Unsupported --mode value: $MODE" >&2
        exit 64
        ;;
esac

task_log_dir=$(dirname "$TASK_LOG_FILE")
mkdir -p "$task_log_dir"
: >"$TASK_LOG_FILE"

append_log() {
    printf '%s\n' "$1" >>"$TASK_LOG_FILE"
}

run_checked() {
    description=$1
    shift
    append_log "[toggle-lora-dependencies] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

if [ ! -x "$VENV_PYTHON" ]; then
    echo "LoRA dependency toggle requires an initialized Python environment at $VENV_PYTHON" >&2
    exit 65
fi

append_log "[toggle-lora-dependencies] mode=$MODE"

if [ "$MODE" = "enable" ]; then
    run_checked "install peft" "$VENV_PYTHON" -m pip install --upgrade peft
    append_log "[toggle-lora-dependencies] LoRA dependencies are enabled"
    exit 0
fi

run_checked "uninstall peft" "$VENV_PYTHON" -m pip uninstall -y peft
append_log "[toggle-lora-dependencies] LoRA dependencies are disabled"