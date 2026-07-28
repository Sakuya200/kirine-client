---
name: tech-stack-frontend
description: Vue 3 前端目录结构与关键模式
metadata: 
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# 前端架构 (Vue 3 + TypeScript)

> 状态截至 2026-07-27 · 分支 `v.0.12.0`

## 目录结构
```
src/
├── assets/styles/          # tailwind.css, theme.css
├── components/
│   ├── common/             # 通用 UI 组件
│   │   ├── BaseDialog.vue
│   │   ├── BaseListbox.vue          # 通用下拉（`teleport` prop 可将面板 Teleport 到 body 以 fixed 跟随按钮，避免被表格 overflow 裁切）
│   │   ├── BaseLoadingIndicator.vue
│   │   ├── BaseLoadingBanner.vue
│   │   ├── BaseButton.vue
│   │   ├── BaseTooltip.vue
│   │   ├── BaseTopNoticeBar.vue
│   │   ├── PageHeader.vue
│   │   ├── PanelCard.vue
│   │   ├── StatusPill.vue
│   │   ├── RecentTaskList.vue
│   │   ├── AudioResultPlayer.vue
│   │   ├── GeneratedAudioResultCard.vue
│   │   ├── BasePagination.vue          # 通用分页 UI 组件
│   │   ├── StreamableAudioPlayer.vue   # 流式/路径音频播放
│   │   └── WarningConfirmDialog.vue
│   ├── form/               # 任务表单组件
│   │   ├── GenericTaskParamsForm.vue      # 配置驱动的通用参数表单
│   │   ├── TextToSpeechTaskDetailForm.vue
│   │   ├── VoiceCloneTaskDetailForm.vue
│   │   ├── VoiceDesignTaskDetailForm.vue
│   │   ├── ModelTrainingTaskDetailForm.vue
│   │   └── ModelTrainingTemplateDownloadDialog.vue
│   ├── history/
│   │   └── HistoryTaskDetailDialog.vue
│   ├── streaming/            # 流式语音生成页专用组件
│   │   ├── StreamingConfigDrawer.vue   # 右侧可拖拽配置抽屉
│   │   └── StreamingSpeakerForm.vue    # 说话人增删改表单
│   └── ui/                 # 配置驱动的 UI 参数组件
│       ├── UiParamInputField.vue          # 数字/文本输入
│       ├── UiParamTextareaField.vue       # 多行文本
│       ├── UiParamSelectField.vue         # 下拉选择
│       ├── UiParamSwitchField.vue         # 开关
│       ├── UiParamAudioFileField.vue      # 音频文件选择
│       ├── UiParamTextFileField.vue       # 文本文件选择
│       └── UiParamEmptyState.vue          # 无参数占位
├── enums/
│   ├── language.ts          # AppLanguage (chinese/english/japanese/korean) + 标签映射
│   ├── modelTraining.ts     # ModelTrainingSampleType, ModelTrainingFileKind
│   ├── settings.ts          # HardwareType (cpu/cuda)
│   ├── status.ts            # TaskStatus, SpeakerStatus, ModelInstallStatus + 样式映射
│   ├── task.ts              # HistoryTaskType (5 种任务类型，含 streaming-speech) + 路由/文本映射
│   └── textToSpeech.ts      # TextToSpeechFormat (wav/mp3/flac)
├── hooks/
│   ├── useErrorMessage.ts   # 错误消息格式化
│   ├── useTaskAudioPlayer.ts # 音频播放
│   ├── useStreamableAudioPlayer.ts # 流式/路径双模式音频播放
│   ├── useTaskDeviceTypeGuard.ts # 设备类型校验
│   ├── usePagination.ts     # 统一服务端分页 hook
│   ├── loadRecentHistoryRecords.ts # 业务页结果卡懒加载历史 detail
│   └── usePollingResume.ts  # 系统睡眠/锁屏唤醒后追赶刷新
├── routers/index.ts         # 路由定义 (10 路由，含 /streaming-speech)
├── stores/
│   ├── models.ts            # 模型列表 CRUD + 设备类型 + 功能检测 + 当前设备选择 (setCurrentDevice) + 安装失败追踪 (failedModelIds)
│   ├── speakers.ts          # 说话人 CRUD + 导入
│   ├── streamingSpeech.ts   # 流式语音会话状态 (说话人/消息/会话配置 + create/send/cancel invoke)
│   ├── taskPreferences.ts   # 任务偏好 (fixedBaseModel)
│   ├── ui.ts                # UI 状态 (侧栏、通知)
│   └── uiConfig.ts          # UI 参数配置 (配置驱动的参数渲染)
├── types/
│   ├── domain.ts            # 核心领域模型和类型 (含流式 StreamingSpeakerInput/TaskResult 等后端契约)
│   ├── streaming.ts         # 流式语音前端类型 (说话人/消息/会话配置；与 Rust 后端契约对齐)
│   └── uiConfig.ts          # UI 配置类型系统
├── utils/
│   ├── createTaskExportAudioName.ts
│   ├── formatDurationClock.ts
│   └── uiConfigModelParams.ts  # 配置->模型参数转换
├── views/
│   ├── ModelTrainingView.vue    # 微调 (默认路由)
│   ├── TextToSpeechView.vue     # TTS
│   ├── VoiceCloneView.vue       # 声音克隆
│   ├── VoiceDesignView.vue      # 音色设计
│   ├── ModelManageView.vue      # 模型管理（含「当前设备」列，BaseListbox teleport）
│   ├── SpeakersView.vue         # 说话人
│   ├── StreamingSpeechView.vue  # 流式语音 (ChatUI)
│   ├── HistoryView.vue          # 历史记录
│   ├── SettingsView.vue         # 设置
│   └── NotFoundView.vue         # 404
├── App.vue
└── main.ts
```

## 关键设计模式

### 配置驱动的参数表单
模型参数不硬编码为表单字段，由后端 `ui_config.rs` 从 JSON 配置文件动态加载，前端通过 `GenericTaskParamsForm.vue` + `components/ui/` 系列组件统一渲染。

流程：
1. 后端 `get_ui_config` 命令返回 `UiConfigCatalog`
2. 前端 `uiConfig.ts` store 管理配置
3. `GenericTaskParamsForm.vue` 根据 `ParamDefinition` 动态选择 `UiParam*` 组件
4. `uiConfigModelParams.ts` 负责将 UI 值转换为后端期望的 `model_params` JSON

### 类型系统
- `domain.ts` 定义与后端 1:1 对应的 TS 接口（camelCase 序列化）
- `BaseModel` 为 `string` 类型（非枚举），由后端 `BaseModel` Rust enum 定义具体值
- `HistoryRecord` 为任务类型的 discriminated union（按 `taskType` 区分，含 streaming-speech）
- `streaming.ts` 为流式语音前端类型，与 Rust 后端契约对齐（见 [[data-flow-and-types]]）

### 状态管理
- `models` store：模型列表、设备检测、install/uninstall、feature 检测（`supportsFeature`）、设备/语言访问器（`getSupportedDevices` / `getSupportedLanguages`，空时回退全部）。`normalizeModelInfo` 将 `currentDevice` 归一为合法且 ∈ supportedDevices 的 HardwareType，否则 null；`setCurrentDevice(modelId, device)` invoke `set_model_current_device` 写库并即时替换本地条目。`installModel`/`reinstallModel` 需显式 device 参数（不再默认 Cpu），ModelManageView 传当前模型的 `currentDevice`，未选设备时安装按钮禁用。
- `speakers` store：说话人 CRUD、模型导入为说话人
- `streamingSpeech` store：流式语音会话状态--本地说话人列表（语音克隆式，不接入 speakers）、聊天消息、会话配置、抽屉开关。`sendMessage` 首条消息 `invoke('create_streaming_speech_task')` 拿 taskId 回填，后续 `invoke('send_streaming_message', { onEvent: Channel })` 接收流式事件；取消 `invoke('cancel_streaming_task')`。防连点/回车连击产生僵尸会话。流式接收交 `StreamableAudioPlayer(mode='stream')`。
- `taskPreferences` store：当前任务的偏好设置（如固定基础模型）
- `uiConfig` store：管理从后端动态加载的 UI 参数配置

### 模型语言灵活配置
- `AppLanguage` 枚举（`enums/language.ts`）含 4 值：chinese / english / japanese / korean。
- `ModelInfo`（`types/domain.ts`）含 `supportedLanguages: AppLanguage[]`；`models` store 提供 `getSupportedLanguages(baseModel, modelVersion)` 访问器。
- 4 个任务 View 的"输出语言"`BaseListbox` 用 computed `languageOptions`，watcher 在模型切换时重置。
- 说话人侧无语言字段（`SpeakerProfile.languages` 已移除）。

### 服务端分页
`list_model_infos` / `list_speaker_infos` / `list_history_records` 为分页接口，统一通过 `usePagination<TItem, TFilter>(options)` hook 封装。`list_history_records` 仅返回 `HistoryRecordSummary`，业务页结果卡通过 `loadRecentHistoryRecords` 逐条 `get_history_record` 懒加载完整 detail。

### 系统睡眠/锁屏恢复 + 安装失败状态
- **`usePollingResume(onResume)`**：注册 `visibilitychange` + `pageshow` 监听，页面重新可见时立即触发追赶刷新。背景：WebView2 在系统睡眠/锁屏时会节流或挂起 `setInterval`，导致运行中任务状态轮询不更新、await 的 `invoke` 被冻结占用标志长期为 true，唤醒后卡死。5 个 View 接入（4 任务页 + ModelManageView）。
- **`ModelInstallStatus`**：前端会话级状态 `installed` / `not-installed` / `failed`。`models` store `failedModelIds: Set<number>` 追踪安装失败。详见 [[data-flow-and-types]]。

### 说话人字段
`SpeakerProfile.speakerName`（对齐后端 `SpeakerInfo.speaker_name`）。create/update/import 三处 `invoke` payload key 均为 `speakerName`。

### 流式语音生成
前端 ChatUI + 后端会话级长期进程的实时流式语音合成。后端架构见 [[streaming-speech-architecture]]。

- `StreamingSpeechView`：ChatUI 风格对话页，user/assistant 气泡 + 底部说话人选择与文本输入（Enter 发送 / Shift+Enter 换行）。assistant 消息挂 `StreamableAudioPlayer(mode='stream')`，借其 `watch immediate` 契约自动 `startStreaming(taskId, contextId)`，经 `send_streaming_message` 的 `ipc::Channel` 接收 `AudioStreamEvent`（started/chunk/finished/error）累计播放。
- `StreamingConfigDrawer`：右侧可拖拽抽屉（左边缘 pointer 事件调宽 320–560px，默认收起按需唤起，带半透明遮罩），三段 `PanelCard`：基础配置（模型/版本/设备/语言，镜像 `VoiceCloneView` 的 watch 同步）/ 说话人管理 / 模型参数（`GenericTaskParamsForm`）。`StreamingSpeakerForm` 基于 `BaseDialog`，字段为名称/对应模型/参考音频/参考文本（按模型可选）/类别只读。
- 说话人为**前端本地语音克隆式**定义（名称+参考音频+参考文本+类别），不接入 `speakerStore`；`StreamingSpeakerCategory` 当前固定 `voice-clone`，预留 `preset`/`trained`。时间字段后端生成、前端只读（见 [[time-field-naming-rule]]）。
- `requiresRefText(baseModel)` 复用 `DYNAMIC_REFERENCE_BASE_MODELS` 模式（`gpt_sovits_cpufast`）判断参考文本是否必填。
- `streamingSpeech` store：首条消息 `invoke create_streaming_speech_task` 拿 `StreamingSpeechTaskResult`（taskId + 三个路径）回填，后续 `invoke send_streaming_message`（payload 含 taskId/contextId/speakerName/text，附 `onEvent: Channel`）；`cancel_streaming_task` 终止会话。

### 路由
- 默认路由 `/` 重定向到 `/model-training`
- 10 个主路由对应 10 个 View（含 `/streaming-speech`）；`HistoryTaskType` 枚举已同步 `StreamingSpeech = 'streaming-speech'`，`HISTORY_TASK_ROUTE_PATH` / `HISTORY_TASK_TYPE_TEXT` 一并补齐（见 [[history-task-type-sync-rule]]）

## 关联记忆
- [[project-overview]]
- [[streaming-speech-architecture]]
- [[tech-stack-backend]]
- [[data-flow-and-types]]
- [[model-adapter-pattern]]
- [[time-field-naming-rule]]
