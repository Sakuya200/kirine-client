mod db;
pub(crate) mod entity;
mod history;
mod model_info;
mod speaker;
mod training;
mod tts;
mod voice_clone;

use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

use anyhow::Context;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use tracing::info;

use crate::{
    config::{resolve_storage_dir, BaseModel, EnvConfig},
    migration,
    service::{
        models::{
            CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateTextToSpeechTaskPayload,
            CreateVoiceCloneTaskPayload, HistoryRecord, HistoryTaskType, ModelInfo,
            ModelTrainingTaskResult, SpeakerInfo, TextToSpeechAudioAsset, TextToSpeechTaskResult,
            UpdateSpeakerPayload, UpdateTaskStatusPayload, VoiceCloneAudioAsset,
            VoiceCloneTaskResult,
        },
        pipeline::{
            resolve_model_task_pipeline, TrainingPipelineRequest, TtsPipelineRequest,
            VoiceClonePipelineRequest,
        },
        Service,
    },
    Result,
};

#[derive(Debug, Clone)]
pub struct LocalService {
    app_dir: PathBuf,
    data_dir: String,
    model_dir: String,
    orm: DatabaseConnection,
}

#[async_trait]
impl Service for LocalService {
    async fn new(config: &EnvConfig) -> Result<LocalService> {
        let app_dir =
            current_dir().context("failed to resolve current directory for local service")?;
        let data_dir = resolve_storage_dir(config.data_dir(), "data")
            .context("failed to resolve local data directory")?;
        let model_dir = resolve_storage_dir(config.model_dir(), "models")
            .context("failed to resolve local model directory")?;
        Self::from_paths(app_dir, data_dir, model_dir).await
    }
    async fn close(&self) -> Result<()> {
        info!("Closing local storage connection pool");
        self.orm.clone().close().await?;
        Ok(())
    }

    async fn create_speaker_info(&self, payload: CreateSpeakerPayload) -> Result<SpeakerInfo> {
        self.create_speaker_info_impl(payload).await
    }

    async fn list_speaker_infos(&self) -> Result<Vec<SpeakerInfo>> {
        self.list_speaker_infos_impl().await
    }

    async fn update_speaker_info(&self, payload: UpdateSpeakerPayload) -> Result<SpeakerInfo> {
        self.update_speaker_info_impl(payload).await
    }

    async fn delete_speaker_info(&self, speaker_id: i64) -> Result<bool> {
        self.delete_speaker_info_impl(speaker_id).await
    }

    async fn list_model_infos(&self) -> Result<Vec<ModelInfo>> {
        self.list_model_infos_impl().await
    }

    async fn list_history_records(&self) -> Result<Vec<HistoryRecord>> {
        self.list_history_records_impl().await
    }

    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        self.get_history_record_impl(history_id).await
    }

    async fn read_text_to_speech_audio(&self, history_id: i64) -> Result<TextToSpeechAudioAsset> {
        self.read_text_to_speech_audio_impl(history_id).await
    }

    async fn read_voice_clone_audio(&self, history_id: i64) -> Result<VoiceCloneAudioAsset> {
        self.read_voice_clone_audio_impl(history_id).await
    }

    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        self.delete_history_record_impl(history_id, task_type).await
    }

    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<HistoryRecord> {
        self.update_task_status_impl(payload).await
    }

    async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        self.create_text_to_speech_task_impl(payload).await
    }

    async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        self.create_model_training_task_impl(payload).await
    }

    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        self.create_voice_clone_task_impl(payload).await
    }
}

impl LocalService {
    pub(crate) async fn from_paths(
        app_dir: PathBuf,
        data_dir: PathBuf,
        model_dir: PathBuf,
    ) -> Result<Self> {
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).with_context(|| {
                format!(
                    "failed to create local data directory: {}",
                    data_dir.display()
                )
            })?;
        }
        if !model_dir.exists() {
            std::fs::create_dir_all(&model_dir).with_context(|| {
                format!(
                    "failed to create local model directory: {}",
                    model_dir.display()
                )
            })?;
        }

        let db_path = data_dir.join("app.db");
        let orm = db::connect_local_database(&db_path).await?;
        Self::init_db(&orm, &data_dir).await?;

        let service = LocalService {
            app_dir,
            data_dir: data_dir.to_string_lossy().to_string(),
            model_dir: model_dir.to_string_lossy().to_string(),
            orm,
        };
        Ok(service)
    }

    pub(crate) fn start_tts_inference(
        &self,
        base_model: BaseModel,
        task_id: i64,
        speaker_id: i64,
    ) -> Result<()> {
        let service = self.clone();
        let pipeline = resolve_model_task_pipeline(base_model);

        tauri::async_runtime::spawn(async move {
            if let Err(err) = pipeline
                .run_tts_pipeline(
                    &service,
                    TtsPipelineRequest {
                        task_id,
                        speaker_id,
                    },
                )
                .await
            {
                tracing::error!(error = %err, "local tts pipeline failed");
            }
        });

        Ok(())
    }

    pub(crate) fn start_voice_clone_inference(
        &self,
        base_model: BaseModel,
        task_id: i64,
    ) -> Result<()> {
        let service = self.clone();
        let pipeline = resolve_model_task_pipeline(base_model);

        tauri::async_runtime::spawn(async move {
            if let Err(err) = pipeline
                .run_voice_clone_pipeline(&service, VoiceClonePipelineRequest { task_id })
                .await
            {
                tracing::error!(error = %err, "local voice clone pipeline failed");
            }
        });

        Ok(())
    }

    pub(crate) fn start_training(
        &self,
        base_model: BaseModel,
        task_id: i64,
        speaker_id: i64,
        speaker_name: &str,
    ) -> Result<()> {
        let service = self.clone();
        let pipeline = resolve_model_task_pipeline(base_model);
        let speaker_name = speaker_name.to_string();

        tauri::async_runtime::spawn(async move {
            if let Err(err) = pipeline
                .run_training_pipeline(
                    &service,
                    TrainingPipelineRequest {
                        task_id,
                        speaker_id,
                        speaker_name,
                    },
                )
                .await
            {
                tracing::error!(error = %err, "local training pipeline failed");
            }
        });

        Ok(())
    }

    pub(crate) fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn data_dir(&self) -> &str {
        &self.data_dir
    }

    pub(crate) fn model_dir(&self) -> &str {
        &self.model_dir
    }

    pub(crate) fn orm(&self) -> &DatabaseConnection {
        &self.orm
    }

    async fn init_db(orm: &DatabaseConnection, data_dir: &Path) -> Result<()> {
        migration::run_local_migrations(orm)
            .await
            .with_context(|| {
                format!(
                    "failed to run SeaORM local migrations in {}",
                    data_dir.display()
                )
            })?;

        Ok(())
    }
}

pub(crate) fn build_task_title(
    task_type_label: &str,
    speaker_label: Option<&str>,
    create_time: &str,
) -> String {
    let normalized_speaker = speaker_label
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-");

    match normalized_speaker {
        Some(speaker_label) => format!("{}-{}-{}", task_type_label, speaker_label, create_time),
        None => format!("{}-{}", task_type_label, create_time),
    }
}

pub(crate) fn build_sample_file_name(sample_id: &str, file_name: &str) -> String {
    let sanitized = file_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();
    format!("{}_{}", sample_id, sanitized)
}

pub(crate) fn build_task_audio_file_name(source_path: &Path) -> String {
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.trim().is_empty())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();
    format!("audio{}", extension)
}

pub(crate) fn sanitize_path_segment(value: &str) -> String {
    let illegal_chars = [
        '<', '>', ':', '"', '/', '\\', '|', '?', '*', '=', '&', ';', '#', '%', '{', '}', '$', '!',
        '`', '^', '(', ')', '+', ',', '[', ']', '~', '\'', '@',
    ];
    let sanitized = value
        .chars()
        .filter(|ch| !illegal_chars.contains(ch) && !ch.is_control())
        .collect::<String>();
    let sanitized = sanitized.trim_end_matches([' ', '.']).to_string();

    if sanitized.is_empty() {
        "speaker".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn sanitize_file_stem(value: &str, default_stem: &str) -> String {
    let sanitized = sanitize_path_segment(value);
    let trimmed = sanitized.trim_matches('.').trim();

    if trimmed.is_empty() || trimmed == "speaker" {
        default_stem.to_string()
    } else {
        trimmed.to_string()
    }
}
