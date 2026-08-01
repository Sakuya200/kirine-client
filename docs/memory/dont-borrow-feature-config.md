---
name: dont-borrow-feature-config
description: 某特性配置未就绪时不要借用其它特性的实现去填充，保持各特性代码路径独立干净
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 753fed4d-4810-4c73-a17c-c0b0847ab2c4
---

当某项特性（feature）的配置/数据尚未就绪（如对应的 params-config 条目、模型 feature 声明、脚本实现还没有）时，不要为了「让 UI 有选项/非空」而借用其它相近特性的实现去填充--例如不要让流式语音的参数表单去读 `VoiceClone` 的 params-config，也不要给暂无 `streaming.py` 的模型声明 `streaming-speech` 特性。

**Why:** 用户明确指出「不需要刻意保留声音克隆的参数表单，后面支持相关功能的模型肯定会有，这样会污染原来的代码逻辑」。借用会引入跨特性的隐式耦合，让两条本应独立的代码路径互相依赖，后续真正实现该特性时还要回头清理；保持空/等待真实实现更干净。

**How to apply:** 各任务类型/特性的模型筛选（`getModelsByFeature`）、参数表单（`getTaskConfig`）都应使用**自身**的 `HistoryTaskType`/feature，即便当前返回空。只在模型/config 真正支持时才声明 feature 或添加 params-config 条目。关联 [[history-task-type-sync-rule]]。
