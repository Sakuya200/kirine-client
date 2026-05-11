mod db;
pub(crate) mod entity;
mod history;
mod model_info;
mod speaker;
mod supported_models;
mod training;
mod tts;
mod voice_clone;

use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use anyhow::Context;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use tokio::sync::watch;
use tracing::info;

use crate::{
    common::local_paths::{resolve_task_path, serialize_task_path},
    config::{
        load_ui_configs, resolve_storage_dir, BaseModel, EnvConfig, UiComponentType,
        UiConfigCatalog, UiTaskKind,
    },
    migration,
    service::{
        models::{
            CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateTextToSpeechTaskPayload,
            CreateVoiceCloneTaskPayload, HistoryRecord, HistoryTaskType,
            ImportModelAsSpeakerPayload, ModelInfo, ModelMutationResult, ModelTrainingTaskResult,
            SpeakerInfo, TextToSpeechAudioAsset, TextToSpeechTaskResult, UpdateSpeakerPayload,
            UpdateTaskStatusPayload, VoiceCloneAudioAsset, VoiceCloneTaskResult,
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
    runtime_config: Arc<RwLock<EnvConfig>>,
    orm: DatabaseConnection,
    active_training_controls: Arc<RwLock<HashMap<i64, watch::Sender<bool>>>>,
    ui_config: Arc<UiConfigCatalog>,
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
        Self::from_paths_with_config(app_dir, data_dir, model_dir, config.clone()).await
    }
    async fn close(&self) -> Result<()> {
        info!("Closing local storage connection pool");
        self.orm.clone().close().await?;
        Ok(())
    }

    async fn create_speaker_info(&self, payload: CreateSpeakerPayload) -> Result<SpeakerInfo> {
        self.create_speaker_info_impl(payload).await
    }

    async fn import_model_as_speaker(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        self.import_model_as_speaker_impl(payload).await
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

    async fn install_model(&self, model_id: i64) -> Result<ModelMutationResult> {
        self.install_model_impl(model_id).await
    }

    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult> {
        self.uninstall_model_impl(model_id).await
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

    async fn cancel_model_training_task(&self, history_id: i64) -> Result<bool> {
        self.cancel_model_training_task_impl(history_id).await
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
        Self::from_paths_with_config(app_dir, data_dir, model_dir, EnvConfig::default()).await
    }

    pub(crate) async fn from_paths_with_config(
        app_dir: PathBuf,
        data_dir: PathBuf,
        model_dir: PathBuf,
        runtime_config: EnvConfig,
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
            runtime_config: Arc::new(RwLock::new(runtime_config)),
            orm,
            active_training_controls: Arc::new(RwLock::new(HashMap::new())),
            ui_config: Arc::new(load_ui_configs().unwrap_or_default()),
        };
        Ok(service)
    }

    pub(crate) fn start_tts_inference(&self, base_model: BaseModel, task_id: i64) -> Result<()> {
        let service = self.clone();
        let pipeline = resolve_model_task_pipeline(&base_model)?;

        tauri::async_runtime::spawn(async move {
            if let Err(err) = pipeline
                .run_tts_pipeline(
                    base_model.to_string(),
                    &service,
                    TtsPipelineRequest { task_id },
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
        let pipeline = resolve_model_task_pipeline(&base_model)?;

        tauri::async_runtime::spawn(async move {
            if let Err(err) = pipeline
                .run_voice_clone_pipeline(
                    base_model.to_string(),
                    &service,
                    VoiceClonePipelineRequest { task_id },
                )
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
        let pipeline = resolve_model_task_pipeline(&base_model)?;
        let speaker_name = speaker_name.to_string();
        let (cancel_tx, _cancel_rx) = watch::channel(false);
        self.register_active_training_control(task_id, cancel_tx);

        tauri::async_runtime::spawn(async move {
            let result = pipeline
                .run_training_pipeline(
                    base_model.to_string(),
                    &service,
                    TrainingPipelineRequest {
                        task_id,
                        speaker_id,
                        speaker_name,
                    },
                )
                .await;
            service.unregister_active_training_control(task_id);

            if let Err(err) = result {
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

    pub(crate) fn runtime_config(&self) -> Result<EnvConfig> {
        self.runtime_config
            .read()
            .map(|config| config.clone())
            .map_err(|_| anyhow::anyhow!("无法读取运行时配置状态"))
    }

    pub(crate) fn replace_runtime_config(&self, next_config: EnvConfig) -> Result<()> {
        let mut config = self
            .runtime_config
            .write()
            .map_err(|_| anyhow::anyhow!("无法写入运行时配置状态"))?;
        *config = next_config;
        Ok(())
    }

    pub(crate) fn register_active_training_control(
        &self,
        task_id: i64,
        cancel_tx: watch::Sender<bool>,
    ) {
        if let Ok(mut controls) = self.active_training_controls.write() {
            controls.insert(task_id, cancel_tx);
        }
    }

    pub(crate) fn unregister_active_training_control(&self, task_id: i64) {
        if let Ok(mut controls) = self.active_training_controls.write() {
            controls.remove(&task_id);
        }
    }

    pub(crate) fn active_training_cancel_receiver(
        &self,
        task_id: i64,
    ) -> Result<watch::Receiver<bool>> {
        let controls = self
            .active_training_controls
            .read()
            .map_err(|_| anyhow::anyhow!("无法读取运行中训练任务句柄"))?;
        let cancel_tx = controls.get(&task_id).ok_or_else(|| {
            anyhow::anyhow!("当前训练任务没有可用的终止句柄，可能已经结束或应用已重启")
        })?;
        Ok(cancel_tx.subscribe())
    }

    pub(crate) fn request_active_training_cancel(&self, task_id: i64) -> Result<bool> {
        let controls = self
            .active_training_controls
            .read()
            .map_err(|_| anyhow::anyhow!("无法读取运行中训练任务句柄"))?;
        let cancel_tx = controls.get(&task_id).ok_or_else(|| {
            anyhow::anyhow!("当前训练任务没有可用的终止句柄，可能已经结束或应用已重启")
        })?;

        if *cancel_tx.borrow() {
            return Ok(false);
        }

        cancel_tx
            .send(true)
            .map_err(|_| anyhow::anyhow!("训练任务终止信号发送失败"))?;
        Ok(true)
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

        supported_models::sync_supported_models(orm)
            .await
            .with_context(|| {
                format!(
                    "failed to sync supported_models.json into local database in {}",
                    data_dir.display()
                )
            })?;

        Ok(())
    }

    pub(crate) fn ui_config(&self) -> &UiConfigCatalog {
        &self.ui_config
    }
}

/// Copies any model-specific file-picker param values (componentType: input-audio-file or
/// input-text-file) into the task's sample directory and rewrites the corresponding entries in
/// `model_params` to the serialized (relative) destination paths.
///
/// If no matching `TaskParamConfig` exists for the given `base_model`/`task_kind`, this is a
/// no-op and returns `Ok(())`.
pub(crate) fn copy_model_param_files(
    base_model: &str,
    task_kind: UiTaskKind,
    model_params: &mut serde_json::Value,
    sample_dir: &Path,
    data_dir: &Path,
    ui_config: &UiConfigCatalog,
) -> crate::Result<()> {
    use anyhow::Context as _;

    let task_config = ui_config
        .task_configs
        .iter()
        .find(|c| c.base_model == base_model && c.task == task_kind);

    let task_config = match task_config {
        Some(c) => c,
        None => return Ok(()),
    };

    for param in &task_config.params {
        if param.component_type != UiComponentType::InputAudioFile
            && param.component_type != UiComponentType::InputTextFile
        {
            continue;
        }

        let raw_value = match model_params.get(&param.name).and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => continue,
        };

        let src_path = resolve_task_path(data_dir, &raw_value);
        if !src_path.exists() {
            tracing::warn!(
                param = %param.name,
                path = %src_path.display(),
                "model param file not found, skipping copy"
            );
            continue;
        }

        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.trim().is_empty())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let dest_name = format!("{}{}", param.name, ext);
        let dest_path = sample_dir.join(&dest_name);

        fs::copy(&src_path, &dest_path).with_context(|| {
            format!(
                "failed to copy model param file '{}' from {} to {}",
                param.name,
                src_path.display(),
                dest_path.display()
            )
        })?;

        let serialized = serialize_task_path(data_dir, &dest_path);
        if let Some(obj) = model_params.as_object_mut() {
            obj.insert(param.name.clone(), serde_json::Value::String(serialized));
        }
    }

    Ok(())
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
