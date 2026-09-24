---
name: remote-api-yaml-sync-rule
description: 后端接口（client::ApiClient 远端方法）发生任何改动时，必须同步更新 docs/remote-api.yaml
metadata:
  type: project
---

涉及后端接口的任何改动时，必须同步更新 `docs/remote-api.yaml`（OpenAPI 3.0.3 契约文档）。属于「接口改动」的情形包括：

- `client::ApiClient` 新增 / 删除 / 重命名远端方法（paths 下对应 path + operationId 增删改）
- 请求 / 响应 payload 的字段增删改、类型变化（components/schemas 及对应 Rust 结构体 `#[serde(rename_all = "camelCase")]` 字段）
- 统一信封 `CommonResponse<T>`、分页 `PageRequest` / `PageResponse<T>` 结构变化
- 鉴权方式、URL 前缀、接口路径等契约层面的调整

**Why:** `docs/remote-api.yaml` 是远端存储模式（`RemoteService`）HTTP 接口的契约来源真值，与 `client::ApiClient` 的远端方法一一对应，并与前端 `src/types/domain.ts` 的 camelCase 字段对齐。该文件无自动生成机制，纯手工维护；若只改 Rust 代码不改 yaml，契约文档会立即失真，误导后续 Remote 模式开发与联调。

**How to apply:** 修改 `client::ApiClient` 方法或其涉及的 payload/响应结构体时，在同一变更中同步更新 `docs/remote-api.yaml` 的对应 path、operationId 与 schema；`info.version` 随版本号一并更新。yaml 中的字段名必须与 Rust 端 serde 重命名后的 camelCase 一致。相关后端结构见 [[tech-stack-backend]]、类型对齐见 [[data-flow-and-types]]。
