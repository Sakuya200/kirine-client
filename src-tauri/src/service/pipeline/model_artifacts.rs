use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde_json::to_string;

use crate::{
    service::{
        models::{ModelDownloadType, ModelInfo},
        pipeline::script_paths::src_model_model_python_script_path,
    },
    Result,
};

pub(crate) const MODEL_ARTIFACTS_DIR: &str = "base-models";

pub(crate) fn build_model_download_script_args(
    src_model_root: &Path,
    model_info: &ModelInfo,
) -> Result<Vec<String>> {
    if model_info.download_type != ModelDownloadType::HfLike {
        bail!(
            "模型 {}:{} 的下载方式为 {}，不能走 HF-like 下载参数构造",
            model_info.base_model,
            model_info.model_version,
            model_info.download_type
        );
    }

    if model_info.required_model_name_list.is_empty() {
        bail!(
            "模型 {}:{} 未配置 required_model_name_list",
            model_info.base_model,
            model_info.model_version
        );
    }

    if model_info.required_model_name_list.len() != model_info.required_model_repo_id_list.len() {
        bail!(
            "模型 {}:{} 的 required_model_name_list 与 required_model_repo_id_list 长度不一致",
            model_info.base_model,
            model_info.model_version
        );
    }

    let target_root_dir = src_model_root.join(MODEL_ARTIFACTS_DIR);

    Ok(vec![
        "--model-id-list".to_string(),
        to_string(&model_info.required_model_repo_id_list)
            .context("serialize required model repo ids")?,
        "--model-name-list".to_string(),
        to_string(&model_info.required_model_name_list)
            .context("serialize required model names")?,
        "--target-root-dir".to_string(),
        target_root_dir.to_string_lossy().to_string(),
    ])
}

pub(crate) fn resolve_custom_model_download_script_path(
    src_model_root: &Path,
    model_info: &ModelInfo,
) -> Result<PathBuf> {
    if model_info.download_type != ModelDownloadType::Custom {
        bail!(
            "模型 {}:{} 的下载方式为 {}，不能走自定义下载脚本",
            model_info.base_model,
            model_info.model_version,
            model_info.download_type
        );
    }

    src_model_model_python_script_path(src_model_root, &model_info.base_model, "download.py")
}

pub(crate) fn resolve_model_download_paths(
    src_model_root: &Path,
    model_info: &ModelInfo,
) -> Vec<PathBuf> {
    model_info
        .required_model_name_list
        .iter()
        .map(|artifact_name| src_model_root.join(MODEL_ARTIFACTS_DIR).join(artifact_name))
        .collect()
}

pub(crate) fn validate_model_artifact_paths(
    base_model: &str,
    model_version: &str,
    paths: &[PathBuf],
) -> Result<()> {
    let invalid_paths = paths
        .iter()
        .filter_map(|path| match validate_model_artifact_dir(path) {
            Ok(()) => None,
            Err(reason) => Some(format!("{} ({})", path.display(), reason)),
        })
        .collect::<Vec<_>>();

    if invalid_paths.is_empty() {
        return Ok(());
    }

    bail!(
        "模型 {}:{} 已在 model_info.downloaded 中标记为已下载，但以下模型目录不存在、不是目录或为空目录: {}。请先在模型管理页卸载该模型，再重新安装或重试任务。",
        base_model,
        model_version,
        invalid_paths.join(", ")
    )
}

fn validate_model_artifact_dir(path: &Path) -> std::result::Result<(), &'static str> {
    if !path.exists() {
        return Err("missing");
    }
    if !path.is_dir() {
        return Err("not a directory");
    }

    let mut entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return Err("unreadable directory"),
    };

    if entries.next().is_none() {
        return Err("empty directory");
    }

    Ok(())
}
