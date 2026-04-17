use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::service::pipeline::model_paths::llm_model_paths;

const SRC_MODEL_PYTHON_ROOT_DIR: &str = "src";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScriptPlatform {
    Windows,
    Unix,
}

impl ScriptPlatform {
    pub(crate) const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else {
            Self::Unix
        }
    }

    pub(crate) const fn init_task_runtime_relative_path(self) -> &'static str {
        match self {
            Self::Windows => "scripts/windows/init_task_runtime.ps1",
            Self::Unix => "scripts/unix/init_task_runtime.sh",
        }
    }

    pub(crate) const fn download_models_relative_path(self) -> &'static str {
        match self {
            Self::Windows => "scripts/windows/download_models.ps1",
            Self::Unix => "scripts/unix/download_models.sh",
        }
    }

    pub(crate) const fn lora_toggle_relative_path(self) -> &'static str {
        match self {
            Self::Windows => "scripts/windows/toggle_lora_dependencies.ps1",
            Self::Unix => "scripts/unix/toggle_lora_dependencies.sh",
        }
    }

    pub(crate) const fn venv_python_relative_path(self) -> &'static str {
        match self {
            Self::Windows => "venv/Scripts/python.exe",
            Self::Unix => "venv/bin/python",
        }
    }

    pub(crate) const fn shell_program(self) -> &'static str {
        match self {
            Self::Windows => "powershell.exe",
            Self::Unix => "sh",
        }
    }

    pub(crate) fn shell_args(self, script_path: &Path) -> Vec<String> {
        match self {
            Self::Windows => vec![
                "-NoProfile".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
                "-File".to_string(),
                script_path.to_string_lossy().to_string(),
            ],
            Self::Unix => vec![script_path.to_string_lossy().to_string()],
        }
    }
}

pub(crate) fn src_model_shared_python_script_path(
    src_model_root: &Path,
    script_name: &str,
) -> PathBuf {
    src_model_root
        .join(SRC_MODEL_PYTHON_ROOT_DIR)
        .join(script_name)
}

pub(crate) fn src_model_model_python_script_path(
    src_model_root: &Path,
    base_model: &str,
    script_name: &str,
) -> Result<PathBuf> {
    Ok(llm_model_paths(base_model)?.python_script_path(src_model_root, script_name))
}

pub(crate) fn src_model_venv_python_path(src_model_root: &Path) -> PathBuf {
    src_model_root.join(ScriptPlatform::current().venv_python_relative_path())
}

pub(crate) fn src_model_lora_toggle_script_path(src_model_root: &Path) -> PathBuf {
    src_model_root.join(ScriptPlatform::current().lora_toggle_relative_path())
}

pub(crate) fn resolve_src_model_root(app_dir: &Path) -> Result<PathBuf> {
    let workspace_root = app_dir
        .parent()
        .context("failed to resolve workspace root from src-tauri directory")?;
    let candidates = [
        workspace_root.join("src-model"),
        app_dir.join("src-model"),
        app_dir.join("lib").join("src-model"),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "src-model runtime not found in any expected location under {}",
        app_dir.display()
    )
}
