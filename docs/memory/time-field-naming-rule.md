---
name: time-field-naming-rule
description: 实体时间字段一律用 createTime/modifyTime，类型 string
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 75728d0d-9320-44ab-b56d-7cf250a58e70
---

新建任何实体类型时，时间字段一律使用 `createTime` 与 `modifyTime`（驼峰），类型为 `string`，与 `SpeakerProfile` / `ModelInfo` 等既有实体保持统一。不要用 `createdAt: number` 之类的时间戳写法。

**Why:** 用户明确强调这是项目统一规范，前后端实体命名需一致；曾因在流式语音页面计划中使用 `createdAt: number` 被纠正。

**How to apply:** 定义接口/类型时，时间字段写 `createTime` / `modifyTime`（类型 `string`，可设可选）；**时间由后端在任务执行/实体持久化前生成，前端只读、不主动赋值、不向后端 payload 提供**。纯前端本地实体时间字段留空，UI 不展示或占位。关联 [[db-schema-sync-rule]]（同属数据规范族）。
