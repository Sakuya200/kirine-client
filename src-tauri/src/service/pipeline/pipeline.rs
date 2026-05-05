use anyhow::bail;
use async_trait::async_trait;

use crate::{
    service::{
        local::LocalService,
        pipeline::{
            training::run_common_training_pipeline, ModelTaskPipeline, TrainingPipelineRequest,
            TtsPipelineRequest, VoiceClonePipelineRequest,
        },
    },
    Result,
};

pub(crate) struct CommonModelTaskPipeline {
    base_model: &'static str,
}

impl CommonModelTaskPipeline {
    pub(crate) const fn new(base_model: &'static str) -> Self {
        Self { base_model }
    }
}

#[async_trait]
impl ModelTaskPipeline for CommonModelTaskPipeline {
    async fn run_training_pipeline(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        run_common_training_pipeline(service, request, self.base_model).await
    }

    async fn run_tts_pipeline(
        &self,
        _service: &LocalService,
        _request: TtsPipelineRequest,
    ) -> Result<()> {
        bail!(
            "{} text-to-speech pipeline is not implemented in CommonModelTaskPipeline yet",
            self.base_model
        )
    }

    async fn run_voice_clone_pipeline(
        &self,
        _service: &LocalService,
        _request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        bail!(
            "{} voice clone pipeline is not implemented in CommonModelTaskPipeline yet",
            self.base_model
        )
    }
}
