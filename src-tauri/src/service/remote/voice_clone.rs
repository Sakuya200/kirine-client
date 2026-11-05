use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{CreateVoiceCloneTaskPayload, VoiceCloneTaskResult},
        voice_clone::VoiceCloneService,
    },
    Result,
};

#[async_trait]
impl VoiceCloneService for RemoteService {
    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        self.client.create_voice_clone_task(payload).await
    }
}
