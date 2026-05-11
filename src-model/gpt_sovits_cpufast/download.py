import argparse
import json
from pathlib import Path
import subprocess
import shutil
import sys
import time
import urllib.error
import urllib.request


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-model", dest="base_model", type=str, required=True)
    parser.add_argument("--model-scale", dest="model_scale", type=str, required=True)
    parser.add_argument("--target-root-dir", dest="target_root_dir", type=str, required=True)
    parser.add_argument("--log-path", dest="log_path", type=str, required=False)
    parser.add_argument("--task-log-file", dest="task_log_file", type=str, required=False)
    parser.add_argument(
        "--repo-url",
        dest="repo_url",
        type=str,
        default="https://github.com/baicai-1145/GPT-SoVITS-CPUFast",
    )
    parser.add_argument(
        "--asset-source",
        dest="asset_source",
        choices=["HF", "HF-Mirror", "ModelScope"],
        default="ModelScope",
        help="Where to download pretrained assets from (matches upstream install.ps1).",
    )
    parser.add_argument(
        "--asset-version",
        dest="asset_version",
        choices=["v1", "v2", "v2Pro", "v2ProPlus", "all"],
        required=False,
        help="Pretrained asset version to download (matches upstream install.ps1).",
    )
    return parser.parse_args(argv)


ASSET_URL_PREFIX = {
    "HF": "https://huggingface.co/XXXXRT/GPT-SoVITS-Pretrained/resolve/main",
    "HF-Mirror": "https://hf-mirror.com/XXXXRT/GPT-SoVITS-Pretrained/resolve/main",
    "ModelScope": "https://www.modelscope.cn/models/XXXXRT/GPT-SoVITS-Pretrained/resolve/master",
}

SHARED_INFERENCE_FILES = [
    "pretrained_models/chinese-hubert-base/config.json",
    "pretrained_models/chinese-hubert-base/preprocessor_config.json",
    "pretrained_models/chinese-hubert-base/pytorch_model.bin",
    "pretrained_models/chinese-roberta-wwm-ext-large/config.json",
    "pretrained_models/chinese-roberta-wwm-ext-large/pytorch_model.bin",
    "pretrained_models/chinese-roberta-wwm-ext-large/tokenizer.json",
    "pretrained_models/fast_langdetect/lid.176.bin",
    "pretrained_models/fast_langdetect/lid.176.ftz",
]

VERSION_INFERENCE_FILES = {
    "v1": [
        "pretrained_models/s1bert25hz-2kh-longer-epoch=68e-step=50232.ckpt",
        "pretrained_models/s2G488k.pth",
    ],
    "v2": [
        "pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
        "pretrained_models/gsv-v2final-pretrained/s2G2333k.pth",
    ],
    "v2Pro": [
        "pretrained_models/s1v3.ckpt",
        "pretrained_models/sv/pretrained_eres2netv2w24s4ep4.ckpt",
        "pretrained_models/v2Pro/s2Gv2Pro.pth",
    ],
    "v2ProPlus": [
        "pretrained_models/s1v3.ckpt",
        "pretrained_models/sv/pretrained_eres2netv2w24s4ep4.ckpt",
        "pretrained_models/v2Pro/s2Gv2ProPlus.pth",
    ],
}

MODEL_SCALE_TO_ASSET_VERSION = {
    "v1": "v1",
    "1": "v1",
    "v2": "v2",
    "2": "v2",
    "v2pro": "v2Pro",
    "pro": "v2Pro",
    "v2proplus": "v2ProPlus",
    "v2pp": "v2ProPlus",
    "plus": "v2ProPlus",
}


def _has_gpt_sovits_structure(root: Path) -> bool:
    """Check if GPT-SoVITS directory structure exists."""
    return (root / "GPT_SoVITS" / "inference_webui_fast.py").exists()


def _get_missing_checkpoints(root: Path, version: str = "v2ProPlus") -> list[str]:
    """Get list of missing checkpoint files."""
    checkpoint_paths_by_version = {
        "v1": [
            "GPT_SoVITS/pretrained_models/s1bert25hz-2kh-longer-epoch=68e-step=50232.ckpt",
            "GPT_SoVITS/pretrained_models/s2G488k.pth",
        ],
        "v2": [
            "GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s1bert25hz-5kh-longer-epoch=12-step=369668.ckpt",
            "GPT_SoVITS/pretrained_models/gsv-v2final-pretrained/s2G2333k.pth",
        ],
        "v2Pro": [
            "GPT_SoVITS/pretrained_models/s1v3.ckpt",
            "GPT_SoVITS/pretrained_models/v2Pro/s2Gv2Pro.pth",
        ],
        "v2ProPlus": [
            "GPT_SoVITS/pretrained_models/s1v3.ckpt",
            "GPT_SoVITS/pretrained_models/v2Pro/s2Gv2ProPlus.pth",
        ],
    }

    if version == "all":
        checkpoint_paths = []
        for key in ("v1", "v2", "v2Pro", "v2ProPlus"):
            checkpoint_paths.extend(checkpoint_paths_by_version[key])
        checkpoint_paths = list(dict.fromkeys(checkpoint_paths))
    else:
        checkpoint_paths = checkpoint_paths_by_version[version]

    missing = []
    for checkpoint_path in checkpoint_paths:
        if not (root / checkpoint_path).exists():
            missing.append(checkpoint_path)
    return missing


def _resolve_asset_version(asset_version: str | None, model_scale: str) -> str:
    if asset_version:
        return asset_version

    normalized_scale = model_scale.strip().lower()
    mapped = MODEL_SCALE_TO_ASSET_VERSION.get(normalized_scale)
    if not mapped:
        raise SystemExit(
            "❌ Unable to infer asset version from --model-scale. "
            f"Got: {model_scale}. Supported scales include: V1, V2, V2Pro, V2ProPlus (or aliases like v2pp)."
        )
    return mapped


def _resolve_version_files(version: str) -> list[str]:
    if version == "all":
        merged = []
        for key in ("v1", "v2", "v2Pro", "v2ProPlus"):
            merged.extend(VERSION_INFERENCE_FILES[key])
        return merged
    return VERSION_INFERENCE_FILES[version]


def _required_asset_files(version: str) -> list[str]:
    merged = SHARED_INFERENCE_FILES + _resolve_version_files(version)
    # Keep deterministic order and remove duplicates.
    return list(dict.fromkeys(merged))


def _missing_asset_files(root: Path, version: str) -> list[str]:
    missing = []
    for rel_path in _required_asset_files(version):
        if not (root / "GPT_SoVITS" / rel_path).exists():
            missing.append(rel_path)
    return missing


def _is_cpufast_runtime_ready(root: Path, version: str = "v2ProPlus") -> bool:
    """Check if GPT-SoVITS-CPUFast runtime is complete and ready to use."""
    # Must have the core directory structure
    if not _has_gpt_sovits_structure(root):
        return False
    
    # Check if all required checkpoints exist
    missing = _get_missing_checkpoints(root, version)
    return len(missing) == 0


def _clone_repo(repo_url: str, destination: Path) -> Path:
    """Clone a git repository with retry logic and timeout handling."""
    git_bin = shutil.which("git")
    if git_bin is None:
        raise SystemExit(
            "Custom download requires git for automatic clone. "
            "Install git and make sure it is available in PATH."
        )

    max_retries = 3
    retry_delay = 5  # seconds
    timeout = 300  # seconds (5 minutes)
    
    for attempt in range(1, max_retries + 1):
        try:
            subprocess.run(
                [git_bin, "clone", "--depth", "1", repo_url, str(destination)],
                check=True,
                timeout=timeout,
            )
            return destination
        except subprocess.TimeoutExpired:
            error_msg = f"Git clone timed out after {timeout}s (attempt {attempt}/{max_retries})"
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                time.sleep(retry_delay)
                retry_delay *= 2  # exponential backoff
            else:
                raise SystemExit(
                    f"❌ Git clone failed: {error_msg}.\n"
                    f"Please check your network connection and try again.\n"
                    f"Alternatively, you can:\n"
                    f"  1. Use a proxy: git config --global http.proxy <proxy_url>\n"
                    f"  2. Retry later if GitHub is unavailable"
                )
        except subprocess.CalledProcessError as e:
            error_msg = f"Git clone failed with exit code {e.returncode} (attempt {attempt}/{max_retries})"
            if attempt < max_retries:
                print(f"⚠️  {error_msg}. Retrying in {retry_delay}s...", file=sys.stderr)
                # Clean up partial clone if any
                if destination.exists():
                    shutil.rmtree(destination)
                time.sleep(retry_delay)
                retry_delay *= 2  # exponential backoff
            else:
                raise SystemExit(
                    f"❌ {error_msg}.\n"
                    f"Error details: {e}\n"
                    f"Common causes:\n"
                    f"  - Network connectivity issues (RPC failed, connection reset)\n"
                    f"  - GitHub is temporarily unavailable\n"
                    f"  - Firewall/proxy blocking connection\n"
                    f"  - SSH key issues (if using SSH)\n"
                    f"\n"
                    f"Solutions:\n"
                    f"  1. Check your network connection\n"
                    f"  2. Try again in a few moments\n"
                    f"  3. Use a proxy if behind firewall: git config --global http.proxy <proxy_url>\n"
                    f"  4. Retry later if GitHub service is unstable"
                )


def _download_file(url: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url, timeout=120) as response:
        with destination.open("wb") as file_obj:
            shutil.copyfileobj(response, file_obj)


def _download_missing_assets(cpufast_root: Path, source: str, version: str) -> None:
    missing = _missing_asset_files(cpufast_root, version)
    if not missing:
        print("✓ Inference assets already complete; skip asset download")
        return

    prefix = ASSET_URL_PREFIX[source].rstrip("/")
    print(f"📦 Missing {len(missing)} inference asset(s), downloading from {source}...")

    failures: dict[str, str] = {}
    for rel_path in missing:
        url = f"{prefix}/{rel_path}"
        local_path = cpufast_root / "GPT_SoVITS" / rel_path
        success = False
        delay = 3
        for attempt in range(1, 4):
            try:
                _download_file(url, local_path)
                print(f"   ✓ {rel_path}")
                success = True
                break
            except (urllib.error.URLError, TimeoutError, OSError) as exc:
                if attempt < 3:
                    print(
                        f"   ⚠️ {rel_path} download failed (attempt {attempt}/3): {exc}. "
                        f"Retrying in {delay}s...",
                        file=sys.stderr,
                    )
                    time.sleep(delay)
                    delay *= 2
                else:
                    failures[rel_path] = str(exc)

        if not success and local_path.exists():
            # Avoid leaving corrupted partial files.
            local_path.unlink(missing_ok=True)

    if failures:
        failure_summary = {
            "source": source,
            "version": version,
            "failed": failures,
        }
        raise SystemExit(
            "❌ Failed to download required GPT-SoVITS-CPUFast assets.\n"
            "Details:\n"
            f"{json.dumps(failure_summary, ensure_ascii=False, indent=2)}\n"
            "Hint: you can switch source via --asset-source HF|HF-Mirror|ModelScope"
        )

    print("✓ Inference assets download completed")


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    target_root = Path(args.target_root_dir).expanduser().resolve()
    target_dir = target_root / args.base_model
    resolved_asset_version = _resolve_asset_version(args.asset_version, args.model_scale)
    target_root.mkdir(parents=True, exist_ok=True)

    # Check 1: Already fully ready?
    if _is_cpufast_runtime_ready(target_dir, resolved_asset_version):
        print(f"✓ GPT-SoVITS-CPUFast is already complete at {target_dir} for {resolved_asset_version}")
        return

    # Check 2: Ensure code checkout exists.
    if not _has_gpt_sovits_structure(target_dir):
        if target_dir.exists():
            print(f"⚠️  Existing target directory is not a valid GPT-SoVITS-CPUFast checkout: {target_dir}")
            print("🧹 Removing invalid directory for a clean direct clone...")
            shutil.rmtree(target_dir)

        print(f"📥 Cloning GPT-SoVITS-CPUFast directly into target directory: {target_dir}")
        _clone_repo(args.repo_url, target_dir)
        print("✓ Clone completed successfully")
    else:
        print(f"✓ GPT-SoVITS-CPUFast checkout already exists at {target_dir}; skip clone")

    # Check 3: Ensure official inference assets are present.
    _download_missing_assets(target_dir, args.asset_source, resolved_asset_version)

    if not _is_cpufast_runtime_ready(target_dir, resolved_asset_version):
        missing = _get_missing_checkpoints(target_dir, resolved_asset_version)
        raise SystemExit(
            f"❌ Setup failed after direct clone: gpt_sovits_cpufast runtime is incomplete.\n"
            f"Target version: {resolved_asset_version}\n"
            f"Missing files ({len(missing)}):\n" +
            "\n".join(f"  - {target_dir / m}" for m in missing) +
            f"\n\n"
            f"Troubleshooting:\n"
            f"  1. Ensure you have sufficient disk space\n"
            f"  2. Check your network connection\n"
            f"  3. Manually download from: {args.repo_url}\n"
            f"  4. Try a different asset source via --asset-source HF-Mirror\n"
            f"  5. Verify selected model-scale/asset-version points to available checkpoints"
        )
    
    print(f"✅ GPT-SoVITS-CPUFast is ready at {target_dir}")


if __name__ == "__main__":
    main(sys.argv[1:])
