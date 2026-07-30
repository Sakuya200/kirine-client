use tauri::{AppHandle, State};

use crate::service::{
    models::{
        CreateModelTrainingTaskPayload, CreateTextToSpeechTaskPayload, CreateVoiceCloneTaskPayload,
        CreateVoiceDesignTaskPayload, GeneratedAudioSource, HistoryFilter, HistoryRecord,
        HistoryRecordSummary, HistoryTaskType, ModelTrainingTaskResult, Page, PageRequest,
        TextToSpeechAudioAsset, TextToSpeechTaskResult, VoiceCloneAudioAsset,
        VoiceCloneTaskResult, VoiceDesignAudioAsset, VoiceDesignTaskResult,
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
        .0
        .service()
        .map_err(|err| err.to_string())?
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
        .0
        .service()
        .map_err(|err| err.to_string())?
        .get_history_record(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_text_to_speech_audio(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<TextToSpeechAudioAsset, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .read_text_to_speech_audio(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_voice_clone_audio(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceCloneAudioAsset, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .read_voice_clone_audio(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn get_voice_design_audio(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceDesignAudioAsset, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .read_voice_design_audio(history_id)
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
        .0
        .service()
        .map_err(|err| err.to_string())?
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
        .0
        .service()
        .map_err(|err| err.to_string())?
        .delete_history_record(history_id, task_type)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_text_to_speech_task(
    payload: CreateTextToSpeechTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<TextToSpeechTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_text_to_speech_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_model_training_task(
    payload: CreateModelTrainingTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<ModelTrainingTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_model_training_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cancel_history_task(
    history_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .cancel_history_task(history_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_voice_clone_task(
    payload: CreateVoiceCloneTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceCloneTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_voice_clone_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn create_voice_design_task(
    payload: CreateVoiceDesignTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<VoiceDesignTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_voice_design_task(payload)
        .await
        .map_err(|err| err.to_string())
}
