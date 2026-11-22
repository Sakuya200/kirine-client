use async_trait::async_trait;

use crate::{
    service::models::{CreateVoiceDesignTaskPayload, VoiceDesignTaskResult},
    Result,
};

#[async_trait]
pub trait VoiceDesignService: Send + Sync {
    async fn create_voice_design_task(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult>;
}
