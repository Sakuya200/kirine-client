#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH='' && cd -- "$(dirname "$0")" && pwd)
SRC_MODEL_ROOT=$(CDPATH='' && cd -- "$SCRIPT_DIR/../.." && pwd)
BASE_MODEL=""
CPU_MODE=0
TASK_LOG_FILE=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --base-model)
            BASE_MODEL=$2
            shift 2
            ;;
        --task-log-file)
            TASK_LOG_FILE=$2
            shift 2
            ;;
        --log-path)
            shift 2
            ;;
        --cpu-mode)
            CPU_MODE=1
            shift
            ;;
        *)
            echo "Unknown ensure-torch-runtime argument: $1" >&2
            exit 64
            ;;
    esac
done

if [ -z "$BASE_MODEL" ]; then
    echo "ensure-torch-runtime requires --base-model." >&2
    exit 64
fi

if [ -z "$TASK_LOG_FILE" ]; then
    echo "ensure-torch-runtime requires --task-log-file." >&2
    exit 64
fi

TASK_LOG_DIR=$(dirname "$TASK_LOG_FILE")
mkdir -p "$TASK_LOG_DIR"

append_log() {
    printf '%s\n' "$1" >>"$TASK_LOG_FILE"
}

run_checked() {
    description=$1
    shift
    append_log "[ensure-torch-runtime] $description: $*"
    "$@" >>"$TASK_LOG_FILE" 2>&1
}

MODEL_ROOT="$SRC_MODEL_ROOT/$BASE_MODEL"
VENV_PYTHON="$MODEL_ROOT/venv/bin/python"

if [ ! -x "$VENV_PYTHON" ]; then
    echo "[ensure-torch-runtime] Python virtual environment not found: $VENV_PYTHON. Please install model from model management first." >&2
    exit 1
fi

read_metadata() {
    "$VENV_PYTHON" -c "import torch; print('TORCH_METADATA|{}|{}|{}'.format(torch.__version__, torch.version.cuda or '', int(bool(torch.cuda.is_available()))))" 2>>"$TASK_LOG_FILE" | tail -n 1
}

if [ "$CPU_MODE" -eq 1 ]; then
    append_log "[ensure-torch-runtime] CPU mode enabled"
    metadata=$(read_metadata || true)
    case "$metadata" in
        TORCH_METADATA*'|'*'|'*)
            torch_cuda=$(printf '%s' "$metadata" | awk -F'|' '{print $3}')
            if [ -z "$torch_cuda" ]; then
                append_log "[ensure-torch-runtime] torch runtime already matches CPU mode"
                append_log "[ensure-torch-runtime] torch runtime is ready"
                exit 0
            fi
            ;;
    esac

    run_checked "install torch CPU wheels" "$VENV_PYTHON" -m pip install --force-reinstall --no-cache-dir torch==2.10.0 torchvision==0.18.0 torchaudio==2.10.0 --index-url https://download.pytorch.org/whl/cpu
    run_checked "verify torch CPU runtime" "$VENV_PYTHON" -c "import torch; assert not (torch.version.cuda or ''), 'CPU runtime expected no CUDA tag'; print(torch.__version__)"
    append_log "[ensure-torch-runtime] torch runtime is ready"
    exit 0
fi

append_log "[ensure-torch-runtime] CUDA mode enabled"
cuda_minor=""
if command -v nvidia-smi >/dev/null 2>&1; then
    cuda_minor=$(nvidia-smi 2>>"$TASK_LOG_FILE" | sed -n 's/.*CUDA Version: \([0-9][0-9]*\.[0-9][0-9]*\).*/\1/p' | head -n 1)
fi
if [ -z "$cuda_minor" ] && command -v nvcc >/dev/null 2>&1; then
    cuda_minor=$(nvcc --version 2>>"$TASK_LOG_FILE" | sed -n 's/.*release \([0-9][0-9]*\.[0-9][0-9]*\).*/\1/p' | head -n 1)
fi
if [ -z "$cuda_minor" ]; then
    echo "[ensure-torch-runtime] No usable NVIDIA GPU or CUDA toolkit was detected." >&2
    exit 1
fi

major=$(printf '%s' "$cuda_minor" | cut -d. -f1)
minor=$(printf '%s' "$cuda_minor" | cut -d. -f2)
candidates=""
if [ "$major" -ge 13 ]; then
    candidates="130 128 126 124 121 118"
elif [ "$major" -eq 12 ] && [ "$minor" -ge 8 ]; then
    candidates="128 126 124 121 118"
elif [ "$major" -eq 12 ] && [ "$minor" -ge 6 ]; then
    candidates="126 124 121 118"
elif [ "$major" -eq 12 ] && [ "$minor" -ge 4 ]; then
    candidates="124 121 118"
elif [ "$major" -eq 12 ] && [ "$minor" -ge 1 ]; then
    candidates="121 118"
elif [ "$major" -eq 11 ] && [ "$minor" -ge 8 ]; then
    candidates="118"
fi

if [ -z "$candidates" ]; then
    echo "[ensure-torch-runtime] Detected CUDA version $cuda_minor is lower than minimum supported 11.8." >&2
    exit 1
fi

metadata=$(read_metadata || true)
if [ -n "$metadata" ]; then
    installed_cuda=$(printf '%s' "$metadata" | awk -F'|' '{print $3}')
    cuda_available=$(printf '%s' "$metadata" | awk -F'|' '{print $4}')
    if [ "$cuda_available" = "1" ] && [ -n "$installed_cuda" ]; then
        installed_tag=$(printf '%s' "$installed_cuda" | tr -d '.')
        for tag in $candidates; do
            if [ "$installed_tag" = "$tag" ]; then
                append_log "[ensure-torch-runtime] torch runtime already matches CUDA mode (cu$tag)"
                append_log "[ensure-torch-runtime] torch runtime is ready"
                exit 0
            fi
        done
    fi
fi

for tag in $candidates; do
    if run_checked "install torch CUDA wheels (cu$tag)" "$VENV_PYTHON" -m pip install --force-reinstall --no-cache-dir torch==2.10.0 torchvision==0.18.0 torchaudio==2.10.0 --index-url "https://download.pytorch.org/whl/cu$tag"; then
        if run_checked "verify torch CUDA runtime (cu$tag)" "$VENV_PYTHON" -c "import torch; assert torch.cuda.is_available(), 'torch.cuda.is_available() is False'; assert torch.version.cuda, 'torch.version.cuda is empty'; print(torch.__version__); print(torch.version.cuda)"; then
            append_log "[ensure-torch-runtime] torch runtime is ready"
            exit 0
        fi
    fi
done

echo "[ensure-torch-runtime] Unable to initialize working CUDA torch runtime for detected CUDA $cuda_minor." >&2
exit 1
