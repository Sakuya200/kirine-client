---
name: ffmpeg-bundling
description: ffmpeg shared build 的打包/解压/PATH 与 moss_tts_realtime DLL 注册规则
metadata:
  type: project
---

状态截至 2026-07-28 · 分支 `v.0.12.0`

随应用分发的 ffmpeg 是 **shared build**（含 avcodec 等 DLL），供 torchcodec/torchaudio 加载。

- 资源：`src-tauri/resources/ffmpeg-8.1.2.zip`（zip 顶层目录 `ffmpeg-8.1.2-full_build-shared/`），在 `tauri.conf.json` 的 `bundle.resources` 中登记。
- NSIS 安装钩子 `src-tauri/windows/prepare-dependencies/ffmpeg.nsh` 的 `InstallBundledFfmpeg`：
  - 解压到 **src-model 同级目录** `$INSTDIR\lib\ffmpeg-8.1.2\`（与 `src-model\` 并列，非 src-model 内）；
  - 解压后把 zip 顶层 `ffmpeg-8.1.2-full_build-shared` **重命名**为 canonical `ffmpeg-8.1.2`；
  - **不探测系统 ffmpeg**；仅首次安装或 `lib\ffmpeg-8.1.2` 缺失时解压；
  - PATH 更新：先 `RemoveFromUserPath` 注销旧条目（旧版 8.0.1 残留 + 当前 bin），再 `AddToUserPathIfMissing` 注册 `$INSTDIR\lib\ffmpeg-8.1.2\bin`；`RemoveFromUserPath` 在 `common.nsh`，纯 NSIS，保留 REG_EXPAND_SZ。
- moss_tts_realtime 推理前需 `os.add_dll_directory`：`src-model/moss_tts_realtime/common.py::ensure_ffmpeg_dlls()` 解析 `_SRC_MODEL_ROOT.parent/ffmpeg-8.1.2/bin`（即 src-model 同级），缺失即 `SystemExit` 报错；`streaming.py::run_session` 在 `import torch` 前调用。
- sox 仍在 `lib\sox-14-4-2`（与 ffmpeg 同级），逻辑未变（仍探测系统 sox）。

相关：[[model-adapter-pattern]]
