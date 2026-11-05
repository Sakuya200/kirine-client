use async_trait::async_trait;

use crate::{
    service::models::{CreateVoiceCloneTaskPayload, VoiceCloneTaskResult},
    Result,
};

#[async_trait]
pub trait VoiceCloneService: Send + Sync {
    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult>;
}
