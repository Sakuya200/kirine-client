---
name: ffmpeg-bundling
description: ffmpeg shared build 的打包/解压规则与 moss_tts_realtime DLL 注册（无系统 PATH 注册）
metadata:
  type: project
---

状态截至 2026-09-10 · 分支 `v0.12.2`

随应用分发的 ffmpeg 是 **shared build**（含 avcodec 等 DLL），供 torchcodec/torchaudio 加载。

- 资源：`src-tauri/resources/ffmpeg-8.1.2.zip`（zip 顶层目录 `ffmpeg-8.1.2-full_build-shared/`），在 `tauri.conf.json` 的 `bundle.resources` 中登记。
- NSIS 安装钩子 `src-tauri/windows/prepare-dependencies/ffmpeg.nsh` 的 `InstallBundledFfmpeg`：
  - 解压到 **src-model 同级目录** `$INSTDIR\lib\ffmpeg-8.1.2\`（与 `src-model\` 并列，非 src-model 内）；
  - 解压后把 zip 顶层 `ffmpeg-8.1.2-full_build-shared` **重命名**为 canonical `ffmpeg-8.1.2`；
  - **不探测系统 ffmpeg**；仅首次安装或 `lib\ffmpeg-8.1.2` 缺失时解压；
  - **不注册系统 PATH**（HKCU Environment\Path 写入/注销流程已整体移除）：脚本端按名调用由 Rust 运行时注入进程级 PATH 前缀解析（`src-tauri/src/utils/process.rs::bundled_tool_path_prefix`，见 [[portable-packaging]]）。
- moss_tts_realtime 推理前需 `os.add_dll_directory`：`src-model/moss_tts_realtime/common.py::ensure_ffmpeg_dlls()` 解析 `_SRC_MODEL_ROOT.parent/ffmpeg-8.1.2/bin`（即 src-model 同级），缺失即 `SystemExit` 报错；`streaming.py::run_session` 在 `import torch` 前调用。
- sox 仍在 `lib\sox-14-4-2`（与 ffmpeg 同级）：`sox.nsh` 已与 ffmpeg 行为对齐（不探测系统 sox、不注册 PATH，目录缺失即解压），并补上 zip 顶层 `sox-14.4.2` → `sox-14-4-2` 的重命名（原 hook 缺此步导致 sox.exe 校验必失败，因无运行时调用未暴露）。

相关：[[model-adapter-pattern]] [[portable-packaging]]
