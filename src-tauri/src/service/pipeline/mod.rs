pub mod api;
pub mod llm_models;
pub mod model_paths;
pub mod qwen3_tts;
pub mod script_paths;
pub mod vox_cpm2;

use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::{service::local::LocalService, Result};

use self::{qwen3_tts::QWEN3_TTS_BASE_MODEL, vox_cpm2::VOX_CPM2_BASE_MODEL};

#[derive(Debug, Clone)]
pub(crate) struct TrainingPipelineRequest {
    pub task_id: i64,
    pub speaker_id: i64,
    pub speaker_name: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TtsPipelineRequest {
    pub task_id: i64,
    pub speaker_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VoiceClonePipelineRequest {
    pub task_id: i64,
}

#[async_trait]
pub(crate) trait ModelTaskPipeline: Send + Sync {
    async fn run_training_pipeline(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()>;

    async fn run_tts_pipeline(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()>;

    async fn run_voice_clone_pipeline(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()>;
}

pub(crate) fn resolve_model_task_pipeline(
    base_model: &str,
) -> Result<&'static dyn ModelTaskPipeline> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => Ok(&qwen3_tts::QWEN3_TTS_MODEL_TASK_PIPELINE),
        VOX_CPM2_BASE_MODEL => Ok(&vox_cpm2::VOX_CPM2_MODEL_TASK_PIPELINE),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}

pub(crate) fn resolve_inference_model_path(
    base_model: &str,
    model_root_path: &Path,
) -> Result<PathBuf> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => qwen3_tts::resolve_inference_model_path(model_root_path),
        VOX_CPM2_BASE_MODEL => vox_cpm2::resolve_inference_model_path(model_root_path),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}
