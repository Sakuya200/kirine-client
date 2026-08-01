#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH='' && cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH='' && cd -- "$SCRIPT_DIR/../.." && pwd)
. "$SCRIPT_DIR/common.sh"

BASE_MODEL=""
SCRIPT_PATH=""
PARAMS_FILE=""
LOG_PATH=""
TASK_LOG_FILE=""

# Split forwarded args (everything after a literal `--`) from this wrapper's own arguments, so
# they can be passed straight through to the target Python script. Mirrors begin_llm_task.ps1.
forwarded_script_args=""
separator_seen=0
while [ "$#" -gt 0 ]; do
    arg=$1
    if [ "$separator_seen" -eq 1 ]; then
        if [ -n "$forwarded_script_args" ]; then
            forwarded_script_args="$forwarded_script_args $arg"
        else
            forwarded_script_args=$arg
        fi
        shift
        continue
    fi
    case "$arg" in
        --)
            separator_seen=1
            shift
            ;;
        --base-model)
            BASE_MODEL=$2
            shift 2
            ;;
        --script-path)
            SCRIPT_PATH=$2
            shift 2
            ;;
        --params-file)
            PARAMS_FILE=$2
            shift 2
            ;;
        --log-path)
            LOG_PATH=$2
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        *)
            echo "Unknown begin-llm-task argument: $arg" >&2
            exit 64
            ;;
    esac
done

if [ -z "$BASE_MODEL" ]; then
    echo "begin-llm-task requires --base-model." >&2
    exit 64
fi
if [ -z "$SCRIPT_PATH" ]; then
    echo "begin-llm-task requires --script-path." >&2
    exit 64
fi
if [ -z "$TASK_LOG_FILE" ]; then
    echo "begin-llm-task requires --task-log-file." >&2
    exit 64
fi

task_log_dir=$(dirname "$TASK_LOG_FILE")
mkdir -p "$task_log_dir"

append_log() {
    printf '%s\n' "$1" >>"$TASK_LOG_FILE"
}

run_checked() {
    description=$1
    shift
    append_log "[begin-llm-task] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

MODEL_ROOT="$SRC_MODEL_ROOT/$BASE_MODEL"

# Use whichever Python environment already exists in the model directory (conda_env or venv). This
# keeps an older venv-based install working even after conda is later installed, instead of routing
# execution through a conda_env that was never set up for this model.
resolve_model_python_environment "$MODEL_ROOT"
VENV_DIR="$PY_ENV_DIR"
VENV_PYTHON="$PY_PYTHON"

append_log "[begin-llm-task] Starting LLM task with base model '$BASE_MODEL'."

if [ ! -f "$SCRIPT_PATH" ]; then
    append_log "[begin-llm-task] Execute Error: Script not found: $SCRIPT_PATH"
    exit 1
fi

if [ ! -x "$VENV_PYTHON" ]; then
    append_log "[begin-llm-task] Execute Error: Python executable not found: $VENV_PYTHON"
    exit 1
fi

append_log "[begin-llm-task] executing script: $SCRIPT_PATH"

# NOTE: --log-path / --task-log-file are intentionally NOT forwarded to the target Python script.
# They are consumed by this wrapper for its own task-log output (append_log / run_checked above).
# The params-file entry scripts (voice_clone/tts/training/voice_design across all models)
# only accept --params-file and would fail with argparse "unrecognized arguments" if these were
# passed through.  Scripts that genuinely need them (e.g. download.py) receive them via the
# forwarded args after `--`.
#
# Build the Python argument list on the positional parameters so paths containing spaces survive
# intact (mirrors the array-based assembly in begin_llm_task.ps1).
set -- -X utf8 -X faulthandler -u "$SCRIPT_PATH"
if [ -n "$PARAMS_FILE" ]; then
    set -- "$@" --params-file "$PARAMS_FILE"
fi
if [ -n "$forwarded_script_args" ]; then
    # shellcheck disable=SC2086 # forwarded args are a pre-split token list from `--`
    set -- "$@" $forwarded_script_args
fi

if run_checked "Running Python command" "$VENV_PYTHON" "$@"; then
    append_log "[begin-llm-task] Completed LLM task successfully."
    exit 0
fi

append_log "[begin-llm-task] Execute Error: Python command exited non-zero."
exit 1
