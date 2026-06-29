use tauri::State;

use crate::{
    config::HardwareType,
    service::{
        models::{ModelFilter, ModelInfo, ModelMutationResult, Page, PageRequest},
        ServiceState,
    },
};

#[tauri::command]
pub async fn list_model_infos(
    request: PageRequest<ModelFilter>,
    state: State<'_, ServiceState>,
) -> std::result::Result<Page<ModelInfo>, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .list_model_infos(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn install_model(
    model_id: i64,
    device: HardwareType,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelMutationResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .install_model(model_id, device)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_device_type(
    base_model: String,
    model_version: String,
    state: State<'_, ServiceState>,
) -> std::result::Result<HardwareType, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .get_device_type(&base_model, &model_version)
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