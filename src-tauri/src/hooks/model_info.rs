use tauri::State;

use crate::error::AppError;
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
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .model_service()
        .list_model_infos(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn install_model(
    model_id: i64,
    device: HardwareType,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelMutationResult, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .model_service()
        .install_model(model_id, device)
        .await
        .map_err(AppError::from_anyhow)
}

#[tauri::command]
pub async fn get_device_type(
    base_model: String,
    model_version: String,
    state: State<'_, ServiceState>,
) -> std::result::Result<HardwareType, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .model_service()
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
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .model_service()
        .uninstall_model(model_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn set_model_current_device(
    model_id: i64,
    device: HardwareType,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelInfo, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .model_service()
        .set_model_current_device(model_id, device)
        .await
        .map_err(AppError::from_anyhow)
}
