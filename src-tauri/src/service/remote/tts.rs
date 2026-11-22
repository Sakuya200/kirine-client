use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{CreateTextToSpeechTaskPayload, TextToSpeechTaskResult},
        tts::TextToSpeechService,
    },
    Result,
};

#[async_trait]
impl TextToSpeechService for RemoteService {
    async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        self.client.create_text_to_speech_task(payload).await
    }
}
