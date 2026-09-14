# Kirine Client User Manual

Language / 语言: [English](README.en-US.md) | [中文](README.md)

Kirine Client is the desktop client of the Kirine (桐音) audio workbench. It supports local text-to-speech, voice cloning, voice design, model training, streaming speech sessions, speaker management, and history task management.

The goal of this project is to provide a general-purpose UI and scheduling layer for audio synthesis models, delivering out-of-the-box multi-model audio synthesis. More audio synthesis features will be introduced in the future. Intro video (Bilibili): https://www.bilibili.com/video/BV1MwLy6yEzU

---

## 1. Prerequisites

Before using Kirine Client for the first time, install the following programs and make sure they are on the system PATH:

1. **Python 3.12.x**: used for the local model runtime initialization and task execution. Version 3.12.x is recommended.
2. **Git**: used by the resource acquisition flow of some models, especially GPT-SoVITS-CPUFast.
3. **Conda (optional)**: Miniconda or Anaconda. If a `conda` command is detected on the system PATH, the app prefers creating a dedicated conda environment (`<model dir>/conda_env`) per model; otherwise it falls back to a standard `venv`. All features work fine without Conda.

## 2. Before You Start

1. Currently the app runs in Windows 10/11 local mode.
2. The first model install, first inference, or first training run is usually slower; this is normal.
3. Each task can run on CPU or CUDA (GPU) individually. If the selected device differs from the model's current runtime environment, a confirmation dialog appears before submission. After confirmation the app switches automatically, but this task's startup time will increase noticeably.
4. GPU is recommended for model training; CPU works but is significantly slower.
5. Currently only **Local mode** (local SQLite + local Python runtime) is available. Remote mode is still under development and not yet wired to real HTTP calls.

## 3. Currently Supported Models

| Base Model | Version | Text to Speech | Voice Clone | Model Training | Voice Design | Streaming Speech | Device Support |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Irodori-TTS-V3 | 500M | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| Dots.TTS | 2B | ✓ | ✓ | ✓ | — | — | CPU / CUDA |
| Qwen3-TTS | 1.7B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| Qwen3-TTS | 0.6B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| VoxCPM2 | 2B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| MOSS-TTS Local | 1.7B | ✓ | ✓ | ✓ | — | — | CPU / CUDA |
| MOSS-TTS Realtime | 1.7B | — | — | — | — | ✓ | CPU / CUDA |
| GPT-SoVITS-CPUFast | V1 / V2 / V2Pro / V2ProPlus | ✓ | ✓ | — | — | — | CPU |

## 4. Quick Start

1. After startup, open the **Settings** page first and confirm the data, log, and model directories match your machine's directory plan.
2. Open the **Model Management** page, pick the current device for each model (single-device models auto-fill), then install the models you need.
3. **Text to Speech**: pick model, speaker, and language, enter text, then submit.
4. **Voice Clone**: upload a reference audio, enter the target lines, then submit; reference audio supports `wav`, `mp3`, `flac`, `ogg`.
5. **Voice Design**: enter a voice prompt and the target lines to generate speech in the target style.
6. **Model Training**: import samples first (single sample or batch dataset), then fill in the speaker name and parameters and start training.
7. **Streaming Speech**: enter the streaming speech page, create a session, then send messages continuously in a chat style and receive audio chunk playback in real time; speakers can come from reference-audio cloning or be picked from already-trained speakers.

## 5. Feature Pages

### 5.1 Model Management

View currently supported models and their install status; install, reinstall, or uninstall models. After a model is installed, the corresponding features are automatically enabled on task pages.

### 5.2 Text to Speech

Pick model, version, device type, language, and output format, optionally pick a speaker, enter text, then submit the task. Results appear live in the result card on the right side of the page and are also recorded in history tasks.

### 5.3 Voice Clone

Pick model, version, device type, language, and output format, upload a reference audio, optionally fill in the reference text, enter the target lines, then submit.

### 5.4 Model Training

Pick language, model, and device type, fill in the speaker name and description, import training data via "Single Sample (audio + lines)" or "Dataset (audio archive + annotation file)", then start.

Annotation file formats: `jsonl`, `xlsx`, `xls`.

### 5.5 Speaker Management

View, search, edit, and delete local speakers, filter by status, and import speakers from the local model directory.

### 5.6 History Tasks

View the status, details, and results of all tasks (including streaming speech sessions) in one place; preview and export audio, and re-launch tasks by backfilling parameters from a history record.

### 5.7 Streaming Speech

Streaming speech uses a session-level long-running process: one session can carry multiple consecutive messages, with audio streams dispatched by `contextId`. The page supports two kinds of input: reference-audio voice cloning and already-trained speakers (picked from Ready speakers). History sessions restore messages and configuration; preview and export of a single message locate the generated audio via the history task ID and message ID.

## 6. Supported Model Project Links

1. Irodori-TTS-V3
   - Project (GitHub): https://github.com/Aratako/Irodori-TTS
   - Model repo (Hugging Face): https://huggingface.co/Aratako
2. Dots.TTS
   - Project (GitHub): https://github.com/rednote-hilab/dots.tts
   - Model repo (Hugging Face): https://huggingface.co/rednote-hilab
3. Qwen3-TTS
   - Project (GitHub): https://github.com/QwenLM/Qwen3-TTS
   - Model repo (Hugging Face): https://huggingface.co/Qwen
4. VoxCPM2
   - Model repo (Hugging Face): https://huggingface.co/openbmb/VoxCPM2
5. MOSS-TTS Local
   - Model repo (Hugging Face): https://huggingface.co/OpenMOSS-Team
6. MOSS-TTS Realtime
   - Project (GitHub): https://github.com/OpenMOSS/MOSS-TTS
   - Model repo (Hugging Face): https://huggingface.co/OpenMOSS-Team
7. GPT-SoVITS-CPUFast
   - Project repo (GitHub): https://github.com/baicai-1145/GPT-SoVITS-CPUFast

## 7. FAQ

**Slow first run**: The first call of a model automatically creates the Python virtual environment and installs dependencies; this is normal and later calls will not repeat it.

**Task shows no progress for a long time**: Check the notification bar for error toasts, or inspect the task's log files under the configured `log_dir`. Slower startup is normal after a first install, first inference, or a device-type switch.

**flash-attn related errors**: Windows lacks stable official support; keep the default `sdpa`.

**Errors after switching to Remote mode**: The current version has not wired Remote mode to real API calls yet; keep using Local mode.

---

## Developer Documentation

This section is for developers who need to build or debug Kirine Client locally.

### D1. Development Environment Requirements

In addition to the user-side Python 3.12.x and Git above, also install:

1. **Node.js**: LTS recommended, for frontend builds.
2. **Rust toolchain**: install via rustup; Tauri 2 requires the stable toolchain.
3. **Tauri dependencies**: Windows needs Microsoft C++ Build Tools and WebView2, see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

### D2. Prepare Bundled Resources (required)

Tauri validates that every file listed in `bundle.resources` exists under `src-tauri/resources/` both in local builds and dev mode. **If any of the following files are missing, `npm run tauri dev` and `npm run tauri build` fail with an error.**

Files to prepare manually:

```
src-tauri/resources/
├── config.toml                          # can be copied from the repository
├── ffmpeg-8.1.2.zip                      # ffmpeg Windows build package (shared build)
├── sox-14.4.2-win32.zip                 # SoX Windows build package
└── src-model-runtime.zip                # Python model runtime package
```

Acquisition notes:

- `ffmpeg-8.1.2.zip`: ffmpeg **shared build** (with avcodec etc. DLLs, loaded by torchcodec/torchaudio). Download the full shared build from https://www.gyan.dev/ffmpeg/builds/ and repack so the top-level directory is `ffmpeg-8.1.2/` (or keep the original name; the install hook normalizes it). The version number must match the filename.
- `sox-14.4.2-win32.zip`: download the matching Windows package from https://sourceforge.net/projects/sox/files/sox/.
- `src-model-runtime.zip`: a package of the repository's `src-model/` directory. During development the app auto-detects `src-model/` under the workspace root (the cloned repo directory), so a missing zip does not affect runtime features locally, but Tauri checks file existence before startup/build. You can create an empty zip as a placeholder, or get the official runtime package from the maintainer.

### D3. Clone and Initialize

```bash
git clone --recurse-submodules <repo_url>
cd kirine-client
npm install
```

If you already did a plain `git clone`, run submodule init separately:

```bash
git submodule update --init --recursive
```

When a model adapter subproject updates, run in the main repo:

```bash
git submodule update --remote --recursive
```

When done, prepare the files under `src-tauri/resources/` as described in D2.

### D4. Start the Dev Environment

```bash
npm run tauri dev
```

This starts the Vite dev server and the Tauri desktop app together; frontend hot reload applies in real time, and Rust changes recompile and restart the app automatically.

### D5. Build

```bash
npm run tauri build
```

It runs `vue-tsc --noEmit` type checking first, then builds the frontend via Vite, and finally packages with Tauri into an NSIS installer. Output goes to `src-tauri/target/release/bundle/`.

### D5.1 Build the Portable Package

Besides the NSIS installer, a portable zip (unzip and run; no registry writes, no install scripts on the user machine — mitigates antivirus false positives on "files released after install") is also available:

```bash
powershell -ExecutionPolicy Bypass -File src-tauri/scripts/make-portable.ps1
```

Default flow runs `npm run tauri build` first, then packs; for iterative packing add `-SkipBuild` to reuse existing artifacts. Output: `src-tauri/target/portable/kirine-client-<version>-portable-x64.zip`.

The package layout matches the NSIS install-hook target layout (exe + config.toml in the root; `lib\sox-14-4-2`, `lib\ffmpeg-8.1.2`, `lib\src-model`); at runtime the Rust side injects an app-local PATH prefix for child processes (`bundled_tool_path_prefix` in `src-tauri/src/utils/process.rs`), so the portable package does not need system PATH writes. Note: the script contains Chinese characters and must be saved as UTF-8 with BOM (Windows PowerShell 5.1 parses BOM-less files as ANSI and errors on mojibake).

### D6. Compile-Only Checks

If you only need to verify the code compiles without a full run:

```bash
# Rust side
cd src-tauri
cargo check

# Frontend type check
npx vue-tsc --noEmit
```

### D7. Configuration File

`config.toml` at the project root is the runtime config file; the app looks for it in the current directory at startup.

```toml
[basic]
mode = "local"
data_dir = 'D:\Project\temp\kirine-client\data'
log_dir = 'D:\Project\temp\kirine-client\logs'
model_dir = 'D:\Project\temp\kirine-client\models'

[training]
attn_implementation = "sdpa"
```

- `data_dir`: storage for task data, samples, SQLite database, and generated audio.
- `log_dir`: app logs and task logs; check here first when debugging.
- `model_dir`: storage for local model weights.
- `training.attn_implementation`: attention implementation; one of `sdpa` (default), `flash_attention_2`, `eager`.

### D8. Logs and Debugging

- **Rust backend logs**: written to log files under `log_dir`; task execution logs are stored separately per task ID.
- **Frontend / Tauri logs**: printed to the terminal in dev mode; in production they are written to `log_dir` as well.
- **Python script logs**: on each task execution, script output is redirected to files named with a task-type prefix under `log_dir/task/` (e.g. `tts-<id>.log`, `voice-clone-<id>.log`, `voice-design-<id>.log`, `training-<id>.log`, `streaming-<id>.log`).
- **Model runtime lookup**: at startup Rust tries `<workspace>/src-model`, `<app_dir>/src-model`, `<app_dir>/lib/src-model` in order; local development usually hits the first path (`src-model/` under the repo directory).

### D9. Model Adapter Development

To add or maintain model adapters in `src-model/` (including the structure of the two config files `model-config.json` / `params-config.json`, the form component types supported by the frontend, and the implementation flow and call chain of the Python adapters), read the **[Model Adapter Development Guide](src-model/ADAPTER_DEVELOPMENT.md)**.

