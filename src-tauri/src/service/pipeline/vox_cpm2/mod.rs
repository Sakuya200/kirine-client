mod tts;
mod voice_clone;

use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use tts::resolve_inference_model_path;

use crate::{
    service::{
        pipeline::{
            llm_models::LlmModelDefinition, model_paths::LlmModelPaths,
            pipeline::CommonModelTaskPipeline,
        },
    },
    Result,
};

pub(crate) const VOX_CPM2_DISPLAY_NAME: &str = "VoxCPM2";
pub(crate) const VOX_CPM2_BASE_MODEL: &str = "vox_cpm2";
const VOX_CPM2_MODEL_PYTHON_SCRIPT_DIR: &str = "vox_cpm2";
const VOX_CPM2_MODEL_ARTIFACTS_DIR: &str = "base-models";
const VOX_CPM2_MODEL_NAME: &str = "VoxCPM2";
pub(crate) const VOX_CPM2_MODEL_SCALE: &str = "2B";
pub(crate) const VOX_CPM2_RUNTIME_METADATA_FILE_NAME: &str = "voxcpm_runtime.json";

pub(crate) static VOX_CPM2_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: VOX_CPM2_DISPLAY_NAME,
        python_script_dir: VOX_CPM2_MODEL_PYTHON_SCRIPT_DIR,
    });

pub(crate) static VOX_CPM2_MODEL_PATHS: VoxCpm2ModelPaths = VoxCpm2ModelPaths;

pub(crate) static VOX_CPM2_MODEL_TASK_PIPELINE: CommonModelTaskPipeline =
    CommonModelTaskPipeline::new(VOX_CPM2_BASE_MODEL);

pub(crate) struct VoxCpm2ModelPaths;

pub(crate) struct VoxCpm2ModelTaskPipeline;

impl LlmModelPaths for VoxCpm2ModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &VOX_CPM2_MODEL_DEFINITION
    }
}

pub(crate) fn vox_cpm2_prepared_model_download_paths(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<Vec<PathBuf>> {
    Ok(vec![vox_cpm2_base_model_path(src_model_root, model_scale)?])
}

fn validate_vox_cpm2_model_scale(model_scale: &str) -> Result<()> {
    let normalized_scale = model_scale.trim();
    if normalized_scale != VOX_CPM2_MODEL_SCALE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的 VoxCPM2 模型规模: {}", model_scale),
        )
        .into());
    }

    Ok(())
}

pub(crate) fn vox_cpm2_base_model_path(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<PathBuf> {
    validate_vox_cpm2_model_scale(model_scale)?;

    Ok(src_model_root
        .join(VOX_CPM2_MODEL_ARTIFACTS_DIR)
        .join(VOX_CPM2_MODEL_NAME))
}

