use async_trait::async_trait;

use crate::{
    service::{
        local::LocalService,
        pipeline::{
            training::run_common_training_pipeline, tts::run_common_tts_pipeline,
            voice_clone::run_common_voice_clone_pipeline, ModelTaskPipeline,
            TrainingPipelineRequest, TtsPipelineRequest, VoiceClonePipelineRequest,
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
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        run_common_tts_pipeline(service, request, self.base_model).await
    }

    async fn run_voice_clone_pipeline(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        run_common_voice_clone_pipeline(service, request, self.base_model).await
    }
}
