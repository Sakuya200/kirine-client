use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{
            CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload, StreamingReplaySnapshot,
            StreamingSpeakerAvatarAsset, StreamingSpeechTaskResult, UpdateStreamingSpeakersPayload,
            UpdateStreamingSpeakersResult,
        },
        streaming::StreamingSpeechService,
    },
    Result,
};
use anyhow::bail;

#[async_trait]
impl StreamingSpeechService for RemoteService {
    async fn create_streaming_speech_task(
        &self,
        _payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult> {
        bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn send_streaming_message(
        &self,
        _payload: SendStreamingMessagePayload,
        _on_event: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
    ) -> Result<()> {
        bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn cancel_streaming_task(&self, _task_id: i64) -> Result<bool> {
        bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn get_streaming_replay_snapshot(
        &self,
        _history_id: i64,
    ) -> Result<StreamingReplaySnapshot> {
        bail!("远程存储模式暂不支持流式语音历史重放")
    }

    async fn read_streaming_speaker_avatar(
        &self,
        _history_id: i64,
        _speaker_name: String,
    ) -> Result<StreamingSpeakerAvatarAsset> {
        bail!("远程存储模式暂不支持流式语音说话人头像")
    }

    async fn update_streaming_speakers(
        &self,
        _payload: UpdateStreamingSpeakersPayload,
    ) -> Result<UpdateStreamingSpeakersResult> {
        bail!("远程存储模式暂不支持更新流式语音说话人")
    }
}
