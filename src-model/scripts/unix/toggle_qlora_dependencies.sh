#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
VENV_PYTHON="$SRC_MODEL_ROOT/venv/bin/python"
MODE=""
TASK_LOG_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --mode)
            MODE=$2
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        *)
            echo "Unknown toggle-qlora-dependencies argument: $1" >&2
            exit 64
            ;;
    esac
done

if [ -z "$TASK_LOG_FILE" ]; then
    echo "toggle-qlora-dependencies requires --task-log-file." >&2
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
    append_log "[toggle-qlora-dependencies] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

if [ ! -x "$VENV_PYTHON" ]; then
    echo "QLoRA dependency toggle requires an initialized Python environment at $VENV_PYTHON" >&2
    exit 65
fi

append_log "[toggle-qlora-dependencies] mode=$MODE"

if [ "$MODE" = "enable" ]; then
    run_checked "install peft" "$VENV_PYTHON" -m pip install --upgrade peft
    run_checked "install bitsandbytes" "$VENV_PYTHON" -m pip install --upgrade bitsandbytes
    append_log "[toggle-qlora-dependencies] QLoRA dependencies are enabled"
    exit 0
fi

run_checked "uninstall bitsandbytes and peft" "$VENV_PYTHON" -m pip uninstall -y bitsandbytes peft
append_log "[toggle-qlora-dependencies] QLoRA dependencies are disabled"