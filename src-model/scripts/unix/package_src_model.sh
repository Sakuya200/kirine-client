#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
OUTPUT_FILE=${1:-"$SRC_MODEL_ROOT/../src-tauri/resources/src-model-runtime.zip"}

mkdir -p "$(dirname "$OUTPUT_FILE")"

if command -v python3 >/dev/null 2>&1; then
    PYTHON_CMD=python3
elif command -v python >/dev/null 2>&1; then
    PYTHON_CMD=python
else
    echo "Python is required to package src-model runtime files." >&2
    exit 65
fi

SRC_MODEL_ROOT="$SRC_MODEL_ROOT" OUTPUT_FILE="$OUTPUT_FILE" "$PYTHON_CMD" - <<'PY'
import os
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile

src_model_root = Path(os.environ["SRC_MODEL_ROOT"]).resolve()
output_file = Path(os.environ["OUTPUT_FILE"]).resolve()

exclude_directory_names = {"__pycache__", ".pytest_cache", ".mypy_cache", ".ruff_cache"}
exclude_suffixes = {".pyc", ".pyo"}
source_directories = ("scripts", "src")
source_files = ("requirements.txt",)

output_file.parent.mkdir(parents=True, exist_ok=True)
if output_file.exists():
    output_file.unlink()

with ZipFile(output_file, "w", compression=ZIP_DEFLATED) as archive:
    for directory_name in source_directories:
        directory_path = src_model_root / directory_name
        if not directory_path.exists():
            raise SystemExit(f"Source directory not found: {directory_path}")

        for file_path in directory_path.rglob("*"):
            if not file_path.is_file():
                continue

            relative_path = file_path.relative_to(src_model_root)
            if any(part in exclude_directory_names for part in relative_path.parts):
                continue
            if file_path.suffix.lower() in exclude_suffixes:
                continue

            archive.write(file_path, relative_path.as_posix())

    for file_name in source_files:
        file_path = src_model_root / file_name
        if not file_path.exists():
            raise SystemExit(f"Source file not found: {file_path}")

        archive.write(file_path, file_path.relative_to(src_model_root).as_posix())

print(f"Created model runtime archive: {output_file}")
PY