use tauri::State;

use crate::service::{
    models::{CreateSpeakerPayload, SpeakerInfo, UpdateSpeakerPayload},
    ServiceState,
};

#[tauri::command]
pub async fn create_speaker_info(
    payload: CreateSpeakerPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<SpeakerInfo, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_speaker_info(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn list_speaker_infos(
    state: State<'_, ServiceState>,
) -> std::result::Result<Vec<SpeakerInfo>, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .list_speaker_infos()
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_speaker_info(
    payload: UpdateSpeakerPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<SpeakerInfo, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .update_speaker_info(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_speaker_info(
    speaker_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .delete_speaker_info(speaker_id)
        .await
        .map_err(|err| err.to_string())
}
