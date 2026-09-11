---
name: child-process-path-handling
description: 子进程 PATH 注入禁止用 std::env::join_paths（Windows 引号语义损坏 PATH）；脚本端外部工具解析优先绝对路径
metadata: 
  node_type: memory
  type: project
  originSessionId: 5221a70a-ccfb-4955-94e5-381f3412dc7e
  modified: 2026-09-10T15:37:11.955Z
---

状态截至 2026-09-10 · 分支 `v0.12.2`

Windows 下 `std::env::join_paths` 会给含 `;` 的组件包双引号；继承的完整 PATH 必然含 `;`，注入后子进程按 `;` 朴素切分不剥引号，导致 PATH 首尾条目变成 `"C:\Windows\system32` 这类带引号非法路径。曾在更新 71ad291 后引发两次线上事故：子脚本里 cmd.exe 解析失败（后经绝对路径修复）、`ensure_torch_runtime.ps1` 的 `Get-CudaVersion` 报「No usable NVIDIA GPU」（nvidia-smi 位于 System32 首条目）。

**Why:** PATH 环境变量值从不做引号解析，`join_paths` 的引号是为命令行场景设计的，对 PATH 是错误语义。

**How to apply:**
- Rust 端拼接 PATH 一律用 `src-tauri/src/utils/process.rs` 的 `join_path_entries`（纯 `;` 拼接，有回归测试 `tests/process_logging.rs`），不要用 `join_paths`。
- PowerShell 脚本端解析系统工具（cmd.exe、nvidia-smi、nvcc 等）优先绝对路径：`[Environment]::SystemDirectory`、`$env:CUDA_PATH\bin` 等可预测位置，PATH `Get-Command` 只作兜底（见 `ensure_torch_runtime.ps1` 的 `Get-NvidiaToolAbsolutePath`、`begin_llm_task.ps1` 的 cmd.exe 解析）。
- 相关：[[ffmpeg-bundling]]（bundled PATH 前缀的来源）。
