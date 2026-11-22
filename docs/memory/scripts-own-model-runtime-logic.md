---
name: scripts-own-model-runtime-logic
description: 模型初始化/调用/硬件探测逻辑归脚本层所有，不得为性能用 Rust 层缓存或直连代替脚本判断
metadata:
  node_type: memory
  type: feedback
  originSessionId: acf18693-44d2-40c6-b842-17a79f6c62ba
  modified: 2026-11-06T16:23:43.054Z
---

用户明确要求（2026-11-07，第 13 批评审时）：脚本层存在的意义就是把具体模型的初始化、调用、硬件探测等逻辑与 Rust 层分离。不得为性能牺牲该层级边界。

**Why:** 第 12 批曾用 Rust 侧 `device-probe.json` 缓存代替 ensure 脚本判断硬件类型（命中即绕过脚本），第 13 批计划消除 powershell 包装直连 ffmpeg/python——用户否决了整条路线：秒级的脚本调用延迟完全可以忽略，层级边界优先于性能。

**How to apply:**
- 硬件类型/设备探测：始终同步走 `ensure_torch_runtime.ps1 --query-device-type`，Rust 只解析输出，不做缓存代替判断
- 子进程链保持 `powershell → 脚本 → 实际程序` 结构，不做直连消除
- 训练规范化并发化（原第 14 批）可以并发 spawn 脚本，但每个子进程仍走完整脚本链
- 性能优化应聚焦其他维度（事件推送、IO、前端渲染），不动脚本边界

相关：[[dont-borrow-feature-config]]、[[child-process-path-handling]]
