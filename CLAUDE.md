# Kirine Client

Tauri 2 + Vue 3 + Rust 桌面 AI 语音合成客户端。

## 上下文记忆目录

Claude 上下文记忆统一存放于 **`docs/memory/`**（纳入 git 管理）。

- 索引：`docs/memory/MEMORY.md`（每次会话自动注入）
- 各主题记忆（架构、数据流、规则等）见索引链接
- 记忆只记**当前状态**，不记版本更新/功能优化的变更过程；每条记忆顶部带「状态截至 日期 · 分支」锚点

### 当前项目上下文快照（2026-09-06）

- 本仓库是 Tauri 2 + Vue 3 + Rust 的桌面端 AI 语音合成客户端，当前分支为 `v0.12.2`，打包版本为 `0.12.2`。
- 重点能力已覆盖 TTS / Voice Clone / Voice Design / Model Training / Streaming Speech；其中 Streaming Speech 在 Local 模式下已实现会话级长期进程、环回 Socket 二进制帧协议实时音频流，以及说话人头像与消息侧别展示。
- 模型体系由 `src-model/` 下 7 个独立 adapter 子模块驱动，`moss_tts_realtime` 是首个实现会话级 `streaming.py` 契约的适配器。
- 远程存储模式仍处于占位阶段，当前实际可用路径仍是 Local 模式；后续改动请优先参考 `docs/memory/` 里的记忆文档。

### 自动加载与目录联接（junction）

Claude Code 的记忆自动加载路径按项目路径编码写死在用户主目录：
`C:\Users\<user>\.claude\projects\d--Project-llm-kirine-client\memory\`，
不由 CLAUDE.md 控制。为让该路径指向仓库内的 `docs/memory/`，本机用 Windows 目录联接（junction）桥接——旧路径的读/写都透明落到 `docs/memory/`，自动加载与后续记忆写入因此都指向仓库。

建联接（cmd，**不需要管理员权限**，跨盘可用；路径按本机调整）：

```bat
mklink /J "${USER_HOME}\.claude\projects\d--Project-llm-kirine-client\memory" "~\docs\memory"
```

或在 PowerShell：

```powershell
New-Item -ItemType Junction -Path "${USER_HOME}\.claude\projects\d--Project-llm-kirine-client\memory" -Target "~\docs\memory"
```

> **换机器或重新克隆后需重建此 junction**，否则自动加载会找不到 `MEMORY.md`。

### 本地-only 目录

`docs/superpowers/`（superpowers 框架的 plans/specs）已加入 `.gitignore`，不纳入版本控制，仅本地保留。
