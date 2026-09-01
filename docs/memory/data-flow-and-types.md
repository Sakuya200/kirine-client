---
name: data-flow-and-types
description: 完整数据流、类型体系与前后端对接方式
metadata: 
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# 数据流与类型体系

> 状态截至 2026-09-01 · 分支 `v.0.12.0`

## 前后端通信
前端通过 `@tauri-apps/api/core` 的 `invoke<T>(command, payload)` 调用后端 Tauri 命令。所有数据经过 serde 自动序列化/反序列化（后端用 `camelCase`，前端一致）。流式语音额外用 Tauri 2 `Channel<InvokeResponseBody>` 下发实时事件（控制事件 Json、音频 chunk Raw 二进制，前端收 ArrayBuffer）。

## 核心 Tauri 命令列表
| 命令 | Hook 模块 | 方向 | 说明 |
|------|-----------|------|------|
| `list_model_infos` | model_info | -> | 获取模型列表 (含特性矩阵、设备支持、下载状态) - **分页**: `PageRequest<ModelFilter>` -> `Page<ModelInfo>` |
| `install_model` | model_info | -> | 安装模型 (init+download) |
| `uninstall_model` | model_info | -> | 卸载模型 (清理产物+venv) |
| `set_model_current_device` | model_info | -> | 设置模型当前设备 (校验 ∈ supportedDevices 后持久化 currentDevice，返回 ModelInfo) |
| `get_device_type` | model_info | -> | 查询设备类型 (cpu/cuda) |
| `create_speaker_info` | speaker_info | -> | 创建说话人 |
| `import_model_as_speaker` | speaker_info | -> | 导入外部模型为说话人 |
| `list_speaker_infos` | speaker_info | -> | 列出说话人 - **分页**: `PageRequest<SpeakerFilter>` -> `SpeakerPageResult` |
| `update_speaker_info` | speaker_info | -> | 更新说话人 |
| `delete_speaker_info` | speaker_info | -> | 删除说话人 |
| `create_tts_task` | task_history | -> | 创建 TTS 任务 |
| `create_voice_clone_task` | task_history | -> | 创建声音克隆任务 |
| `create_voice_design_task` | task_history | -> | 创建音色设计任务 |
| `create_model_training_task` | task_history | -> | 创建微调任务 |
| `list_history_records` | task_history | -> | 历史列表 - **分页**: `PageRequest<HistoryFilter>` -> `Page<HistoryRecordSummary>` |
| `get_history_record` | task_history | -> | 历史详情 (含 detail JSON) |
| `cancel_history_task` | task_history | -> | 取消运行中任务 |
| `delete_history_record` | task_history | -> | 删除历史记录 |
| `get_generated_audio` | task_history | -> | 按 `GeneratedAudioSource` 读取生成音频字节，前端构造 Blob URL 播放 |
| `save_generated_audio_as` | task_history | -> | 按同一 `GeneratedAudioSource` 另存为生成音频 |
| `save_model_training_template_as` | task_history | -> | 微调模板导出 |
| `create_streaming_speech_task` | streaming | -> | 创建流式语音会话（`CreateStreamingSpeechTaskPayload` -> `StreamingSpeechTaskResult`，建 task_history+streaming_tasks 记录 + 拉起长期进程） |
| `send_streaming_message` | streaming | -> | 发送一条流式消息（`SendStreamingMessagePayload` + `on_event: Channel<InvokeResponseBody>`，等终帧或 300s 超时） |
| `cancel_streaming_task` | streaming | -> | 取消流式会话（`taskId` -> `bool`，kill 子进程） |
| `get_settings_config` | settings | -> | 获取配置 |
| `save_settings_config` | settings | -> | 保存配置 |
| `get_ui_config` | settings | -> | 获取 UI 参数配置 |

## 任务执行流程

### 一次性任务（TTS / Voice Clone / Voice Design / Model Training）
```
前端 invoke -> Tauri Command (hooks/)
   -> service::local 模块
   -> 校验参数 + 设备检查
   -> 写入 DB (task_history + 对应 entity)
   -> 启动 Pipeline 后台线程 (tokio::spawn)
   -> 返回 TaskResult (含 taskId)

Pipeline 后台线程:
   -> 解析 PipelineBootstrapPaths (model_paths.rs)
   -> validate_and_init (运行时环境初始化)
   -> validate_and_download (模型权重下载，已下载则跳过)
   -> 构造 PythonScriptInvocationSpec (api/mod.rs)
   -> 写入参数 JSON 文件
   -> 执行 begin_llm_task -> python script.py --params-file /path/params.json
   -> 支持取消 (watch::Receiver<bool>)
   -> 更新任务状态 (Running -> Completed/Failed)
   -> 写入音频文件 / 模型产物

前端轮询:
   -> 每 3 秒 invoke get_history_record
   -> 状态为 done 时加载音频播放
```

### 流式语音（StreamingSpeech）- 会话级长期进程
```
前端 invoke create_streaming_speech_task
   -> LocalService::create_streaming_speech_task_impl
   -> 事务写 task_history(Pending) + streaming_tasks + 初始 context.json
   -> register_streaming_session + start_streaming_session (spawn runner)
   -> 返回 StreamingSpeechTaskResult { taskId, 三个路径, status: Pending }

runner (run_streaming_session, 长期存活):
   -> 置 Running -> spawn begin_llm_task -> streaming.py
   -> bind 127.0.0.1 随机端口 + accept + AUTH 令牌校验
   -> reader task 分帧读 Socket: parse_streaming_frame -> frame_to_event -> forward_event
   -> 按 contextId 分发到 message_channels 的 Channel (chunk 为 Raw 二进制)
   -> cancel 信号到达 kill 子进程; 退出置 Cancelled/Failed
   -> init_db 时 sweep 残留 Running 流式会话为 Cancelled

前端发消息 invoke send_streaming_message (带 on_event Channel):
   -> 校验 Running -> 注册 Channel -> append context.json + input.jsonl (审计)
   -> 经 input_tx 推 INPUT 帧 -> Python read_frame 收到后合成
   -> 等终帧 oneshot (300s 超时) -> 成功自增 message_count
   -> 前端 StreamableAudioPlayer 订阅 Channel 累计 chunk (ArrayBuffer) 播放
```
完整架构（帧协议/并发模型/状态机/清扫）见 [[streaming-speech-architecture]]。

## 核心类型对应 (TS ↔ Rust)

| TS 接口 (domain.ts) | Rust 结构体 (models.rs) | 用途 |
|---------------------|------------------------|------|
| `ModelInfo` | `ModelInfo` | 模型信息 (含特性矩阵、supportedDevices、currentDevice、下载状态) |
| `ModelMutationResult` | `ModelMutationResult` | 安装/卸载操作结果 (含路径变更) |
| `SpeakerProfile` | `SpeakerInfo` | 说话人信息 |
| `HistoryRecordBase` | `HistoryRecord` | 历史记录 (含 device, taskLog 字段) |
| `TextToSpeechTaskDetail` | `TextToSpeechTaskDetail` | TTS 任务详情 (speakerId 可 null) |
| `VoiceCloneTaskDetail` | `VoiceCloneTaskDetail` | 声音克隆详情 (含 refAudio* 字段) |
| `VoiceDesignTaskDetail` | `VoiceDesignTaskDetail` | 音色设计详情 (含 prompt 字段) |
| `ModelTrainingTaskDetail` | `ModelTrainingTaskDetail` | 微调任务详情 (含 samples 数组) |
| `ModelTrainingSampleDetail` | `ModelTrainingSampleInput` | 训练样本 (含 primaryFile/secondaryFile) |
| `ModelTrainingFileDetail` | `ModelTrainingFileInput` | 训练文件 (fileName/fileKind/filePath) |
| `StreamingSpeakerInput` | `StreamingSpeakerInput` | 流式说话人 (name/baseModel/refAudio*/refText) |
| `CreateStreamingSpeechTaskPayload` | `CreateStreamingSpeechTaskPayload` | 创建流式会话请求 (baseModel/device/language/modelParams/speakers) |
| `SendStreamingMessagePayload` | `SendStreamingMessagePayload` | 发送流式消息 (taskId/contextId/speakerName/text) |
| `StreamingSpeechTaskResult` | `StreamingSpeechTaskResult` | 创建会话响应 (taskId + context/input/audio 路径 + status) |

## 枚举对应
| TS 枚举/类型 | Rust 枚举 | 值 |
|-------------|----------|-----|
| `HistoryTaskType` | `HistoryTaskType` | model-training, text-to-speech, voice-clone, voice-design, **streaming-speech** |
| `TaskStatus` | `TaskStatus` | pending, running, completed, cancelled, failed |
| `HardwareType` | `HardwareType` | cpu, cuda |
| `AppLanguage` | `AppLanguage` | chinese, english, japanese, korean |
| `TextToSpeechFormat` | `TextToSpeechFormat` | wav, mp3, flac |
| `SpeakerStatus` | `SpeakerStatus` | ready, training, disabled |
| `SpeakerSource` | `SpeakerSource` | local, preset, remote |
| `ModelDownloadType` | `ModelDownloadType` | HF-Like, Custom |
| `ModelTrainingSampleType` | `ModelTrainingSampleType` | single, dataset |
| `ModelTrainingFileKind` | `ModelTrainingFileKind` | audio, archive, annotation |
| `AudioStreamEvent` (streamingSpeech.ts) | `AudioStreamEvent` (hooks/streaming.rs) | started, finished, error{message}（控制事件 Json）；chunk 为 ArrayBuffer（Raw 路径，无 TS↔Rust 枚举对应） |

> ⚠️ `ModelInstallStatus`（`enums/status.ts`）为**前端会话级**状态，**无对应 Rust 枚举**：值 `installed` / `not-installed` / `failed`。Rust 侧 `ModelInfo.downloaded` 仅反映权重是否就绪，无法表达"最近一次安装失败"，故 `models` store 用 `failedModelIds: Set<number>` 在前端单独追踪。

> ⚠️ 流式语音的 `StreamingSpeakerConfig` / `StreamingChatMessage` / `StreamingSessionConfig`（`types/streaming.ts`）为**前端专用**类型（说话人本地管理、聊天消息、抽屉配置），无 1:1 Rust 结构对应；但 `StreamingSpeakerInput` 与 `AudioStreamEvent` 与后端契约对齐。

## 各 TaskDetail 差异字段

| 特有字段 | TTS | Voice Clone | Voice Design | Model Training |
|---------|-----|-------------|--------------|----------------|
| `speakerId` | ✅ (nullable) | - | - | - |
| `refAudioName/Path/Text` | - | ✅ | - | - |
| `prompt` | - | - | ✅ | - |
| `speakerName` | - | - | - | ✅ |
| `sampleCount/samples/notes` | - | - | - | ✅ |
| `format` | ✅ | ✅ | ✅ | - |
| `exportAudioName` | ✅ | ✅ | ✅ | - |

> 流式语音不沿用 TaskDetail 模式，详情存于 `streaming_tasks` 表 + `context.json`（messages 数组），经 `history.rs::load_streaming_detail` 加载。

## 分页体系

服务端分页：`list_model_infos` / `list_speaker_infos` / `list_history_records` 三个查询接口返回分页响应。

| TS 接口 (domain.ts) | Rust 结构体 (models.rs) | 用途 |
|---------------------|------------------------|------|
| `PageRequest<TFilter>` | `PageRequest<T>` | 统一分页请求 `{ page, pageSize, filter }` (实现 Default) |
| `Page<T>` | `Page<T>` | 通用分页响应 `{ items, total, page, pageSize, totalPages }` (含 `::new()`) |
| `SpeakerFilter` | `SpeakerFilter` | 说话人筛选 `{ keyword, status }` (language 已移除) |
| `ModelFilter` | `ModelFilter` | 模型筛选 `{ keyword, downloaded, feature }` |
| `HistoryFilter` | `HistoryFilter` | 历史筛选 `{ keyword, taskType, status }` |
| `SpeakerPagedResult` | `SpeakerPageResult` | 说话人分页响应 (`Page<SpeakerProfile>` + 统计字段) |
| `HistoryRecordSummary` | `HistoryRecord` (摘要用途) | 历史列表仅返回摘要 (不含 detail/taskLog) |

前端封装：
- `src/hooks/usePagination.ts` - 统一服务端分页 hook，维护 items/total/page/pageSize/loading，filter 变化时 debounce(300ms) + 重置第 1 页 + 重新查询；用 `requestSeed` 防止乱序响应覆盖
- `src/hooks/loadRecentHistoryRecords.ts` - 业务页结果卡用：先取一页 summary，再逐条 `get_history_record` 懒加载 detail
- `src/components/common/BasePagination.vue` - 通用分页 UI 组件

## 模型语言灵活配置

语言下拉由模型声明驱动：

- `ModelInfo` 含 `supportedLanguages: AppLanguage[]`（TS）/ `supported_languages: Vec<AppLanguage>`（Rust），源自 `model_info.supported_languages` 列（JSON 数组字符串）。
- `model-config.json` 的 model 对象含 `supportedLanguages`；缺省 `[chinese, english, japanese]`。`sync_supported_models` 启动时写入 DB。
- `AppLanguage` 枚举含 `Korean`，共 4 值。qwen3_tts 配置为 4 种语言，其余模型为 3 种。
- 前端 `useModelStore` 提供 `getSupportedLanguages(baseModel, modelVersion)`。4 个任务页的"输出语言"`BaseListbox` 用 computed `languageOptions`，watcher 在模型切换时重置。
- 说话人侧无语言字段（`speakers.languages_json` 列已删除）；任务级 `language` 字段保留。

## 模型当前设备 (currentDevice)

模型管理页每模型可选「当前设备」，决定 `install_model`/`reinstallModel` 使用的设备（不再默认 Cpu）。

- **字段**：`ModelInfo.currentDevice: HardwareType | null`（TS）/ `current_device: Option<HardwareType>`（Rust）。DB 列 `model_info.current_device TEXT`（可空，schema 29，migration `m20260723_000012`）。
- **回填规则**（`supported_models.rs::resolve_current_device_value`，每次启动 sync 跑）：单设备模型自动回填唯一设备；多设备模型留空待用户选；已有值仍属 supportedDevices 则保留（跨 sync 持久化用户选择），失效则按上述规则纠偏。
- **写入**：`set_model_current_device(modelId, device)` Tauri 命令 -> LocalService 校验 device ∈ supportedDevices 后写库（非法设备 Err，不触达脚本）；RemoteService 委托 ApiClient `PUT /api/models/{id}/current-device`。
- **前端**：`models` store `normalizeModelInfo` 将缺省/非法/失效 currentDevice 归一为 null；`setCurrentDevice(modelId, device)` 写库并即时替换本地条目。ModelManageView 用 `BaseListbox(teleport)` 下拉展示，安装按钮在 currentDevice 为 null 时禁用。

## speakers 字段 speakerName + 远程存储模式

- **speaker 字段**：`SpeakerProfile.speakerName`（对齐后端 `SpeakerInfo.speaker_name`）。create/update/import 三处 `invoke` payload key 统一为 `speakerName`。
- **远程存储模式**：后端 `Service` trait 含 `RemoteService` 实现，`StorageMode::Remote` 时启用，经 `client::ApiClient` 调用远端 HTTP API。新增 Rust 类型（前端复用 `domain.ts`，无独立 TS 接口）：`TextToSpeechAudioAsset`/`VoiceCloneAudioAsset`/`VoiceDesignAudioAsset`（`{ taskId, fileName, contentType, bytes: Vec<u8> }`）、`UpdateTaskStatusPayload`（`{ taskId, status, durationSeconds? }`）、各 `*TaskResult`。`Service` trait 共 **25 业务方法** + `new`/`close`（含流式 3 方法）。`ApiClient` 当前为占位实现，Remote 模式不可用；流式三方法直接 bail 不支持。`set_model_current_device` 对应远端 `PUT /api/models/{id}/current-device`。完整契约见 `docs/remote-api.yaml`。详见 [[tech-stack-backend]]。

## UI 配置类型

| TS 接口 (uiConfig.ts) | Rust 结构体 (ui_config.rs) | 用途 |
|----------------------|--------------------------|------|
| `UiConfigCatalog` | `UiConfigCatalog` | UI 配置总目录 |
| `TaskParamConfig` | `TaskParamConfig` | 按 baseModel+task 维度的参数配置 |
| `ParamDefinition` | `ParamDefinition` | 参数定义 (类型/组件/默认值/校验) |
| `ComponentProps` | `ComponentProps` | 组件属性 (label/options/visibleWhen 等) |
| `SelectOption` | `SelectOption` | 下拉选项 |
| `VisibleWhenRule` | `VisibleWhenRule` | 条件显示规则 |
| `UiParamType` | `UiParamType` | number/string/boolean |
| `UiComponentType` | `UiComponentType` | input-number/input-text/textarea/select/switch/input-audio-file/input-text-file |

## 流式语音（StreamingSpeech）

完整架构见 [[streaming-speech-architecture]]。

**前端类型**（`types/streaming.ts`）：
- `StreamingSpeakerConfig`：流式说话人配置（id/name/category/baseModel/refAudioPath/refAudioName/refText）。`voice-clone` 使用前端本地参考音频与台词；`trained` 从 Ready 说话人按基础模型选择并携带 `speakerDirName`；`preset` 仍为预留。时间字段后端生成、前端只读（见 [[time-field-naming-rule]]）。
- `StreamingChatMessage`：聊天消息（role/text/synthText/speakerId/taskId/contextId/status）；assistant 消息挂 `StreamableAudioPlayer(mode='stream')`。
- `StreamingSessionConfig`：会话级配置（baseModel/modelVersion/device/language/modelParams），抽屉编辑、聊天页消费；提交后端 payload 不含时间。

**前后端契约类型**（`domain.ts` ↔ `service/models.rs` + `hooks/streaming.rs`）：`StreamingSpeakerInput` / `CreateStreamingSpeechTaskPayload` / `SendStreamingMessagePayload` / `StreamingSpeechTaskResult` / `AudioStreamEvent`（serde `tag="type"` + `rename_all="camelCase"`：`started` / `finished` / `error{message}`，仅控制事件；chunk 不在枚举中，以 `InvokeResponseBody::Raw` 二进制直接下发）。`on_event` 为 `Channel<InvokeResponseBody>`，前端 `onmessage` 收到 ArrayBuffer（chunk）或 JSON 对象（控制事件）。

**帧协议**：Python↔Rust 走环回 TCP Socket 二进制帧（`[u32 LE len][kind u8][payload]`）：CHUNK 帧为 `contextId 长度前缀 + contextId + PCM 裸字节`（首帧前置 WAV 哨兵头，无 base64/JSON），CONTROL 帧为 JSON，INPUT 帧（Rust→Python）为 JSON。`contextId` 由前端生成，多路复用同一会话进程。详见 [[streaming-speech-architecture]]。

## 统一生成音频读取、回放与导出

`GeneratedAudioSource` 是所有已生成音频的唯一定位契约：`text-to-speech`、`voice-clone`、`voice-design` 使用 `historyId`，`streaming-speech` 使用 `historyId + messageId`（后端兼容 `contextId` 别名）。

前端通过 `get_generated_audio` 取得 `{ fileName, contentType, bytes }`，以 `Uint8Array` 和 `Blob` 构建 object URL 后交给浏览器 `Audio` 播放；组件卸载时释放 object URL。导出复用完全相同的 source，通过 `save_generated_audio_as` 在 Rust 侧读取字节并弹出另存为对话框。该路径不依赖 WebView 对本地文件或 asset URL 的媒体访问能力，历史流式消息也不依赖会话运行句柄。

前端 `useStreamableAudioPlayer` 订阅 Channel、累计 chunk 为 Blob、点击播放时构建 objectURL，缓冲播放语义。`streamingSpeech` store 首条消息 invoke `create_streaming_speech_task` 拿 taskId 回填，后续 invoke `send_streaming_message`，取消 invoke `cancel_streaming_task`。

## 关联记忆
- [[project-overview]]
- [[streaming-speech-architecture]]
- [[model-adapter-pattern]]
- [[tech-stack-frontend]]
- [[tech-stack-backend]]
- [[time-field-naming-rule]]
