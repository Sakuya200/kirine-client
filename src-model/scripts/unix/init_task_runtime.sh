#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/../.." && pwd)
BASE_MODEL=""
MODEL_ROOT=""
REQUIREMENTS_FILE=""
VENV_DIR=""
VENV_PYTHON=""
CPU_MODE=0
TASK_LOG_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base-model)
            BASE_MODEL=$2
            shift 2
            ;;
        --requirements-file)
            REQUIREMENTS_FILE=$2
            shift 2
            ;;
        --log-path)
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        --cpu-mode)
            CPU_MODE=1
            shift
            ;;
        *)
            echo "Unknown init-task-runtime argument: $1" >&2
            exit 64
            ;;
    esac
done

if [ -z "$BASE_MODEL" ]; then
    echo "init-task-runtime requires --base-model." >&2
    exit 64
fi

MODEL_ROOT="$SRC_MODEL_ROOT/$BASE_MODEL"
if [ -z "$REQUIREMENTS_FILE" ]; then
    REQUIREMENTS_FILE="$MODEL_ROOT/requirements.txt"
fi
VENV_DIR="$MODEL_ROOT/venv"
VENV_PYTHON="$VENV_DIR/bin/python"

ensure_task_log_file() {
    if [ -z "$TASK_LOG_FILE" ]; then
        echo "init-task-runtime requires --task-log-file." >&2
        exit 64
    fi

    task_log_dir=$(dirname "$TASK_LOG_FILE")
    mkdir -p "$task_log_dir"
}

append_log() {
    printf '%s\n' "$1" >>"$TASK_LOG_FILE"
}

run_checked() {
    description=$1
    shift
    append_log "[init-task-runtime] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

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

ensure_env() {
    if [ -x "$VENV_PYTHON" ]; then
        return 0
    fi

    PYTHON_CMD=$(detect_python) || {
        echo "[init-task-runtime] Python environment is unavailable. Tried python3 and python but neither succeeded." >&2
        return 65
    }

    append_log "[init-task-runtime] creating Python virtual environment at $VENV_DIR"
    run_checked "create Python virtual environment" "$PYTHON_CMD" -m venv "$VENV_DIR"

    if [ ! -x "$VENV_PYTHON" ]; then
        echo "[init-task-runtime] Python virtual environment was not created at $VENV_PYTHON." >&2
        return 65
    fi
}

parse_cuda_version() {
    printf '%s\n' "$1" | sed -n -e 's/.*CUDA Version: \([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1 \2/p' -e 's/.*release \([0-9][0-9]*\)\.\([0-9][0-9]*\).*/\1 \2/p' | head -n 1
}

detect_cuda_version() {
    if command -v nvidia-smi >/dev/null 2>&1; then
        output=$(nvidia-smi 2>&1 || true)
        version=$(parse_cuda_version "$output")
        if [ -n "$version" ]; then
            append_log "[init-task-runtime] detected CUDA $(printf '%s' "$version" | tr ' ' '.') via nvidia-smi"
            printf '%s\n' "$version"
            return 0
        fi
    fi

    if command -v nvcc >/dev/null 2>&1; then
        output=$(nvcc --version 2>&1 || true)
        version=$(parse_cuda_version "$output")
        if [ -n "$version" ]; then
            append_log "[init-task-runtime] detected CUDA $(printf '%s' "$version" | tr ' ' '.') via nvcc"
            printf '%s\n' "$version"
            return 0
        fi
    fi

    echo "[init-task-runtime] No usable NVIDIA GPU or CUDA toolkit was detected. init-task-runtime requires nvidia-smi or nvcc to report a supported CUDA version." >&2
    return 1
}

version_at_least() {
    current_major=$1
    current_minor=$2
    supported_major=$3
    supported_minor=$4

    if [ "$current_major" -gt "$supported_major" ]; then
        return 0
    fi
    if [ "$current_major" -lt "$supported_major" ]; then
        return 1
    fi
    if [ "$current_minor" -ge "$supported_minor" ]; then
        return 0
    fi

    return 1
}

select_cuda_tag() {
    current_major=$1
    current_minor=$2

    matched_tags=''

    for supported in '13 0 130' '12 8 128' '12 6 126' '12 4 124' '12 1 121' '11 8 118'; do
        set -- $supported
        supported_major=$1
        supported_minor=$2
        supported_tag=$3
        if version_at_least "$current_major" "$current_minor" "$supported_major" "$supported_minor"; then
            if [ -n "$matched_tags" ]; then
                matched_tags="$matched_tags $supported_tag"
            else
                matched_tags=$supported_tag
            fi
        fi
    done

    if [ -n "$matched_tags" ]; then
        append_log "[init-task-runtime] candidate PyTorch CUDA tags for detected CUDA $current_major.$current_minor: $(printf '%s' "$matched_tags" | sed 's/ /, cu/g; s/^/cu/')"
        printf '%s\n' "$matched_tags"
        return 0
    fi

    echo "[init-task-runtime] Detected CUDA version $current_major.$current_minor is lower than the minimum supported stable PyTorch tag 11.8." >&2
    return 1
}

verify_torch_cuda_runtime() {
    run_checked "$1" "$VENV_PYTHON" -c "import torch, torchaudio, torchvision; assert torch.cuda.is_available(), 'torch.cuda.is_available() returned False'; assert torch.version.cuda, 'torch.version.cuda is empty'; print(torch.__version__); print(torch.version.cuda)"
}

verify_torch_cpu_runtime() {
    run_checked "$1" "$VENV_PYTHON" -c "import torch, torchaudio, torchvision; print(torch.__version__)"
}

get_torch_runtime_metadata() {
    if [ ! -x "$VENV_PYTHON" ]; then
        return 1
    fi

    append_log "[init-task-runtime] inspect installed torch runtime metadata"
    output=$(
        "$VENV_PYTHON" -c "import torch, torchaudio, torchvision; print(f'{torch.__version__}|{torch.version.cuda or ''''}|{int(bool(torch.cuda.is_available()))}')" \
            2>>"$TASK_LOG_FILE"
    ) || return 1
    printf '%s\n' "$output" >>"$TASK_LOG_FILE"
    printf '%s\n' "$output"
}

torch_cuda_version_to_tag() {
    version=$1
    printf '%s\n' "$version" | sed -n 's/^\([0-9][0-9]*\)\.\([0-9][0-9]*\)$/\1\2/p'
}

torch_cpu_initialized() {
    metadata=$(get_torch_runtime_metadata) || return 1
    old_ifs=$IFS
    IFS='|'
    set -- $metadata
    IFS=$old_ifs
    torch_version=$1
    torch_cuda=${2-}

    if [ -n "$torch_cuda" ]; then
        append_log "[init-task-runtime] existing torch runtime uses CUDA $torch_cuda, which does not match current CPU mode"
        return 1
    fi

    append_log "[init-task-runtime] detected compatible CPU torch runtime version $torch_version; offline initialization check passed"
    return 0
}

torch_cuda_initialized() {
    candidates=$1
    metadata=$(get_torch_runtime_metadata) || return 1
    old_ifs=$IFS
    IFS='|'
    set -- $metadata
    IFS=$old_ifs
    torch_version=$1
    torch_cuda=${2-}
    cuda_available=${3-0}

    if [ "$cuda_available" != '1' ]; then
        append_log '[init-task-runtime] installed torch runtime reports cuda_available=False; runtime does not match current GPU mode'
        return 1
    fi

    installed_tag=$(torch_cuda_version_to_tag "$torch_cuda")
    if [ -z "$installed_tag" ]; then
        append_log '[init-task-runtime] installed torch runtime does not expose a usable CUDA version; runtime does not match current GPU mode'
        return 1
    fi

    for candidate_tag in $candidates; do
        if [ "$candidate_tag" = "$installed_tag" ]; then
            append_log "[init-task-runtime] detected compatible CUDA torch runtime cu$installed_tag (torch $torch_version); offline initialization check passed"
            return 0
        fi
    done

    append_log "[init-task-runtime] installed torch CUDA tag cu$installed_tag is incompatible with the current GPU environment; expected one of: $candidates"
    return 1
}

ensure_base_dependencies() {
    run_checked "install project requirements" "$VENV_PYTHON" -m pip install -r "$REQUIREMENTS_FILE"
    append_log "[init-task-runtime] base Python dependencies are ready"
}

install_compatible_torch_cuda() {
    candidates=$1
    failed_tags=''

    for candidate_tag in $candidates; do
        if run_checked "install torch wheels for cu$candidate_tag" "$VENV_PYTHON" -m pip install --no-cache-dir torch torchvision torchaudio --index-url "https://download.pytorch.org/whl/cu$candidate_tag" \
            && verify_torch_cuda_runtime "verify torch CUDA runtime using cu$candidate_tag"; then
            append_log "[init-task-runtime] verified working PyTorch CUDA runtime using cu$candidate_tag"
            printf '%s\n' "$candidate_tag"
            return 0
        fi

        if [ -n "$failed_tags" ]; then
            failed_tags="$failed_tags, cu$candidate_tag"
        else
            failed_tags="cu$candidate_tag"
        fi
        append_log "[init-task-runtime] cu$candidate_tag verification failed, trying next compatible tag"
    done

    echo "[init-task-runtime] Unable to initialize a working PyTorch CUDA runtime for the detected CUDA environment. Tried $failed_tags. Re-run with --cpu-mode if you want a CPU-only environment." >&2
    return 1
}

ensure_task_log_file

if [ ! -f "$REQUIREMENTS_FILE" ]; then
    echo "[init-task-runtime] Requirements file not found: $REQUIREMENTS_FILE" >&2
    exit 64
fi

ensure_env

if [ "$CPU_MODE" -eq 1 ]; then
    append_log "[init-task-runtime] CPU mode enabled; skipping CUDA detection"
    if torch_cpu_initialized; then
        append_log "[init-task-runtime] local task runtime already matches current CPU mode; skipping dependency installation"
        append_log "[init-task-runtime] local task runtime is ready"
        exit 0
    fi

    ensure_base_dependencies
    if verify_torch_cpu_runtime "verify existing torch runtime"; then
        append_log "[init-task-runtime] existing torch runtime is already usable after base dependency sync; skipping torch reinstall"
    else
        run_checked "install torch CPU wheels" "$VENV_PYTHON" -m pip install --no-cache-dir torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cpu
    fi
else
    cuda_version=$(detect_cuda_version)
    set -- $cuda_version
    cuda_major=$1
    cuda_minor=$2
    cuda_candidates=$(select_cuda_tag "$cuda_major" "$cuda_minor")
    if torch_cuda_initialized "$cuda_candidates"; then
        append_log "[init-task-runtime] local task runtime already matches current GPU mode; skipping dependency installation"
        append_log "[init-task-runtime] local task runtime is ready"
        exit 0
    fi

    ensure_base_dependencies
    if verify_torch_cuda_runtime "verify existing torch CUDA runtime"; then
        append_log "[init-task-runtime] existing torch CUDA runtime is already usable after base dependency sync; skipping torch reinstall"
    else
        install_compatible_torch_cuda "$cuda_candidates" >/dev/null
    fi
fi

if [ "$CPU_MODE" -eq 1 ]; then
    verify_torch_cpu_runtime "verify torch CPU runtime after dependency install"
else
    verify_torch_cuda_runtime "verify torch CUDA runtime after dependency install"
fi

append_log "[init-task-runtime] local task runtime is ready"