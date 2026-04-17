mod training;
mod tts;
mod voice_clone;

use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use tts::resolve_inference_model_path;

use async_trait::async_trait;

use crate::{
    service::{
        local::LocalService,
        pipeline::{
            llm_models::LlmModelDefinition, model_paths::LlmModelPaths, ModelTaskPipeline,
            TrainingPipelineRequest, TtsPipelineRequest, VoiceClonePipelineRequest,
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

pub(crate) static VOX_CPM2_MODEL_TASK_PIPELINE: VoxCpm2ModelTaskPipeline =
    VoxCpm2ModelTaskPipeline;

pub(crate) struct VoxCpm2ModelPaths;

pub(crate) struct VoxCpm2ModelTaskPipeline;

impl LlmModelPaths for VoxCpm2ModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &VOX_CPM2_MODEL_DEFINITION
    }
}

pub(crate) fn vox_cpm2_base_model_path(src_model_root: &Path, model_scale: &str) -> Result<PathBuf> {
    let normalized_scale = model_scale.trim();
    if normalized_scale != VOX_CPM2_MODEL_SCALE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的 VoxCPM2 模型规模: {}", model_scale),
        )
        .into());
    }

    Ok(src_model_root
        .join(VOX_CPM2_MODEL_ARTIFACTS_DIR)
        .join(VOX_CPM2_MODEL_NAME))
}

#[async_trait]
impl ModelTaskPipeline for VoxCpm2ModelTaskPipeline {
    async fn run_training_pipeline(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        self.run_training_pipeline_impl(service, request).await
    }

    async fn run_tts_pipeline(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        self.run_tts_pipeline_impl(service, request).await
    }

    async fn run_voice_clone_pipeline(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        self.run_voice_clone_pipeline_impl(service, request).await
    }
}
