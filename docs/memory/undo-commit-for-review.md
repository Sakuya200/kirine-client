---
name: undo-commit-for-review
description: 执行计划开发的代码 commit 后须撤回到 commit 前状态，交由开发者检查后再提交
metadata:
  node_type: memory
  type: feedback
  originSessionId: acf18693-44d2-40c6-b842-17a79f6c62ba
  modified: 2026-11-05T14:18:41.101Z
---

执行已批准计划的开发任务时，每批代码完成并 commit 后，立即用 `git reset --soft HEAD~1`（默认保留改动于暂存区）撤回该 commit，使改动以未提交状态呈现，方便开发者检查 diff；开发者确认后自行提交，或明确要求后再重新提交。

**Why:** 开发者希望对计划执行产出的每一批代码做人工 diff 审查，commit 直接落入历史会跳过这层检查。

**How to apply:** 仅适用于按计划/批次执行产出的 commit；用户主动要求的提交（发布、PR 等）不在此列。撤回一律用 `git reset --soft HEAD~1`，绝不用 `--hard`（避免丢失改动）。
