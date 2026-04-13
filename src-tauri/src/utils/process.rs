use std::{fs::OpenOptions, io::Write, path::Path};

use anyhow::{bail, Context};
use tokio::process::Command;
use tracing::{error, info};

use crate::utils::file_ops::ensure_parent_dir;
use crate::Result;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

fn prepare_command(program: &Path, args: &[String], current_dir: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(current_dir)
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONLEGACYWINDOWSSTDIO", "0");

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);

    command
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

    bail!(
        "process exited with status {}. Task log: {}",
        status,
        task_log_path.display(),
    )
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
