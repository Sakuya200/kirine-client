//! 规则3：命令行脚本逻辑测试，仅测当前平台支持的脚本（`#[cfg(windows)]` 测 `.ps1`，
//! `#[cfg(unix)]` 测 `.sh`）。
//!
//! 安全策略：所有断言均落在脚本的**参数解析 / 前置校验**阶段——这些路径在脚本计算
//! `model_root`、探测 python 或调用 ffmpeg **之前**即以非零退出码终止。因此直接运行仓库
//! 真实脚本不会触达真实模型运行时或文件系统写入，无需桩 python、无需真实 venv/conda/
//! torch/ffmpeg，也不污染真实 src-model。
//!
//! 退出码说明：脚本内对参数错误的处理用 `Write-Error`（Windows）配合 `$ErrorActionPreference
//! = 'Stop'`，或 `echo >&2` 后 `exit`（Unix）。Windows 下 `Write-Error` 在 Stop 模式会抛出
//! 终止性错误，使紧跟其后的 `exit 64` 成为死代码——实际退出码为 1。因此本文件**不**断言
//! 具体的 64/66，而是断言「非法输入被非零退出码拒绝」这一稳健契约（同时验证合法输入下
//! `--task-log-file` 父目录被创建的正向行为）。

use std::path::{Path, PathBuf};
use tokio::process::Command;

/// 仓库根下的 `src-model/scripts/<platform>/` 目录。
fn scripts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src-model")
        .join("scripts")
}

/// 断言脚本以非零退出码拒绝输入（参数错误 / 校验失败）。
fn assert_rejected(code: Option<i32>) {
    assert!(
        matches!(code, Some(c) if c != 0),
        "expected non-zero exit code (arg/validation rejection), got {code:?}"
    );
}

#[cfg(windows)]
mod windows_scripts {
    use super::*;

    fn ps1(name: &str) -> PathBuf {
        scripts_dir().join("windows").join(name)
    }

    async fn run_ps1(script: &Path, user_args: &[&str]) -> Option<i32> {
        Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(script)
            .args(user_args)
            .output()
            .await
            .ok()?
            .status
            .code()
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_when_base_model_missing() {
        let script = ps1("begin_llm_task.ps1");
        let log = temp_log_path("begin-no-base");
        let code = run_ps1(
            &script,
            &[
                "--script-path",
                &script.to_string_lossy(),
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_when_script_path_missing() {
        let script = ps1("begin_llm_task.ps1");
        let log = temp_log_path("begin-no-script");
        let code = run_ps1(
            &script,
            &["--base-model", "qwen3_tts", "--task-log-file", &log],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_when_task_log_file_missing() {
        let script = ps1("begin_llm_task.ps1");
        let code = run_ps1(
            &script,
            &[
                "--base-model",
                "qwen3_tts",
                "--script-path",
                &script.to_string_lossy(),
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_unknown_argument() {
        let script = ps1("begin_llm_task.ps1");
        let log = temp_log_path("begin-unknown");
        let code = run_ps1(
            &script,
            &[
                "--base-model",
                "qwen3_tts",
                "--script-path",
                &script.to_string_lossy(),
                "--task-log-file",
                &log,
                "--totally-unknown-flag",
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_creates_task_log_file_parent_dir() {
        // Ensure-TaskLogFile 在解析后、python 探测前创建 --task-log-file 的父目录。
        // 使用不存在的 base_model，使脚本在 python 探测阶段即以非零退出（不调用真实 python），
        // 但父目录应已存在。
        let script = ps1("begin_llm_task.ps1");
        let nested =
            std::env::temp_dir().join(format!("kirine-shell-begin-mkdir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&nested);
        let log = nested.join("deep/sub/task.log");

        let _ = run_ps1(
            &script,
            &[
                "--base-model",
                "no_such_model_xyz",
                "--script-path",
                &script.to_string_lossy(),
                "--task-log-file",
                &log.to_string_lossy(),
            ],
        )
        .await;

        assert!(log.parent().expect("log has parent").exists());
        let _ = std::fs::remove_dir_all(&nested);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_when_required_arg_missing() {
        let script = ps1("transcode_audio.ps1");
        let log = temp_log_path("trans-no-format");
        // 缺 --format
        let code = run_ps1(
            &script,
            &[
                "--input-path",
                "x",
                "--output-path",
                "y",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_when_input_missing() {
        let script = ps1("transcode_audio.ps1");
        let log = temp_log_path("trans-no-input");
        let code = run_ps1(
            &script,
            &[
                "--input-path",
                "/does/not/exist.wav",
                "--output-path",
                "out.wav",
                "--format",
                "wav",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_unsupported_format() {
        let script = ps1("transcode_audio.ps1");
        let log = temp_log_path("trans-bad-fmt");
        let code = run_ps1(
            &script,
            &[
                "--input-path",
                "/does/not/exist.wav",
                "--output-path",
                "out.ogg",
                "--format",
                "ogg",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_invalid_sample_rate() {
        let script = ps1("transcode_audio.ps1");
        let log = temp_log_path("trans-bad-sr");
        let code = run_ps1(
            &script,
            &[
                "--input-path",
                "/does/not/exist.wav",
                "--output-path",
                "out.wav",
                "--format",
                "wav",
                "--sample-rate",
                "0",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn init_task_runtime_rejects_when_base_model_missing() {
        let script = ps1("init_task_runtime.ps1");
        let log = temp_log_path("init-no-base");
        let code = run_ps1(&script, &["--task-log-file", &log]).await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn init_task_runtime_rejects_unknown_argument() {
        let script = ps1("init_task_runtime.ps1");
        let log = temp_log_path("init-unknown");
        let code = run_ps1(
            &script,
            &[
                "--base-model",
                "qwen3_tts",
                "--task-log-file",
                &log,
                "--no-such-option",
            ],
        )
        .await;
        assert_rejected(code);
    }
}

#[cfg(unix)]
mod unix_scripts {
    use super::*;

    fn sh(name: &str) -> PathBuf {
        scripts_dir().join("unix").join(name)
    }

    async fn run_sh(script: &Path, user_args: &[&str]) -> Option<i32> {
        Command::new("sh")
            .arg(script)
            .args(user_args)
            .output()
            .await
            .ok()?
            .status
            .code()
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_when_base_model_missing() {
        let script = sh("begin_llm_task.sh");
        let log = temp_log_path("begin-no-base");
        let code = run_sh(
            &script,
            &[
                "--script-path",
                &script.to_string_lossy(),
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_when_task_log_file_missing() {
        let script = sh("begin_llm_task.sh");
        let code = run_sh(
            &script,
            &[
                "--base-model",
                "qwen3_tts",
                "--script-path",
                &script.to_string_lossy(),
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn begin_llm_task_rejects_unknown_argument() {
        let script = sh("begin_llm_task.sh");
        let log = temp_log_path("begin-unknown");
        let code = run_sh(
            &script,
            &[
                "--base-model",
                "qwen3_tts",
                "--script-path",
                &script.to_string_lossy(),
                "--task-log-file",
                &log,
                "--totally-unknown-flag",
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_when_input_missing() {
        let script = sh("transcode_audio.sh");
        let log = temp_log_path("trans-no-input");
        let code = run_sh(
            &script,
            &[
                "--input-path",
                "/does/not/exist.wav",
                "--output-path",
                "out.wav",
                "--format",
                "wav",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn transcode_audio_rejects_unsupported_format() {
        let script = sh("transcode_audio.sh");
        let log = temp_log_path("trans-bad-fmt");
        let code = run_sh(
            &script,
            &[
                "--input-path",
                "/does/not/exist.wav",
                "--output-path",
                "out.ogg",
                "--format",
                "ogg",
                "--task-log-file",
                &log,
            ],
        )
        .await;
        assert_rejected(code);
    }

    #[tokio::test]
    async fn init_task_runtime_rejects_when_base_model_missing() {
        let script = sh("init_task_runtime.sh");
        let log = temp_log_path("init-no-base");
        let code = run_sh(&script, &["--task-log-file", &log]).await;
        assert_rejected(code);
    }
}

fn temp_log_path(label: &str) -> String {
    std::env::temp_dir()
        .join(format!("kirine-shell-{label}-{}.log", std::process::id()))
        .to_string_lossy()
        .to_string()
}
