use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};

use crate::{config::SRC_MODEL_DIR_RELATIVE_PATHS, Result};

pub const MODEL_CONFIG_FILE_NAME: &str = "model-config.json";
pub const PARAMS_CONFIG_FILE_NAME: &str = "params-config.json";

pub fn resolve_src_model_root_dir() -> Result<PathBuf> {
    for src_model_relative_path in SRC_MODEL_DIR_RELATIVE_PATHS {
        let candidate_path = Path::new(src_model_relative_path);
        if candidate_path.exists() && candidate_path.is_dir() {
            return Ok(candidate_path.to_path_buf());
        }
    }

    bail!(
        "未找到 src-model 目录，请检查路径候选: {}",
        SRC_MODEL_DIR_RELATIVE_PATHS.join(", ")
    )
}

pub fn discover_model_config_file_paths(file_name: &str) -> Result<Vec<PathBuf>> {
    let src_model_root = resolve_src_model_root_dir()?;
    let mut model_dirs: Vec<PathBuf> = fs::read_dir(&src_model_root)
        .with_context(|| format!("读取 src-model 目录失败: {}", src_model_root.display()))?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.is_dir())
        .collect();

    model_dirs.sort();

    let mut paths = Vec::new();

    for model_dir in model_dirs {
        let candidate = model_dir.join("configs").join(file_name);
        if candidate.is_file() {
            paths.push(candidate);
        }
    }

    paths.sort();

    if paths.is_empty() {
        bail!("未发现配置文件 {}", file_name);
    }

    Ok(paths)
}
