---
name: history-task-type-sync-rule
description: 前端 HistoryTaskType 枚举必须与 Rust 端 HistoryTaskType 枚举逐一对齐，新增任务类型时两侧同步
metadata: 
  node_type: memory
  type: project
  originSessionId: 753fed4d-4810-4c73-a17c-c0b0847ab2c4
---

前端 `src/enums/task.ts` 的 `HistoryTaskType` 枚举必须与 Rust 端 `src-tauri/src/service/models.rs` 的 `HistoryTaskType` 枚举逐一对齐：相同的变体、相同的 kebab-case 字符串值。在 Rust 端新增任务类型时，前端必须同步：

1. 在 `src/enums/task.ts` 的 `HistoryTaskType` 枚举追加变体；
2. 更新同文件两个 `Record<HistoryTaskType, string>`（TypeScript 强制穷尽，漏一个即编译报错）：
   - `HISTORY_TASK_ROUTE_PATH` -- 该任务页路由（如 streaming 用 `/streaming-speech`，注意路由表 `src/routers/index.ts` 中 streaming 是硬编码路径而非取自该 Record）；
   - `HISTORY_TASK_TYPE_TEXT` -- 历史列表/详情展示用的中文标签（如 `流式语音`）。

**Why:** 前端模型筛选 `modelStore.getModelsByFeature(HistoryTaskType)`、参数配置 `uiConfigStore.getTaskConfig(baseModel, HistoryTaskType)`、历史展示 `HISTORY_TASK_TYPE_TEXT[row.taskType]`、历史详情跳转 `HISTORY_TASK_ROUTE_PATH[record.taskType]` 全部以**前端枚举**为键。Rust 端已存在而前端缺失时，前端无法引用该类型，只能“借用”邻近类型。流式语音即典型反例：Rust 端有 `HistoryTaskType::StreamingSpeech`（`"streaming-speech"`，`storage_dir="streaming"`，写入统一历史表、有 `load_streaming_detail`），前端枚举曾缺该变体，导致 `StreamingConfigDrawer`/`StreamingSpeakerForm` 借用 `HistoryTaskType.VoiceClone` 筛模型与读参数表单，且历史列表中流式任务标签为空。当前前端枚举与两个 Record 已补齐，模型列表与参数表单均用 `StreamingSpeech`。注意 gpt_sovits_cpufast 虽是流式 pipeline 的目标模型，但因暂无可用 `streaming.py`，未在 `model-config.json` 声明 `streaming-speech` 特性--故当前流式页模型列表/参数表单为空，待真正支持流式的模型出现再补 feature + params-config。

**How to apply:** 在 Rust `HistoryTaskType` 增变体时，立即在前端 `src/enums/task.ts` 同步枚举 + 两个 Record。只在模型**真正支持**某 feature 时，才在 `src-model/<base_model>/configs/model-config.json` 的 `supportedFeatureList` 声明该字符串；不要为了「让下拉有选项」而借用其它任务类型（如 VoiceClone）的配置去填充尚未实现的特性，那会污染原逻辑（见 [[dont-borrow-feature-config]]）。相关数据流见 [[data-flow-and-types]]，前端结构见 [[tech-stack-frontend]]。
