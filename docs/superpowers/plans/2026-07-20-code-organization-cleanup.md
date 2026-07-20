# 代码组织清理（2026-07-20）

## 问题

`src-tauri` 存在两处组织问题：

1. **业务 CRUD/启动代码暴露在模块文件中**：`service/local/mod.rs` 内有 8 个业务专属方法（4 个 `start_*_inference` + 4 个流式会话方法），它们仅被各自业务文件调用，应下沉到对应业务文件。
2. **测试代码与业务代码混在一起**：仅 `pipeline/streaming.rs`（~142 行）与 `pipeline/api/mod.rs`（~175 行）含内联 `#[cfg(test)] mod tests`，项目约定是 `tests/` 目录（已 1564 行）经 `pub mod test_support` 再导出通道承载。

## Part 1：业务方法从 `mod.rs` 下沉

### 搬移表

| 方法 | 来源 (mod.rs) | 目标文件 |
|---|---|---|
| `start_tts_inference` | 277 | `tts.rs` |
| `start_voice_clone_inference` | 306 | `voice_clone.rs` |
| `start_training` | 339 | `training.rs` |
| `start_voice_design_inference` | 379 | `voice_design.rs` |
| `register_streaming_session` | 521 | `streaming.rs` |
| `streaming_session_extra` | 537 | `streaming.rs` |
| `load_streaming_task_detail` | 549 | `streaming.rs` |
| `start_streaming_session` | 594 | `streaming.rs` |

### 安全性依据

grep 确认每个方法**仅被自身业务文件**（或同组流式方法间）调用：

- `start_tts_inference` ← `tts.rs:137`；`start_voice_clone_inference` ← `voice_clone.rs:148`；`start_training` ← `training.rs:285`；`start_voice_design_inference` ← `voice_design.rs:125`
- 4 个流式方法 ← `streaming.rs:164/165/234` 及组内互调

目标文件已有 `impl LocalService` 块（如 `tts.rs:29`），符合既有模式。调用 `register_active_task_control`/`unregister_active_task_control` 等通用句柄方法（`pub(crate)`，留 mod.rs）不受影响。

### 导入调整

- **tts.rs**：补 `BaseModel`、`resolve_model_task_pipeline`、`TtsPipelineRequest`、`tokio::sync::watch`
- **voice_clone.rs**：补 `BaseModel`、`resolve_model_task_pipeline`、`VoiceClonePipelineRequest`、`watch`
- **training.rs**：补 `BaseModel`、`resolve_model_task_pipeline`、`TrainingPipelineRequest`、`watch`
- **voice_design.rs**：补 `BaseModel`、`resolve_model_task_pipeline`、`VoiceDesignPipelineRequest`、`watch`
- **streaming.rs (local)**：补 `BaseModel`、`HardwareType`、`watch`、`Arc`、`crate::service::pipeline::streaming::{StreamingSessionExtra, LoadedStreamingDetail, StreamingContextJson, run_streaming_session}`、`StreamingPipelineRequest`、`crate::common::local_paths::resolve_task_path`、`crate::service::models::StreamingSpeakerInput`、sea_orm `ColumnTrait/EntityTrait/QueryFilter`（`load_streaming_task_detail` 查询用）
- **mod.rs**：移除搬走后不再使用的导入（`BaseModel`、`resolve_model_task_pipeline`、各 `*PipelineRequest`、`watch` 若不再用）；`ActiveTaskControl.streaming_extra` 仍需 `StreamingSessionExtra` 类型，保留。最终以 `cargo check`/`clippy` 收敛未用导入。

mod.rs 保留：模块声明、imports、`ActiveTaskControl`/`LocalService` 结构体、`impl Service` trait 委托、`from_paths*`/`init_db`/`ui_config`、访问器、通用任务句柄方法（register/unregister/cancel receiver/request cancel）、共享路径工具函数（`copy_model_param_files`/`build_task_title`/`build_sample_file_name`/`build_task_audio_file_name`/`sanitize_path_segment`/`sanitize_file_stem`）。

## Part 2：内联测试迁至 `tests/`（经 `test_support` 再导出）

用户已确认「Move both via test_support」。

### 在 `src/test_support.rs` 增补再导出

`pipeline/mod.rs` 中 `api`/`streaming` 均为 `pub mod`，其内项为 `pub(crate)`，可经 `test_support`（`pub mod`，已用同模式再导出 `build_llm_task_script_args`）`pub use` 转出，无需提升可见性。

- 自 `crate::service::pipeline::streaming`：`parse_streaming_frame`、`frame_to_event`、`serialize_input_entry`、`StreamingFrame`、`StreamingFramePayload`、`StreamingContextJson`、`StreamingContextBasic`、`StreamingSpeaker`、`StreamingMessageEntry`
- 自 `crate::service::pipeline::api`：`PythonScriptInvocationSpec`、`PythonScriptTaskKind`、`PythonScriptTaskArgs`、`PythonScriptRuntimeOptions`、`TTSArgs`、`VoiceCloneArgs`、`VoiceDesignArgs`、`TrainingArgs`、`StreamingArgs`、`StreamingSpeakerArg`

### 测试文件改动

- **新建 `tests/streaming_frames.rs`**：迁入 `streaming.rs` 内联模块全部 9 个用例（parses_started/chunk/finished_and_error、blank_line、malformed、context_json_round_trip、input_entry_single_line、unknown_frame_type、frame_to_event_maps_each_payload），经 `kirine_client_lib::test_support::...` 引用。
- **`tests/pipeline_params.rs`**：追加 `api/mod.rs` 内联模块的 5 个 params 文件内容用例（writes_tts/voice_clone/voice_design/training/streaming_params_file_with_expected_fields），并**更新头部注释 8-10 行**：删除「位于 src/service/pipeline/api/mod.rs 的内部 #[cfg(test)] 模块」指向，改为说明本文件同时覆盖 CLI 参数向量与 params 文件 JSON 内容。
- **删除** `src/service/pipeline/streaming.rs:525-667` 与 `src/service/pipeline/api/mod.rs:152-327` 的 `#[cfg(test)] mod tests` 块。

## 验证

- `cargo check`（src-tauri）
- `cargo clippy --all-targets`（含迁移后测试）
- `cargo test`：确认 14 个迁出用例全绿、既有用例无回归
- 不跑全局 `cargo fmt`（src-tauri HEAD 非 rustfmt-1.8.0 干净，见记忆 `src-tauri-not-rustfmt-clean`）；以 check/clippy/test 兜底

## 约束 / 不在范围

- 前端不在范围：无 `tests/` 设施、无 `.spec.ts`、未发现 barrel 文件类比问题；如需可另起审计。
- 遵守 `retain-future-use-fields`：`ResolvedStreamingPaths` 的 `base_model`/`model_version`/`sample_root` 及 `resolve_streaming_paths` 的 `model_version` 参数本次一律不动。
- 纯代码搬运 + 测试迁移，无行为变更；`test_support` 仅测试通道，非真实公共 API。

## 风险

低。所有搬移经 grep 确认无跨业务调用；再导出沿用既有 `test_support` 模式；行为零变更。
