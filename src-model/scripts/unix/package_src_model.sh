#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH='' && cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH='' && cd -- "$SCRIPT_DIR/../.." && pwd)
OUTPUT_FILE=${1:-"$SRC_MODEL_ROOT/../src-tauri/resources/src-model-runtime.tar"}

SOURCE_DIRECTORIES="scripts configs"
MODEL_DIRECTORIES="qwen3_tts vox_cpm2 moss_tts_local gpt_sovits_cpufast"

mkdir -p "$(dirname "$OUTPUT_FILE")"

if ! command -v tar >/dev/null 2>&1; then
    echo "tar is required to package src-model runtime files." >&2
    exit 65
fi

for directory_name in $SOURCE_DIRECTORIES $MODEL_DIRECTORIES; do
    if [ ! -d "$SRC_MODEL_ROOT/$directory_name" ]; then
        echo "Source directory not found: $SRC_MODEL_ROOT/$directory_name" >&2
        exit 66
    fi
done

FILE_LIST=$(mktemp)
cleanup() {
    rm -f "$FILE_LIST"
}
trap cleanup EXIT INT TERM

cd "$SRC_MODEL_ROOT"

for directory_name in $SOURCE_DIRECTORIES $MODEL_DIRECTORIES; do
    find "$directory_name" \
        \( -type d \( \
            -name base-models -o \
            -name tests -o \
            -name __pycache__ -o \
            -name .pytest_cache -o \
            -name .mypy_cache -o \
            -name .ruff_cache -o \
            -name venv -o \
            -name .venv \
        \) -prune \) -o \
        \( -type f ! -name '*.pyc' ! -name '*.pyo' -print \) >> "$FILE_LIST"
done

LC_ALL=C sort -u "$FILE_LIST" -o "$FILE_LIST"

rm -f "$OUTPUT_FILE"
tar -cf "$OUTPUT_FILE" -T "$FILE_LIST"

echo "Created model runtime archive: $OUTPUT_FILE"