use tauri::State;

use crate::service::{
    models::{
        CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload, StreamingReplaySnapshot,
        StreamingSpeakerAvatarAsset, StreamingSpeechTaskResult, UpdateStreamingSpeakersPayload,
        UpdateStreamingSpeakersResult,
    },
    ServiceState,
};

/// 流式音频事件协议（控制事件，经 Channel 以 JSON 下发）。chunk 不走此协议：
/// Rust 侧以 `InvokeResponseBody::Raw` 直发二进制，前端收到 ArrayBuffer。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AudioStreamEvent {
    Started,
    Finished,
    Error { message: String },
}

#[tauri::command]
pub async fn create_streaming_speech_task(
    payload: CreateStreamingSpeechTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<StreamingSpeechTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_streaming_speech_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn send_streaming_message(
    payload: SendStreamingMessagePayload,
    on_event: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
    state: State<'_, ServiceState>,
) -> std::result::Result<(), String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .send_streaming_message(payload, on_event)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cancel_streaming_task(
    task_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .cancel_streaming_task(task_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_streaming_replay_snapshot(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<StreamingReplaySnapshot, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .get_streaming_replay_snapshot(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_streaming_speaker_avatar(
    history_id: i64,
    speaker_name: String,
    state: State<'_, ServiceState>,
) -> std::result::Result<StreamingSpeakerAvatarAsset, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .read_streaming_speaker_avatar(history_id, speaker_name)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_streaming_speakers(
    payload: UpdateStreamingSpeakersPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<UpdateStreamingSpeakersResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .update_streaming_speakers(payload)
        .await
        .map_err(|err| err.to_string())
}
