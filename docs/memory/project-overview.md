---
name: project-overview
description: 项目全貌、技术栈、核心功能、存储模式概览
metadata: 
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# Kirine Client - 项目全貌

> 状态截至 2026-07-27 · 分支 `v.0.12.0`

## 项目简介
Kirine Client 是一款桌面端 AI 语音合成应用，支持 TTS（文本转语音）、声音克隆（Voice Clone）、音色设计（Voice Design）、模型微调（Model Training）和流式语音（Streaming Speech）五大核心能力。基于 Tauri 2 + Vue 3 构建，跨 Windows/macOS/Linux。

## 技术栈
| 层 | 技术 | 版本 |
|---|---|---|
| 前端框架 | Vue 3 + TypeScript | ^3.5.13 |
| 路由 | Vue Router | ^5.0.4 |
| 状态管理 | Pinia | ^3.0.4 |
| 样式 | TailwindCSS 3 | ^3.4.17 |
| 图标 | @heroicons/vue | ^2.2.0 |
| 桌面框架 | Tauri 2 | ^2 |
| Tauri 插件 | dialog, opener | ^2 |
| 后端语言 | Rust (2021 edition) | - |
| ORM | SeaORM 1.1 + sea-orm-migration | SQLite |
| 异步运行时 | Tokio 1.50 | full |
| HTTP 客户端 | Reqwest 0.13 | json |
| 配置解析 | config 0.15 (TOML) | toml feature |
| 日志 | tracing + tracing-subscriber | 0.1 / 0.3 |
| 构建工具 | Vite ^6.0.3 | - |

## 核心功能模块
1. **模型管理** - 安装/卸载 7 个模型（irodori_tts_v3, dots_tts, qwen3_tts, vox_cpm2, moss_tts_local, moss_tts_realtime, gpt_sovits_cpufast），自动检测设备类型，配置驱动的特性矩阵；每模型可选「当前设备」（currentDevice，单设备自动选中、多设备用户选择，决定安装/重装使用的设备）。运行环境支持 venv 与 conda_env（系统 PATH 有 conda 时优先用 conda）
2. **文本转语音 (TTS)** - 选择说话人、语言、格式，输入文本生成语音
3. **声音克隆 (Voice Clone)** - 上传参考音频 + 文本，克隆声音生成语音
4. **音色设计 (Voice Design)** - 通过 prompt 描述音色，生成语音
5. **模型微调 (Model Training)** - 单样本/数据集训练，配置驱动参数（dots_tts 支持 full fine-tune + CPU 训练）
6. **流式语音 (Streaming Speech)** - ChatUI 对话页，会话级长期进程复用，多消息实时 chunked 音频流（详见 [[streaming-speech-architecture]]）
7. **说话人管理** - 创建/导入/编辑/删除说话人档案
8. **历史记录** - 查看/回放/导出所有任务历史
9. **设置** - 存储、日志、模型目录配置，API 地址配置

## 架构概览
```
┌──────────────────────────────────────────┐
│  Vue 3 Frontend (src/)                   │
│  views/ · components/ · stores/ · types/ │
│  hooks/ · enums/ · utils/                │
├──────────────────────────────────────────┤
│  Tauri IPC (invoke<T>)                   │
├──────────────────────────────────────────┤
│  Rust Backend (src-tauri/src/)           │
│  hooks/ · config/ · service/ · client/   │
│  common/ · migration/ · utils/           │
├──────────────────────────────────────────┤
│  SQLite DB (SeaORM) · Pipeline · Scripts │
└──────────────────────────────────────────┘
```

## 当前状态
- 分支 `v.0.12.0`（开发中）；Tauri 应用版本 (tauri.conf.json) `0.12.0`；Cargo.toml version `0.1.0`
- 数据库 schema version `29`；最新迁移 `m20260723_000012_add_model_current_device`
- 已集成 7 个模型子模块（均为独立 GitHub adapter 仓库）：irodori_tts_v3, dots_tts, qwen3_tts, vox_cpm2, moss_tts_local, moss_tts_realtime, gpt_sovits_cpufast
- 远程存储模式（RemoteService）开发中：`client::ApiClient` + `utils::HttpClient` 基础设施 + `docs/remote-api.yaml` 契约就位，但 ApiClient 各方法为占位实现（bail「HTTP 调用尚未接入」），Remote 模式当前不可用
- 流式语音生成（StreamingSpeech）全栈已落地（Local 模式可用，Remote 模式不支持）

## 存储模式
后端通过 `config::StorageMode`（`config/mod.rs`）区分两种存储后端，`init_service` 据此分发：
- **Local**（默认）：`LocalService`，SQLite + 本地 Pipeline 执行 Python 脚本（当前实际可用路径，流式语音仅 Local 支持）
- **Remote**：`RemoteService`，经 `client::ApiClient` 调用远端 HTTP API（`config.toml [remote].api_url` + `api_token`），开发中、尚未接通真实 HTTP 调用；流式语音三方法直接 bail「远程存储模式暂不支持」

## 关联记忆
- [[tech-stack-frontend]] - 前端详细架构
- [[tech-stack-backend]] - 后端详细架构
- [[streaming-speech-architecture]] - 流式语音生成全栈架构
- [[model-adapter-pattern]] - 模型适配器与配置驱动
- [[data-flow-and-types]] - 数据流与类型对应
