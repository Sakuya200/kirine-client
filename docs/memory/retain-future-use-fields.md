---
name: retain-future-use-fields
description: 不要为消除 dead-code 警告而移除「预留待用」字段（如 ResolvedStreamingPaths 的 base_model/model_version/sample_root）
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 753fed4d-4810-4c73-a17c-c0b0847ab2c4
---

在流式语音流水线实现中，我移除了 `ResolvedStreamingPaths` 的 `base_model`/`model_version`/`sample_root` 三个未被读取的字段以消除 dead-code 警告，被用户明确拒绝。

**Why:** 这三个字段是预留待用的——`base_model` 与 `model_version` 未来都要传递给脚本侧（streaming.py / begin_llm_task），`sample_root` 亦属会话路径上下文。用户认为「未来都要传递给脚本侧，不能擅自移除」。为消除编译器警告而删掉有明确未来用途的字段，属于擅自缩减设计契约。

**How to apply:** 遇到「当前未读但有明确未来用途」的字段/参数/占位，保留它们；若 dead-code 警告碍眼，用 `#[allow(dead_code)]` 加注释说明预留原因，而不是删除。区分「真正可删的死代码」与「预留待用的契约字段」——后者即使当前无读引用也不能动。关联 [[db-schema-sync-rule]]（同为不可擅自缩减的契约）。
