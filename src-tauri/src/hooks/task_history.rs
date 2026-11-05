use tauri::{AppHandle, State};

use crate::error::AppError;
use crate::service::{
    models::{
        CreateModelTrainingTaskPayload, CreateTextToSpeechTaskPayload, CreateVoiceCloneTaskPayload,
        CreateVoiceDesignTaskPayload, GeneratedAudioAsset, GeneratedAudioSource, HistoryFilter,
        HistoryRecord, HistoryRecordSummary, HistoryTaskType, ModelTrainingTaskResult, Page,
        PageRequest, TextToSpeechTaskResult, VoiceCloneTaskResult, VoiceDesignTaskResult,
    },
    ServiceState,
};
use crate::utils::audio::save_bytes_as;

const MODEL_TRAINING_TEMPLATE_JSONL_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../public/templates/model-training/model-training-annotation-template.jsonl"
));
const MODEL_TRAINING_TEMPLATE_XLSX_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../public/templates/model-training/model-training-annotation-template.xlsx"
));

#[tauri::command]
pub async fn list_history_records(
    request: PageRequest<HistoryFilter>,
    state: State<'_, ServiceState>,
) -> std::result::Result<Page<HistoryRecordSummary>, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .history_service()
        .list_history_records(request)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_history_record(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<HistoryRecord, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .history_service()
        .get_history_record(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_generated_audio(
    source: GeneratedAudioSource,
    state: State<'_, ServiceState>,
) -> std::result::Result<GeneratedAudioAsset, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .generated_audio_service()
        .read_generated_audio(source)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn save_generated_audio_as(
    source: GeneratedAudioSource,
    app: AppHandle,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .generated_audio_service()
        .save_generated_audio_as(source, app)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub fn save_model_training_template_as(
    template_format: String,
    app: AppHandle,
) -> std::result::Result<bool, String> {
    let normalized = template_format.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "jsonl" => save_bytes_as(
            &app,
            "model-training-annotation-template.jsonl",
            MODEL_TRAINING_TEMPLATE_JSONL_BYTES,
            Some("JSONL 文件"),
        ),
        "xlsx" => save_bytes_as(
            &app,
            "model-training-annotation-template.xlsx",
            MODEL_TRAINING_TEMPLATE_XLSX_BYTES,
            Some("Excel 文件"),
        ),
        _ => Err(format!("不支持的模板格式: {}", template_format)),
    }
}

#[tauri::command]
pub async fn delete_history_record(
    history_id: i64,
    task_type: HistoryTaskType,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .inner()
        .service()
        .map_err(|err| err.to_string())?
        .history_service()
        .delete_history_record(history_id, task_type)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_text_to_speech_task(
    payload: CreateTextToSpeechTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<TextToSpeechTaskResult, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .text_to_speech_service()
        .create_text_to_speech_task(payload)
        .await
        .map_err(AppError::from_anyhow)
}

#[tauri::command]
pub async fn create_model_training_task(
    payload: CreateModelTrainingTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelTrainingTaskResult, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .model_training_service()
        .create_model_training_task(payload)
        .await
        .map_err(AppError::from_anyhow)
}

#[tauri::command]
pub async fn cancel_history_task(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .history_service()
        .cancel_history_task(history_id)
        .await
        .map_err(AppError::from_anyhow)
}

#[tauri::command]
pub async fn create_voice_clone_task(
    payload: CreateVoiceCloneTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceCloneTaskResult, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .voice_clone_service()
        .create_voice_clone_task(payload)
        .await
        .map_err(AppError::from_anyhow)
}

#[tauri::command]
pub async fn create_voice_design_task(
    payload: CreateVoiceDesignTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceDesignTaskResult, AppError> {
    state
        .inner()
        .service()
        .map_err(AppError::from_anyhow)?
        .voice_design_service()
        .create_voice_design_task(payload)
        .await
        .map_err(AppError::from_anyhow)
}
