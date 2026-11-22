use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{CreateVoiceDesignTaskPayload, VoiceDesignTaskResult},
        voice_design::VoiceDesignService,
    },
    Result,
};

#[async_trait]
impl VoiceDesignService for RemoteService {
    async fn create_voice_design_task(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult> {
        self.client.create_voice_design_task(payload).await
    }
}
