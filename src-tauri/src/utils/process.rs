use std::{fs::OpenOptions, io::Write, path::Path};

use anyhow::{bail, Context};
use tokio::process::Command;
use tracing::{error, info};

use crate::utils::file_ops::ensure_parent_dir;
use crate::Result;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const MAX_ERROR_SNIPPET_CHARS: usize = 2000;

fn prepare_command(program: &Path, args: &[String], current_dir: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(current_dir)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0")
        .env("PYTHONFAULTHANDLER", "1")
        .env("PYTHONUNBUFFERED", "1");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

fn trim_error_snippet(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_ERROR_SNIPPET_CHARS {
        return trimmed.to_string();
    }

    let truncated = trimmed
        .chars()
        .rev()
        .take(MAX_ERROR_SNIPPET_CHARS)
        .collect::<String>()
        .chars()
        .rev()
        .collect::<String>();
    format!("...{}", truncated)
}

fn decode_output(bytes: &[u8]) -> String {
    trim_error_snippet(&String::from_utf8_lossy(bytes))
}

fn read_task_log_tail(task_log_path: &Path) -> String {
    match std::fs::read_to_string(task_log_path) {
        Ok(content) => trim_error_snippet(&content),
        Err(_) => String::new(),
    }
}

fn windows_status_hint(status: std::process::ExitStatus) -> Option<&'static str> {
    #[cfg(windows)]
    {
        if status.code() == Some(0xC0000005_u32 as i32) {
            return Some(
                "Windows access violation (0xC0000005). This usually means a native dependency crashed inside Python, such as torch or onnxruntime.",
            );
        }
    }

    #[cfg(not(windows))]
    {
        let _ = status;
    }

    None
}

fn build_process_failure_message(
    label: &str,
    status: std::process::ExitStatus,
    task_log_path: &Path,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    let stdout_text = decode_output(stdout);
    let stderr_text = decode_output(stderr);
    let task_log_tail = read_task_log_tail(task_log_path);

    let mut message = format!(
        "process `{}` exited with status {}. Task log: {}",
        label,
        status,
        task_log_path.display(),
    );

    if let Some(hint) = windows_status_hint(status) {
        message.push_str(" ");
        message.push_str(hint);
    }

    if !stderr_text.is_empty() {
        message.push_str("\n[stderr]\n");
        message.push_str(&stderr_text);
    }

    if !stdout_text.is_empty() {
        message.push_str("\n[stdout]\n");
        message.push_str(&stdout_text);
    }

    if !task_log_tail.is_empty() {
        message.push_str("\n[task-log-tail]\n");
        message.push_str(&task_log_tail);
    }

    message
}

pub async fn run_logged_command(
    program: &Path,
    args: &[String],
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    success_message: &str,
) -> Result<()> {
    ensure_parent_dir(task_log_path, "task log")?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(task_log_path)
        .with_context(|| {
            format!(
                "failed to initialize task log file: {}",
                task_log_path.display()
            )
        })?;

    let output = prepare_command(program, args, current_dir)
        .output()
        .await
        .with_context(|| {
            format!(
                "failed to spawn `{}` with program {} in {}",
                label,
                program.display(),
                current_dir.display()
            )
        })?;

    if !output.stdout.is_empty() || !output.stderr.is_empty() {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(task_log_path)
            .with_context(|| {
                format!(
                    "failed to append subprocess output into task log file: {}",
                    task_log_path.display()
                )
            })?;

        if !output.stdout.is_empty() {
            file.write_all(&output.stdout).with_context(|| {
                format!(
                    "failed to write subprocess stdout into task log file: {}",
                    task_log_path.display()
                )
            })?;
        }

        if !output.stderr.is_empty() {
            file.write_all(&output.stderr).with_context(|| {
                format!(
                    "failed to write subprocess stderr into task log file: {}",
                    task_log_path.display()
                )
            })?;
        }
    }

    let status = output.status;

    if status.success() {
        info!(
            command = label,
            message = success_message,
            "command completed"
        );
        return Ok(());
    }

    error!(command = label, log_path = %task_log_path.display(), status = %status, "command failed");

    bail!(build_process_failure_message(
        label,
        status,
        task_log_path,
        &output.stdout,
        &output.stderr,
    ))
}

pub async fn run_logged_python_script(
    python_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    success_message: &str,
    script_args: Vec<String>,
) -> Result<()> {
    let mut args = vec![
        "-X".to_string(),
        "utf8".to_string(),
        "-X".to_string(),
        "faulthandler".to_string(),
        "-u".to_string(),
        script_path.to_string_lossy().to_string(),
    ];
    args.extend(script_args);

    run_logged_command(
        python_path,
        &args,
        current_dir,
        label,
        task_log_path,
        success_message,
    )
    .await
}
