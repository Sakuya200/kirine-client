pub mod api;
pub mod llm_models;
pub mod model_paths;
pub mod qwen3_tts;
pub mod script_paths;

use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::{config::BaseModel, service::local::LocalService, Result};

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

pub(crate) fn resolve_model_task_pipeline(base_model: BaseModel) -> &'static dyn ModelTaskPipeline {
    match base_model {
        BaseModel::Qwen3Tts => &qwen3_tts::QWEN3_TTS_MODEL_TASK_PIPELINE,
    }
}

pub(crate) fn resolve_inference_model_path(
    base_model: BaseModel,
    model_root_path: &Path,
) -> Result<PathBuf> {
    match base_model {
        BaseModel::Qwen3Tts => qwen3_tts::resolve_inference_model_path(model_root_path),
    }
}
