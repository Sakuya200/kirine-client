use async_trait::async_trait;

use crate::{
    service::models::{CreateTextToSpeechTaskPayload, TextToSpeechTaskResult},
    Result,
};

#[async_trait]
pub trait TextToSpeechService: Send + Sync {
    async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult>;
}
