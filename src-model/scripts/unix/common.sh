#!/usr/bin/env sh
# Shared helpers for the Unix model scripts. Mirrors scripts/windows/common.ps1.
#
# This file is intended to be sourced (`. common.sh`), not executed directly. It must stay
# POSIX-sh compatible and safe under `set -eu`: every conda probe is guarded so a missing conda
# CLI never aborts the caller.

# Print the conda executable path if the conda CLI is on PATH, otherwise return non-zero.
# Mirrors Get-CondaExecutable in common.ps1 — we deliberately only probe PATH and do NOT search
# common install locations.
get_conda_executable() {
    if command -v conda >/dev/null 2>&1; then
        command -v conda
        return 0
    fi
    return 1
}

# Print the prefix-based conda env path for a model directory: $MODEL_ROOT/conda_env.
# Mirrors Get-CondaEnvPath in common.ps1 — the conda env lives parallel to venv/.
get_conda_env_path() {
    printf '%s/conda_env\n' "$1"
}

# Echo the first available Python bootstrap command (python3 preferred, then python).
# Mirrors Get-BootstrapPythonCommand in common.ps1.
detect_python() {
    if command -v python3 >/dev/null 2>&1; then
        printf '%s\n' "python3"
        return 0
    fi
    if command -v python >/dev/null 2>&1; then
        printf '%s\n' "python"
        return 0
    fi
    return 1
}

# Resolve which Python environment a model directory should use. Mirrors
# Resolve-ModelPythonEnvironment in common.ps1.
#
# We always prefer an environment that already exists on disk, so a model that was set up with a
# plain venv keeps using that venv even after the user later installs conda. Migrating an existing
# working environment just because conda became available on PATH previously caused init-task-runtime
# to create a fresh (often incomplete) conda_env and break.
#
# Selection order:
#   1. conda_env/bin/python present  -> existing conda env
#   2. venv/bin/python present        -> existing venv
#   3. neither present + conda on PATH -> conda (caller is responsible for creation)
#   4. neither present + no conda      -> venv (caller is responsible for creation)
#
# Sets the globals: PY_BACKEND (conda|venv), PY_ENV_DIR, PY_PYTHON, PY_EXISTS (1|0).
resolve_model_python_environment() {
    model_root=$1
    venv_dir="$model_root/venv"
    venv_python="$venv_dir/bin/python"
    conda_env_path=$(get_conda_env_path "$model_root")
    conda_env_python="$conda_env_path/bin/python"

    if [ -x "$conda_env_python" ]; then
        PY_BACKEND=conda
        PY_ENV_DIR=$conda_env_path
        PY_PYTHON=$conda_env_python
        PY_EXISTS=1
        return 0
    fi

    if [ -x "$venv_python" ]; then
        PY_BACKEND=venv
        PY_ENV_DIR=$venv_dir
        PY_PYTHON=$venv_python
        PY_EXISTS=1
        return 0
    fi

    # Neither environment exists yet — pick the one to create. Prefer conda when its CLI is
    # available; otherwise fall back to a plain venv.
    if get_conda_executable >/dev/null 2>&1; then
        PY_BACKEND=conda
        PY_ENV_DIR=$conda_env_path
        PY_PYTHON=$conda_env_python
        PY_EXISTS=0
        return 0
    fi

    PY_BACKEND=venv
    PY_ENV_DIR=$venv_dir
    PY_PYTHON=$venv_python
    PY_EXISTS=0
    return 0
}
