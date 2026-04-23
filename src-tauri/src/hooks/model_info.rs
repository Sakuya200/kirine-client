use tauri::State;

use crate::service::{models::{ModelInfo, ModelMutationResult}, ServiceState};

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

#[tauri::command]
pub async fn install_model(
    model_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelMutationResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .install_model(model_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn uninstall_model(
    model_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelMutationResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .uninstall_model(model_id)
        .await
        .map_err(|err| err.to_string())
}