use tauri::State;

use crate::service::{
    models::{
        CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload,
        StreamingReplaySnapshot, StreamingSpeechTaskResult,
    },
    ServiceState,
};

/// 流式音频事件协议。前端通过 `Channel<AudioStreamEvent>` 订阅。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AudioStreamEvent {
    Started,
    Chunk { bytes: Vec<u8> },
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
    on_event: tauri::ipc::Channel<AudioStreamEvent>,
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
