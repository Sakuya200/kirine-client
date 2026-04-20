use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::config::{load_configs, resolve_base_log_dir};

const DATA_DIR_PATH_PLACEHOLDER: &str = "%DATA_DIR_PATH%";
const MODEL_DIR_PATH_PLACEHOLDER: &str = "%MODEL_DIR_PATH%";
const SRC_MODEL_ROOT_PATH_PLACEHOLDER: &str = "%SRC_MODEL_ROOT_PATH%";

pub(crate) fn resolve_local_log_dir() -> Result<PathBuf> {
    let config =
        load_configs().context("failed to load config.toml before resolving log directory")?;
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

pub(crate) fn serialize_model_path(model_dir: &Path, path: &Path) -> String {
    serialize_path_with_placeholder(path, model_dir, MODEL_DIR_PATH_PLACEHOLDER)
        .unwrap_or_else(|| normalize_path_string(path))
}

pub(crate) fn serialize_runtime_model_path(
    model_dir: &Path,
    src_model_root: &Path,
    path: &Path,
) -> String {
    serialize_path_with_placeholder(path, model_dir, MODEL_DIR_PATH_PLACEHOLDER)
        .or_else(|| {
            serialize_path_with_placeholder(path, src_model_root, SRC_MODEL_ROOT_PATH_PLACEHOLDER)
        })
        .unwrap_or_else(|| normalize_path_string(path))
}

pub(crate) fn resolve_runtime_model_path(
    model_dir: &Path,
    src_model_root: &Path,
    value: &str,
) -> Result<PathBuf> {
    let trimmed = value.trim();
    let resolved = if trimmed.contains(MODEL_DIR_PATH_PLACEHOLDER) {
        resolve_placeholder_path(trimmed, MODEL_DIR_PATH_PLACEHOLDER, model_dir)
            .unwrap_or_else(|| PathBuf::from(trimmed))
    } else if trimmed.contains(SRC_MODEL_ROOT_PATH_PLACEHOLDER) {
        resolve_placeholder_path(trimmed, SRC_MODEL_ROOT_PATH_PLACEHOLDER, src_model_root)
            .unwrap_or_else(|| PathBuf::from(trimmed))
    } else {
        PathBuf::from(trimmed)
    };

    Ok(resolved)
}

pub(crate) fn src_model_relative_runtime_path(relative_path: &str) -> String {
    let normalized = relative_path.trim().trim_matches('/').replace('\\', "/");
    if normalized.is_empty() {
        SRC_MODEL_ROOT_PATH_PLACEHOLDER.to_string()
    } else {
        format!("{}/{}", SRC_MODEL_ROOT_PATH_PLACEHOLDER, normalized)
    }
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

#[cfg(test)]
mod tests {
    use super::{
        resolve_runtime_model_path, serialize_runtime_model_path, src_model_relative_runtime_path,
    };
    use std::path::Path;

    #[test]
    fn serialize_runtime_model_path_prefers_model_dir_placeholder() {
        let model_dir = Path::new("D:/models");
        let src_model_root = Path::new("D:/workspace/src-model");
        let path = Path::new("D:/models/42_speaker/checkpoint-epoch-9");

        let value = serialize_runtime_model_path(model_dir, src_model_root, path);

        assert_eq!(value, "%MODEL_DIR_PATH%/42_speaker/checkpoint-epoch-9");
    }

    #[test]
    fn serialize_runtime_model_path_supports_src_model_root_placeholder() {
        let model_dir = Path::new("D:/models");
        let src_model_root = Path::new("D:/workspace/src-model");
        let path = Path::new("D:/workspace/src-model/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice");

        let value = serialize_runtime_model_path(model_dir, src_model_root, path);

        assert_eq!(
            value,
            "%SRC_MODEL_ROOT_PATH%/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice"
        );
    }

    #[test]
    fn resolve_runtime_model_path_supports_src_model_root_placeholder() {
        let model_dir = Path::new("D:/models");
        let src_model_root = Path::new("D:/workspace/src-model");

        let path = resolve_runtime_model_path(
            model_dir,
            src_model_root,
            "%SRC_MODEL_ROOT_PATH%/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice",
        )
        .expect("resolve runtime model path");

        assert_eq!(
            path,
            Path::new("D:/workspace/src-model/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice")
        );
    }

    #[test]
    fn src_model_relative_runtime_path_normalizes_separators() {
        let value = src_model_relative_runtime_path("base-models\\Qwen3-TTS-12Hz-1.7B-CustomVoice");

        assert_eq!(
            value,
            "%SRC_MODEL_ROOT_PATH%/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice"
        );
    }

    #[test]
    fn resolve_runtime_model_path_keeps_missing_directory_uncreated() {
        let unique = format!(
            "kirine-runtime-model-path-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time before unix epoch")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(unique);
        let model_dir = root.join("models");
        let src_model_root = root.join("src-model");

        std::fs::create_dir_all(&model_dir).expect("create model dir");
        std::fs::create_dir_all(&src_model_root).expect("create src-model dir");

        let resolved = resolve_runtime_model_path(
            &model_dir,
            &src_model_root,
            "%SRC_MODEL_ROOT_PATH%/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice",
        )
        .expect("resolve runtime model path");

        assert!(!resolved.exists());
        assert_eq!(
            resolved,
            src_model_root
                .join("base-models")
                .join("Qwen3-TTS-12Hz-1.7B-CustomVoice")
        );

        std::fs::remove_dir_all(&root).expect("remove temp root dir");
    }
}
