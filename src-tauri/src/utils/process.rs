use std::{fs::OpenOptions, io::Write, path::Path, process::Stdio};

use anyhow::{bail, Context};
use tokio::{process::Command, sync::watch, time::{timeout, Duration}};
use tracing::{error, info};

use crate::utils::file_ops::ensure_parent_dir;
use crate::Result;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

const MAX_ERROR_SNIPPET_CHARS: usize = 2000;
const GRACEFUL_TERMINATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedCommandResult {
    Completed,
    Cancelled,
}

fn prepare_command(program: &Path, args: &[String], current_dir: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(current_dir)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0")
        .env("PYTHONFAULTHANDLER", "1")
        .env("PYTHONUNBUFFERED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
}

fn prepare_cancellable_command(program: &Path, args: &[String], current_dir: &Path) -> Command {
    let mut command = prepare_command(program, args, current_dir);

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);

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

fn initialize_task_log(task_log_path: &Path) -> crate::Result<()> {
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
    Ok(())
}

fn append_process_output(task_log_path: &Path, stdout: &[u8], stderr: &[u8]) -> crate::Result<()> {
    if stdout.is_empty() && stderr.is_empty() {
        return Ok(());
    }

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

    if !stdout.is_empty() {
        file.write_all(stdout).with_context(|| {
            format!(
                "failed to write subprocess stdout into task log file: {}",
                task_log_path.display()
            )
        })?;
    }

    if !stderr.is_empty() {
        file.write_all(stderr).with_context(|| {
            format!(
                "failed to write subprocess stderr into task log file: {}",
                task_log_path.display()
            )
        })?;
    }

    Ok(())
}

#[cfg(unix)]
fn request_process_termination(process_id: u32) -> std::io::Result<()> {
    let status = std::process::Command::new("kill")
        .args(["-TERM", &process_id.to_string()])
        .status()?;
    if status.success() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "failed to send SIGTERM to process {}",
        process_id
    )))
}

#[cfg(not(any(unix, windows)))]
fn request_process_termination(process_id: u32) -> std::io::Result<()> {
    let _ = process_id;
    Err(std::io::Error::other(
        "graceful process termination is not implemented on this platform",
    ))
}

#[cfg(windows)]
fn request_process_termination(process_id: u32) -> std::io::Result<()> {
    use windows_sys::Win32::System::Console::{
        AttachConsole, FreeConsole, GenerateConsoleCtrlEvent, SetConsoleCtrlHandler,
        CTRL_BREAK_EVENT,
    };

    unsafe {
        FreeConsole();
        if AttachConsole(process_id) == 0 {
            return Err(std::io::Error::last_os_error());
        }
        if SetConsoleCtrlHandler(None, 1) == 0 {
            let err = std::io::Error::last_os_error();
            FreeConsole();
            return Err(err);
        }
        if GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_id) == 0 {
            let err = std::io::Error::last_os_error();
            SetConsoleCtrlHandler(None, 0);
            FreeConsole();
            return Err(err);
        }
        SetConsoleCtrlHandler(None, 0);
        FreeConsole();
    }

    Ok(())
}

#[cfg(unix)]
fn force_terminate_process(process_id: u32) -> std::io::Result<()> {
    let status = std::process::Command::new("kill")
        .args(["-KILL", &process_id.to_string()])
        .status()?;
    if status.success() {
        return Ok(());
    }

    Err(std::io::Error::other(format!(
        "failed to send SIGKILL to process {}",
        process_id
    )))
}

#[cfg(not(any(unix, windows)))]
fn force_terminate_process(process_id: u32) -> std::io::Result<()> {
    let _ = process_id;
    Err(std::io::Error::other(
        "forceful process termination is not implemented on this platform",
    ))
}

#[cfg(windows)]
fn force_terminate_process(process_id: u32) -> std::io::Result<()> {
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE},
    };

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, process_id);
        if handle.is_null() {
            return Err(std::io::Error::last_os_error());
        }

        let result = TerminateProcess(handle, 1);
        let status = if result == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        };
        CloseHandle(handle);
        status
    }
}

pub async fn run_logged_command(
    program: &Path,
    args: &[String],
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    success_message: &str,
) -> Result<()> {
    initialize_task_log(task_log_path)?;

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

    append_process_output(task_log_path, &output.stdout, &output.stderr)?;

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

pub async fn run_logged_command_cancellable(
    program: &Path,
    args: &[String],
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    success_message: &str,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    initialize_task_log(task_log_path)?;

    if *cancel_rx.borrow() {
        info!(command = label, "command cancelled before process spawn");
        return Ok(LoggedCommandResult::Cancelled);
    }

    let child = prepare_cancellable_command(program, args, current_dir)
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn `{}` with program {} in {}",
                label,
                program.display(),
                current_dir.display()
            )
        })?;
    let process_id = child.id();
    let wait_with_output = child.wait_with_output();
    tokio::pin!(wait_with_output);

    let mut cancellation_requested = false;

    let output = loop {
        if cancellation_requested {
            match timeout(GRACEFUL_TERMINATION_TIMEOUT, &mut wait_with_output).await {
                Ok(output) => break output.with_context(|| {
                    format!(
                        "failed while waiting for cancelled `{}` with program {} in {}",
                        label,
                        program.display(),
                        current_dir.display()
                    )
                })?,
                Err(_) => {
                    if let Some(process_id) = process_id {
                        if let Err(err) = force_terminate_process(process_id) {
                            error!(command = label, process_id, error = %err, "failed to force terminate cancelled process");
                        }
                    }
                    break wait_with_output.await.with_context(|| {
                        format!(
                            "failed while force-waiting for cancelled `{}` with program {} in {}",
                            label,
                            program.display(),
                            current_dir.display()
                        )
                    })?;
                }
            }
        }

        tokio::select! {
            output = &mut wait_with_output => {
                break output.with_context(|| {
                    format!(
                        "failed to wait for `{}` with program {} in {}",
                        label,
                        program.display(),
                        current_dir.display()
                    )
                })?;
            }
            changed = cancel_rx.changed() => {
                if changed.is_ok() && *cancel_rx.borrow() {
                    cancellation_requested = true;
                    if let Some(process_id) = process_id {
                        if let Err(err) = request_process_termination(process_id) {
                            error!(command = label, process_id, error = %err, "failed to request graceful termination for cancelled process");
                        }
                    }
                }
            }
        }
    };

    append_process_output(task_log_path, &output.stdout, &output.stderr)?;

    if cancellation_requested {
        info!(command = label, message = "command cancelled", "command completed");
        return Ok(LoggedCommandResult::Cancelled);
    }

    let status = output.status;
    if status.success() {
        info!(command = label, message = success_message, "command completed");
        return Ok(LoggedCommandResult::Completed);
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

pub async fn run_logged_python_script_cancellable(
    python_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    success_message: &str,
    script_args: Vec<String>,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    let mut args = vec![
        "-X".to_string(),
        "utf8".to_string(),
        "-X".to_string(),
        "faulthandler".to_string(),
        "-u".to_string(),
        script_path.to_string_lossy().to_string(),
    ];
    args.extend(script_args);

    run_logged_command_cancellable(
        python_path,
        &args,
        current_dir,
        label,
        task_log_path,
        success_message,
        cancel_rx,
    )
    .await
}
