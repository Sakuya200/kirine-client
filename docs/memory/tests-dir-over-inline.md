---
name: tests-dir-over-inline
description: 项目约定：单元测试放 tests/ 目录经 test_support 桥接，不用内联
metadata: 
  node_type: memory
  type: project
  originSessionId: 1378d90e-c255-492a-a0f2-56f1cceaeb46
---

Kirine Client 的 src-tauri 约定：测试代码放 `src-tauri/tests/` 目录，**不**用源文件内联的 `#[cfg(test)] mod tests`。这与 Rust 内联单测的惯例相反，但是项目既定方向（tests/ 已 ~1700 行，内联仅作为历史残留）。

**Why:** `service` / `config` 等模块在 lib.rs 中为私有，集成测试（外部 crate）无法直接命名其中的 `pub(crate)` 类型与纯函数。`src/test_support.rs`（`pub mod test_support`）作为公共测试桥接，`pub use` 重导出测试所需的 `pub(crate)` 项（如 `build_llm_task_script_args`、`PythonScriptInvocationSpec`、`parse_streaming_frame` 等）。

**How to apply:**
- 新增 `pub(crate)` 代码需测试时，写进 `tests/` 而非内联。
- 被测项若为 `pub(crate)`：先在源文件把它从 `pub(crate)` 提升为 `pub`（仍位于 `pub(crate) mod pipeline` 内，原始路径对外不可达，仅 `test_support` 重导出暴露），再在 `test_support.rs` 加 `pub use crate::service::pipeline::<mod>::{...}`，最后在 `tests/<name>.rs` 经 `kirine_client_lib::test_support::...` 引用。
- `pub use` 重导出 `pub(crate)` 项对外部 crate 不可达；必须先把项本身提到 `pub`（`build_llm_task_script_args` 即此模式：pub 项在 pub(crate) mod 内，经 test_support 重导出供 tests/ 用）。
- 父模块私有结构（如 `ActiveTaskControl`）在子模块中需用 `super::` 限定，不能直接用短名。
- 相关：[[src-tauri-not-rustfmt-clean]]（验证用 cargo check/clippy/test，不跑全局 cargo fmt）。
