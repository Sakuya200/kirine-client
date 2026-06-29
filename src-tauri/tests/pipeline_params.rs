//! 规则4：任务执行 hooks（4 个 `create_*_task`）不真正执行任务，仅验证调用模型层脚本
//! 时的参数契约。
//!
//! 本文件覆盖 **CLI 参数向量** —— `build_llm_task_script_args`（从 `pipeline/mod.rs`
//! 抽取的纯函数，经 `test_support` 重导出）。该向量是 Rust 侧传给 `begin_llm_task`
//! 包装器脚本的参数，决定脚本如何定位 python、params 文件与 task 日志。
//!
//! 「params 文件内容」（`PythonScriptInvocationSpec` 写入的 JSON，即模型层 python 脚本
//! 实际接收的参数）的覆盖位于 `src/service/pipeline/api/mod.rs` 的内部 `#[cfg(test)]`
//! 模块（该 spec 类型为 `pub(crate)`，只能内部测试）。

use std::path::Path;

use kirine_client_lib::test_support::build_llm_task_script_args;

#[test]
fn build_llm_task_script_args_has_expected_order_and_values() {
    let script_path = Path::new("/src-model/qwen3_tts/tts.py");
    let params_path = Path::new("/data/params/42.json");
    let log_path = Path::new("/data/logs/task-42.log");
    let base_model = "qwen3_tts";

    let args = build_llm_task_script_args(script_path, params_path, log_path, base_model);

    // 10 个元素：5 个 flag + 5 个 value
    assert_eq!(args.len(), 10);

    let expected = [
        "--base-model",
        base_model,
        "--script-path",
        &script_path.to_string_lossy().to_string(),
        "--params-file",
        &params_path.to_string_lossy().to_string(),
        "--log-path",
        &log_path.to_string_lossy().to_string(),
        "--task-log-file",
        &log_path.to_string_lossy().to_string(),
    ];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(&args[i], want, "arg[{i}] mismatch");
    }
}

#[test]
fn build_llm_task_script_args_log_path_equals_task_log_file() {
    // 包装器脚本：--log-path 与 --task-log-file 都指向同一个 task 日志文件
    // （脚本侧负责不把它们转发给 python，见 shell_scripts.rs 的脚本逻辑测试）。
    let args = build_llm_task_script_args(
        Path::new("/s.py"),
        Path::new("/p.json"),
        Path::new("/l.log"),
        "vox_cpm2",
    );

    let log_path = locate_value(&args, "--log-path");
    let task_log_file = locate_value(&args, "--task-log-file");
    assert_eq!(log_path, Some("/l.log"));
    assert_eq!(task_log_file, Some("/l.log"));
    assert_eq!(log_path, task_log_file);
}

#[test]
fn build_llm_task_script_args_forwards_params_file_and_base_model() {
    let args = build_llm_task_script_args(
        Path::new("/s.py"),
        Path::new("/p.json"),
        Path::new("/l.log"),
        "irodori_tts_v3",
    );

    assert_eq!(locate_value(&args, "--base-model"), Some("irodori_tts_v3"));
    assert_eq!(locate_value(&args, "--params-file"), Some("/p.json"));
    assert_eq!(locate_value(&args, "--script-path"), Some("/s.py"));
}

fn locate_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let idx = args.iter().position(|a| a == flag)?;
    args.get(idx + 1).map(String::as_str)
}
