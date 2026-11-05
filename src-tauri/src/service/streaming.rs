use async_trait::async_trait;

use crate::{
    service::models::{
        CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload, StreamingReplaySnapshot,
        StreamingSpeakerAvatarAsset, StreamingSpeechTaskResult, UpdateStreamingSpeakersPayload,
        UpdateStreamingSpeakersResult,
    },
    Result,
};

#[async_trait]
pub trait StreamingSpeechService: Send + Sync {
    async fn create_streaming_speech_task(
        &self,
        payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult>;
    async fn send_streaming_message(
        &self,
        payload: SendStreamingMessagePayload,
        on_event: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
    ) -> Result<()>;
    async fn cancel_streaming_task(&self, task_id: i64) -> Result<bool>;
    async fn get_streaming_replay_snapshot(
        &self,
        history_id: i64,
    ) -> Result<StreamingReplaySnapshot>;
    async fn read_streaming_speaker_avatar(
        &self,
        history_id: i64,
        speaker_name: String,
    ) -> Result<StreamingSpeakerAvatarAsset>;
    async fn update_streaming_speakers(
        &self,
        payload: UpdateStreamingSpeakersPayload,
    ) -> Result<UpdateStreamingSpeakersResult>;
}
