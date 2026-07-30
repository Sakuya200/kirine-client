# Kirine Client 项目记忆索引

> 状态截至 2026-07-30 · 分支 `v.0.12.0`。记忆只记当前状态，不记版本更新/功能优化的变更过程。

## 当前项目上下文快照
- 当前仓库是基于 Tauri 2 + Vue 3 + Rust 的桌面端 AI 语音合成客户端，重点能力已覆盖 TTS / Voice Clone / Voice Design / Model Training / Streaming Speech。
- Local 模式下流式语音会话已落地：会话级长期进程、`contextId` 多路分发、帧缓冲文件协议与实时音频回传均已就绪；Remote 模式仍处于占位，当前不可用。
- 模型体系仍由 `src-model/` 下 7 个独立 adapter 子模块驱动，`moss_tts_realtime` 是首个实现会话级 `streaming.py` 契约的适配器。

- [Project Overview](project-overview.md) - 项目全貌、技术栈、核心功能、存储模式（Local/Remote）概览
- [Frontend Architecture](tech-stack-frontend.md) - Vue 3 前端目录结构、配置驱动参数表单、UI 组件体系
- [Backend Architecture](tech-stack-backend.md) - Tauri/Rust 后端模块、Hooks 层、配置系统、Pipeline、远程 client/RemoteService、流式语音会话层
- [Streaming Speech Architecture](streaming-speech-architecture.md) - 流式语音生成全栈架构：会话级长期进程 runner、stdout 帧协议、contextId 多路分发、清扫、远程不支持
- [Model Adapter Pattern](model-adapter-pattern.md) - 7 个模型适配器（含 irodori_tts_v3/moss_tts_realtime）、特性矩阵、参数文件执行、venv/conda_env 运行时、当前设备选择
- [Data Flow & Types](data-flow-and-types.md) - 前后端通信、完整命令列表、类型与枚举对应、UI 配置类型、远程模式与 AudioAsset
- [DB Schema Sync Rule](db-schema-sync-rule.md) - 表结构变更须同时更新 db/tables.sql 与 db/tables_pgsql.sql
- [HistoryTaskType Sync Rule](history-task-type-sync-rule.md) - 前端 HistoryTaskType 枚举须与 Rust 端逐一对齐，新增任务类型两侧同步
- [Don't Borrow Feature Config](dont-borrow-feature-config.md) - 特性未就绪时勿借用其它特性实现填充，保持代码路径独立
- [Time Field Naming Rule](time-field-naming-rule.md) - 实体时间字段一律 createTime/modifyTime，类型 string
- [Retain Future-Use Fields](retain-future-use-fields.md) - 勿为消警告删预留待用字段（如 ResolvedStreamingPaths 的 base_model/model_version）
- [src-tauri Not rustfmt-clean](src-tauri-not-rustfmt-clean.md) - src-tauri HEAD 非 rustfmt-1.8.0 干净；全局 cargo fmt 会改 30 个无关文件，勿用
- [Tests Dir Over Inline](tests-dir-over-inline.md) - 单测放 tests/ 经 test_support 桥接，不用内联 #[cfg(test)]；pub(crate) 项先提 pub 再重导出
- [FFmpeg Bundling](ffmpeg-bundling.md) - ffmpeg shared build 打包/解压（src-model 同级）/PATH 注销重注册 + moss_tts_realtime DLL 注册
