#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
VENV_DIR="$SRC_MODEL_ROOT/venv"
VENV_PYTHON="$VENV_DIR/bin/python"
TASK_LOG_FILE=""
MODEL_ID_LIST_JSON=""
MODEL_NAME_LIST_JSON=""
TARGET_ROOT_DIR=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --model-id-list)
            MODEL_ID_LIST_JSON=$2
            shift 2
            ;;
        --model-name-list)
            MODEL_NAME_LIST_JSON=$2
            shift 2
            ;;
        --target-root-dir)
            TARGET_ROOT_DIR=$2
            shift 2
            ;;
        --log-path)
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        *)
            echo "Unknown download-models argument: $1" >&2
            exit 64
            ;;
    esac
done

ensure_task_log_file() {
    if [ -z "$TASK_LOG_FILE" ]; then
        echo "download-models requires --task-log-file." >&2
        exit 64
    fi

    task_log_dir=$(dirname "$TASK_LOG_FILE")
    mkdir -p "$task_log_dir"
}

TAB=$(printf '\t')
append_log() {
    printf '%s\n' "$1" >>"$TASK_LOG_FILE"
}

run_checked() {
    description=$1
    shift
    append_log "[download-models] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

resolve_modelscope_command() {
    if [ -x "$VENV_DIR/bin/modelscope" ]; then
        printf '%s\n' "$VENV_DIR/bin/modelscope"
        return 0
    fi

    if [ -x "$VENV_PYTHON" ]; then
        printf '%s\n' "$VENV_PYTHON -m modelscope.cli.cli"
        return 0
    fi

    return 1
}

download_model() {
    model_id=$1
    model_name=$2
    target_root_dir=$3
    modelscope_command=$4
    target_dir="$target_root_dir/$model_name"

    if [ -e "$target_dir" ]; then
        append_log "[download-models] model already cached at $target_dir"
        return 0
    fi

    set -- $modelscope_command
    run_checked "download $model_id" "$@" download --model "$model_id" --local_dir "$target_dir"
}

ensure_task_log_file

for required_value in "$MODEL_ID_LIST_JSON" "$MODEL_NAME_LIST_JSON" "$TARGET_ROOT_DIR"; do
    if [ -z "$required_value" ]; then
        echo "download-models requires --model-id-list, --model-name-list and --target-root-dir." >&2
        exit 64
    fi
done

if [ ! -x "$VENV_PYTHON" ]; then
    echo "[download-models] Python virtual environment is missing at $VENV_PYTHON. Run init-task-runtime first." >&2
    exit 65
fi

mkdir -p "$TARGET_ROOT_DIR"

modelscope_command=$(resolve_modelscope_command) || {
    echo "[download-models] ModelScope CLI is unavailable. Run init-task-runtime first." >&2
    exit 65
}

parsed_lists=$(
    "$VENV_PYTHON" -c 'import json, sys
model_ids = json.loads(sys.argv[1])
model_names = json.loads(sys.argv[2])
if not isinstance(model_ids, list) or not isinstance(model_names, list):
    raise SystemExit("download-models requires JSON arrays for model ids and model names.")
if not model_ids:
    raise SystemExit("download-models requires at least one model id.")
if len(model_ids) != len(model_names):
    raise SystemExit("download-models requires model ids and model names to have the same length.")
for model_id, model_name in zip(model_ids, model_names):
    if not str(model_id).strip() or not str(model_name).strip():
        raise SystemExit("download-models does not allow empty model ids or model names.")
    print(f"{model_id}\t{model_name}")' "$MODEL_ID_LIST_JSON" "$MODEL_NAME_LIST_JSON"
) || {
    echo "$parsed_lists" >&2
    exit 64
}

printf '%s\n' "$parsed_lists" | while IFS="$TAB" read -r model_id model_name; do
    download_model "$model_id" "$model_name" "$TARGET_ROOT_DIR" "$modelscope_command"
done

append_log "[download-models] required models are ready"