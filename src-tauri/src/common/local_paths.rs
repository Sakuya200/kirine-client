use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::config::{resolve_base_log_dir, EnvConfig};

const DATA_DIR_PATH_PLACEHOLDER: &str = "%DATA_DIR_PATH%";

pub(crate) fn resolve_local_log_dir(config: &EnvConfig) -> Result<PathBuf> {
    resolve_base_log_dir(config.log_dir())
}

pub(crate) fn ensure_child_dir(root: &Path, child: &str) -> Result<PathBuf> {
    let dir = root.join(child);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create directory: {}", dir.display()))?;
    }
    Ok(dir)
}

pub(crate) fn serialize_task_path(data_dir: &Path, path: &Path) -> String {
    serialize_path_with_placeholder(path, data_dir, DATA_DIR_PATH_PLACEHOLDER)
        .unwrap_or_else(|| normalize_path_string(path))
}

pub(crate) fn resolve_task_path(data_dir: &Path, value: &str) -> PathBuf {
    let trimmed = value.trim();
    // 如果路径中不包含占位符，直接返回原始路径，避免不必要的字符串处理和路径拼接
    if !trimmed.contains(DATA_DIR_PATH_PLACEHOLDER) {
        return PathBuf::from(trimmed);
    }
    resolve_placeholder_path(trimmed, DATA_DIR_PATH_PLACEHOLDER, data_dir)
        .unwrap_or_else(|| PathBuf::from(trimmed))
}

fn serialize_path_with_placeholder(path: &Path, root: &Path, placeholder: &str) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let relative = normalize_relative_path(relative);

    if relative.is_empty() {
        Some(placeholder.to_string())
    } else {
        Some(format!("{}/{}", placeholder, relative))
    }
}

fn resolve_placeholder_path(value: &str, placeholder: &str, root: &Path) -> Option<PathBuf> {
    if value == placeholder {
        return Some(root.to_path_buf());
    }

    let remainder = value.strip_prefix(placeholder)?.trim_start_matches('/');
    if remainder.is_empty() {
        Some(root.to_path_buf())
    } else {
        let mut path = root.to_path_buf();
        for segment in remainder.split('/') {
            if !segment.is_empty() {
                path.push(segment);
            }
        }
        Some(path)
    }
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
