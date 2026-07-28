---
name: tech-stack-backend
description: Tauri/Rust 后端模块与 Pipeline 架构
metadata: 
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# 后端架构 (Tauri 2 / Rust)

> 状态截至 2026-07-27 · 分支 `v.0.12.0`

## 目录结构
```
src-tauri/src/
├── main.rs                        # 入口，调用 lib
├── lib.rs                         # 模块声明 + Tauri Builder 组装
├── test_support.rs                # 测试辅助（pub mod，再导出测试所需类型/函数）
│
├── client/                        # 远端 HTTP 客户端 (开发中)
│   ├── mod.rs                     # ApiClient (持 api_url+api_token, 当前占位实现)
│   ├── entity.rs                  # CommonResponse<T>/PageRequest/PageResponse/SpeakerPageResponse + 边界 Into 转换
│   └── paths.rs                   # 远端 API 路径常量 (/api/speakers, /api/models, /api/history...)
│
├── common/                        # 通用工具模块
│   ├── mod.rs
│   ├── local_paths.rs             # 本地路径解析 (data/log/model 目录)
│   └── task_paths.rs              # 任务路径 (日志、输出文件、音频、流式 context/input/audio)
│
├── config/                        # 配置系统
│   ├── mod.rs                     # 基础类型: BaseModel, HardwareType, StorageMode, AttentionImplementation
│   ├── env_config.rs              # EnvConfig (TOML): basic/remote/training 三个配置区
│   ├── model_catalog.rs           # 模型目录: 从 JSON 文件加载模型元信息 (ModelCatalog, ModelVariantConfig)
│   └── ui_config.rs               # UI 配置: 配置驱动的参数表单 (UiConfigCatalog, ParamDefinition)
│
├── hooks/                         # Tauri 命令层
│   ├── mod.rs                     # load_hooks() 注册所有命令
│   ├── model_info.rs              # list_model_infos, get_device_type, install_model, uninstall_model, set_model_current_device
│   ├── speaker_info.rs            # create_speaker_info, import_model_as_speaker, list/update/delete
│   ├── task_history.rs            # 任务创建/查询/取消 + 音频获取/导出/删除
│   ├── streaming.rs               # 流式语音命令（create/send/cancel）+ AudioStreamEvent 协议
│   └── settings.rs                # get_settings_config, save_settings_config, get_ui_config
│
├── migration/                     # 数据库迁移 (SeaORM Migration)
│   ├── mod.rs                     # Migrator + schema version 29 + rename_column_if_needed 守卫
│   ├── create_local_schema.rs     # 初始建表
│   ├── m20260508_000002_make_tts_speaker_nullable.rs
│   ├── m20260508_000003_add_model_download_type.rs
│   ├── m20260511_000004_remove_task_history_error_message.rs
│   ├── m20260512_000005_rename_model_scale_to_model_version.rs
│   ├── m20260521_000006_rename_model_training_task_model_name_to_speaker_name.rs
│   ├── m20260522_000007_add_task_device_and_model_supported_devices.rs
│   ├── m20260604_000008_add_voice_design_tasks.rs
│   ├── m20260626_000009_add_model_supported_languages_and_drop_speaker_languages.rs
│   ├── m20260706_000010_rename_speakers_name_to_speaker_name.rs
│   ├── m20260718_000011_add_streaming_tasks.rs  # schema 27->28
│   └── m20260723_000012_add_model_current_device.rs  # schema 28->29
│
├── service/
│   ├── mod.rs                     # Service trait (25 业务方法) + ServiceImpl(Local/Remote) 分发
│   ├── models.rs                  # 所有业务模型枚举与结构体统一定义 (含流式 payload/result)
│   ├── local/                     # 本地业务逻辑层
│   │   ├── mod.rs                 # LocalService 定义 + impl Service 委托 + 通用任务句柄方法
│   │   ├── db.rs                  # DB 连接管理
│   │   ├── entity/                # SeaORM Entity 定义
│   │   │   ├── model_info.rs
│   │   │   ├── speaker.rs
│   │   │   ├── task_history.rs
│   │   │   ├── training_task.rs
│   │   │   ├── tts_task.rs
│   │   │   ├── voice_clone_task.rs
│   │   │   ├── voice_design_task.rs
│   │   │   └── streaming_task.rs
│   │   ├── model_info.rs          # 模型安装/卸载/设备检测/当前设备设置
│   │   ├── supported_models.rs    # 模型目录加载 -> ModelInfo 列表 + current_device 回填/保留
│   │   ├── speaker.rs             # 说话人 CRUD + 模型导入
│   │   ├── history.rs             # 历史记录查询 (含 load_streaming_detail)
│   │   ├── tts.rs                 # TTS 任务创建 + start_tts_inference
│   │   ├── voice_clone.rs         # 声音克隆任务创建 + start_voice_clone_inference
│   │   ├── voice_design.rs        # 音色设计任务创建 + start_voice_design_inference
│   │   ├── training.rs            # 微调任务创建 + start_training
│   │   └── streaming.rs           # 流式会话 LocalService 层 (create/send/cancel + 会话句柄 + sweep)
│   ├── remote/                    # 远程服务层 (开发中)
│   │   └── mod.rs                 # RemoteService 实现 Service trait, 委托 ApiClient; 流式三方法 bail 不支持
│   └── pipeline/                  # 任务执行管线
│       ├── mod.rs                 # ModelTaskPipeline trait + 公共工具函数 + StreamingPipelineRequest
│       ├── api/mod.rs             # PythonScriptInvocationSpec (参数文件驱动) + StreamingArgs/StreamingSpeakerArg
│       ├── pipeline.rs            # CommonModelTaskPipeline 实现
│       ├── model_paths.rs         # 模型目录路径解析
│       ├── model_artifacts.rs     # 模型产物校验 + 下载脚本参数
│       ├── script_paths.rs        # 跨平台脚本路径 (Windows/Unix)
│       ├── tts.rs                 # TTS Pipeline
│       ├── voice_clone.rs         # Voice Clone Pipeline
│       ├── voice_design.rs        # Voice Design Pipeline
│       ├── training.rs            # Training Pipeline
│       └── streaming.rs           # 流式会话 runner (帧解析纯函数 + 长期进程 + contextId 分发)
│
└── utils/                         # 工具函数
    ├── mod.rs
    ├── audio.rs                   # 音频处理
    ├── file_ops.rs                # 文件操作辅助
    ├── http.rs                    # HttpClient 封装 reqwest (Bearer Token+30s 超时, 前瞻基础设施, 待 client 启用)
    ├── process.rs                 # 子进程执行 (Shell 脚本, 可取消)
    └── time.rs                    # 时间格式化
```

## 关键机制

### 配置系统分层
| 文件 | 职责 |
|------|------|
| `config/mod.rs` | 基础类型 `BaseModel`, `HardwareType`, `StorageMode`, `AttentionImplementation` |
| `config/env_config.rs` | TOML 配置文件 (`config.toml`) - basic/remote/training |
| `config/model_catalog.rs` | JSON 模型目录 -> 模型列表、特性矩阵、下载方式 |
| `config/ui_config.rs` | JSON 参数配置 -> 配置驱动的参数表单 |

### 业务模型统一
`service/models.rs` 集中定义所有枚举（`AppLanguage`, `HistoryTaskType`(含 StreamingSpeech), `TaskStatus`, `SpeakerStatus`, `SpeakerSource`, `ModelDownloadType`, `ModelTrainingSampleType`, `ModelTrainingFileKind`, `TextToSpeechFormat`）和结构体（`ModelInfo`, `SpeakerInfo`, `HistoryRecord`, 各 TaskDetail/Payload/Result，流式 `CreateStreamingSpeechTaskPayload`/`SendStreamingMessagePayload`/`StreamingSpeechTaskResult`/`StreamingSpeakerInput`）。

分页类型：`PageRequest<T>` (含 `Default`)、`Page<T>` (含 `::new()`)、`SpeakerFilter` / `ModelFilter` / `HistoryFilter`、`SpeakerPageResult` (分页 + 统计)。`list_model_infos` / `list_speaker_infos` / `list_history_records` 三个 service 方法与对应 hooks 命令均接收 `PageRequest<TFilter>`、返回 `Page<T>` / `SpeakerPageResult`；`list_history_records` 仅返回 `HistoryRecordSummary`（不含 detail/taskLog）。

### Pipeline 架构
- `ModelTaskPipeline` trait 定义 4 个 pipeline 方法（training/tts/voice_clone/voice_design）
- `CommonModelTaskPipeline` 统一实现，通过 `PythonScriptInvocationSpec` 用参数文件 (`--params-file`) 驱动 Python 脚本
- 支持任务取消（`watch::Receiver<bool>` + `run_logged_shell_script_cancellable`）
- 跨平台脚本执行（Windows: PowerShell, Unix: Shell）
- **流式语音不走 ModelTaskPipeline**：它有独立的 `run_streaming_session` runner（会话级长期进程，非一次性脚本），见 [[streaming-speech-architecture]]

### Pipeline 执行模型（conda 支持）
Pipeline 通过 shell 脚本包装器 `begin_llm_task` 执行 Python 脚本（不直接调 venv python），以支持 conda 环境。流式语音 runner 同样走 `begin_llm_task` -> `streaming.py`。

- **包装器脚本**: `src-model/scripts/{windows/unix}/begin_llm_task.{ps1,sh}`
  - 解析 CLI 参数 `--base-model` `--script-path` `--params-file` `--log-path` `--task-log-file`
  - 探测 conda 环境 -> 选择 python 命令 (`Resolve-PythonCommand`)
  - 注入 `-X utf8 -X faulthandler -u` 等 python 参数
  - 将 stdout/stderr 追加写入 task-log-file
- **conda 环境策略** (Windows `common.ps1`): conda env 位于 `<model_root>/conda_env`，通过 `conda run --prefix <conda_env> python --` 运行，**优先于** venv (`<model_root>/venv`)。`Get-CondaExecutable` 优先选 `*.exe` 的 conda 命令。
- **Rust 侧** (`script_paths.rs`): 脚本路径经 `begin_llm_task_relative_path()` / `src_model_begin_llm_task_script_path()` 解析（固定脚本路径，不依赖 `base_model`）。
- **`pipeline/mod.rs`**: `run_llm_task_invocation` (及 `_cancellable`) 调 `run_logged_shell_script`，额外透传 `--log-path` `--base-model` `--model-version` 给包装器；`PipelineBootstrapPaths.begin_llm_task_script_path` 字段。
- **`utils/process.rs`**: 仅保留 `run_logged_shell_script[_cancellable]`（无直接调 python 的封装）。
- **各 pipeline (tts/voice_clone/voice_design/training)**: `ResolvedXxxPaths.begin_llm_task_script_path` 字段；不预校验 venv python 是否存在（交由包装器运行时处理）。
- Windows 与 Unix 包装器脚本均存在。

### Windows 安装器 src-model 合并
`src-tauri/windows/prepare-dependencies/common.nsh` 的 `MergeBundledZipArchiveContents`：先解压到 `%TEMP%` 临时目录，再逐文件 `Copy-Item` 合并到目标（不删除既有文件），保留 venv 与 conda_env。（旧实现解压前 `Remove-Item` 目标目录下除 `venv`/`.venv` 外内容，会误删 `conda_env`。）

### 远程存储模式 (RemoteService / ApiClient) - 开发中
后端第二种存储后端 `RemoteService`，与 `LocalService` 并列实现同一 `Service` trait。

- **`Service` trait** (`service/mod.rs`)：25 个业务方法 + `new`/`close` 生命周期（含流式 3 方法：`create_streaming_speech_task`/`send_streaming_message`/`cancel_streaming_task`；含模型当前设备 `set_model_current_device`）。`ServiceImpl` 枚举（`Local(LocalService)` / `Remote(RemoteService)`）+ `init_service(config)` 按 `config.mode()` 分发。
- **`client::ApiClient`** (`client/mod.rs`)：持有 `api_url` + 可选 `api_token`，方法签名与 `Service` trait 业务方法一一对应。**当前为占位实现**--`placeholder()` 打印 method/url/params 后 `bail!("client HTTP 调用尚未接入")`，即 Remote 模式当前会报错，不可用。
- **`client::paths`** / **`client::entity`**：远端 API 路径常量 + `CommonResponse<T>`/分页响应边界转换。
- **`utils::HttpClient`** (`utils/http.rs`)：封装 `reqwest::Client`（30s 超时 + Bearer Token），`#[allow(dead_code)]` 前瞻基础设施。
- **`docs/remote-api.yaml`**：OpenAPI 3.0.3 契约文档，与 `Service` trait 业务方法一一对应。
- ⚠️ **流式语音远程不支持**：`RemoteService` 的三个流式方法直接 `bail!("远程存储模式暂不支持流式语音会话")`，不经 ApiClient。

### speakers 表 speaker_name 列
DB 列 `speakers.speaker_name`（`rename_column_if_needed` 守卫）。`entity/speaker.rs` 字段 `speaker_name`。`service/models.rs` 的 `SpeakerInfo`/`CreateSpeakerPayload`/`UpdateSpeakerPayload`/`ImportModelAsSpeakerPayload` 字段统一 `speaker_name`（camelCase `speakerName`）。前端联动见 [[tech-stack-frontend]]，DB 文件同步见 [[db-schema-sync-rule]]。

### model_info.current_device 列
DB 列 `model_info.current_device TEXT`（可空，schema 29，migration `m20260723_000012`）。存储用户在模型管理页选择的当前设备（`Option<HardwareType>`）。
- **写入**：`LocalService::set_model_current_device_impl`（`service/local/model_info.rs`）先校验 device ∈ `supported_devices`（非法 bail，不触达脚本），再 `update` 写库 + 刷新 `modify_time`，返回最新 `ModelInfo`。`parse_current_device` 将 None/空串/非法值统一归一为 None。
- **sync 回填/保留**：`supported_models.rs::resolve_current_device_value` 在每次启动 sync 的 upsert insert+update 两分支均跑——已有值仍属 supportedDevices 则保留（跨会话持久化用户选择），失效且单设备则回填唯一设备，多设备无选择则 None。
- **speaker.rs** 的 `map_speaker_*` 也映射 currentDevice 到内嵌 ModelInfo。
- **Remote**：`RemoteService` 委托 `ApiClient::set_model_current_device` -> `PUT /api/models/{id}/current-device`（`client/paths.rs::MODEL_CURRENT_DEVICE`，当前占位实现）。
- **测试**：`tests/model_hooks.rs` 4 个新用例（单设备回填 / 合法写入持久化 / 不支持设备拒绝 / 未知 model_id 报错）。

### 流式语音生成 (StreamingSpeech)
会话级长期进程的实时流式语音合成，区别于一次性脚本的 TTS/克隆/设计。完整架构（任务类型/DB/hooks/Service/LocalService 会话层/Pipeline runner/帧协议/并发模型/清扫/测试）见 [[streaming-speech-architecture]]。要点：
- 新表 `streaming_tasks` + migration `m20260718_000011`（schema 27->28）；`HistoryTaskType::StreamingSpeech`（`"streaming-speech"` / 目录 `"streaming"`）。
- `hooks/streaming.rs`：`AudioStreamEvent` 协议（started/chunk/finished/error，经 `ipc::Channel` 下发）+ 3 命令（create/send/cancel）。
- `service/local/streaming.rs`：会话创建/发消息/取消/清扫 + 会话句柄管理（`ActiveTaskControl.streaming_extra`）。
- `service/pipeline/streaming.rs`：帧解析纯函数 + `run_streaming_session` 长期 runner（stdout 分帧按 contextId 分发、cancel kill、状态机 Running->Cancelled/Failed）。
- Remote 模式不支持（bail）。

### 测试约定
6 个测试文件（speaker_hooks/model_hooks/history_hooks/settings_hooks/pipeline_params/shell_scripts）。规则：临时库隔离、平台门控脚本测试、任务执行 hooks 仅验证脚本调用参数不真正执行。详见 [[tests-dir-over-inline]]。不跑全局 `cargo fmt`（见 [[src-tauri-not-rustfmt-clean]]），以 check/clippy/test 兜底。

## Tauri 命令完整列表
| 命令 | Hook 模块 | 说明 |
|------|-----------|------|
| `list_model_infos` | model_info | 获取模型列表 - **分页** `PageRequest<ModelFilter>` -> `Page<ModelInfo>` |
| `get_device_type` | model_info | 查询设备类型 |
| `install_model` | model_info | 安装模型 |
| `uninstall_model` | model_info | 卸载模型 |
| `set_model_current_device` | model_info | 设置模型当前设备（校验 ∈ supportedDevices 后持久化 currentDevice） |
| `create_speaker_info` | speaker_info | 创建说话人 |
| `import_model_as_speaker` | speaker_info | 导入模型为说话人 |
| `list_speaker_infos` | speaker_info | 列出说话人 - **分页** `PageRequest<SpeakerFilter>` -> `SpeakerPageResult` |
| `update_speaker_info` | speaker_info | 更新说话人 |
| `delete_speaker_info` | speaker_info | 删除说话人 |
| `create_text_to_speech_task` | task_history | 创建 TTS 任务 |
| `create_voice_clone_task` | task_history | 创建声音克隆任务 |
| `create_voice_design_task` | task_history | 创建音色设计任务 |
| `create_model_training_task` | task_history | 创建微调任务 |
| `list_history_records` | task_history | 历史列表 - **分页** `PageRequest<HistoryFilter>` -> `Page<HistoryRecordSummary>` |
| `get_history_record` | task_history | 历史详情 |
| `cancel_history_task` | task_history | 取消任务 |
| `delete_history_record` | task_history | 删除历史记录 |
| `get_text_to_speech_audio` | task_history | 获取 TTS 音频 |
| `get_voice_clone_audio` | task_history | 获取声音克隆音频 |
| `get_voice_design_audio` | task_history | 获取音色设计音频 |
| `save_text_to_speech_audio_as` | task_history | TTS 音频另存为 |
| `save_voice_clone_audio_as` | task_history | 声音克隆音频另存为 |
| `save_voice_design_audio_as` | task_history | 音色设计音频另存为 |
| `save_model_training_template_as` | task_history | 微调模板另存为 |
| `create_streaming_speech_task` | streaming | 创建流式语音会话（建表 + 拉起长期进程） |
| `send_streaming_message` | streaming | 发送一条流式消息（带 `on_event: Channel`，等终帧/300s 超时） |
| `cancel_streaming_task` | streaming | 取消流式会话（kill 子进程） |
| `get_settings_config` | settings | 获取配置 |
| `save_settings_config` | settings | 保存配置 |
| `get_ui_config` | settings | 获取 UI 参数配置 |

## 关联记忆
- [[project-overview]]
- [[streaming-speech-architecture]]
- [[tech-stack-frontend]]
- [[model-adapter-pattern]]
- [[data-flow-and-types]]
