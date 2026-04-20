use tauri::State;

use crate::service::{models::ModelInfo, ServiceState};

#[tauri::command]
pub async fn list_model_infos(
    state: State<'_, ServiceState>,
) -> std::result::Result<Vec<ModelInfo>, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .list_model_infos()
        .await
        .map_err(|err| err.to_string())
}