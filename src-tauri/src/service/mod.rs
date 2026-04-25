mod local;
pub mod models;
mod remote;
use crate::{
    config::{EnvConfig, StorageMode},
    service::models::{
        CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateTextToSpeechTaskPayload,
        CreateVoiceCloneTaskPayload, HistoryRecord, HistoryTaskType, ImportModelAsSpeakerPayload,
        ModelInfo, ModelMutationResult, ModelTrainingTaskResult, SpeakerInfo,
        TextToSpeechAudioAsset, TextToSpeechTaskResult, UpdateSpeakerPayload,
        UpdateTaskStatusPayload, VoiceCloneAudioAsset, VoiceCloneTaskResult,
    },
    Result,
};
use anyhow::Context;
use async_trait::async_trait;
pub(crate) use local::entity;
pub use local::LocalService;
pub use remote::RemoteService;
pub(crate) mod pipeline;

#[derive(Debug, Clone)]
pub enum ServiceImpl {
    Local(LocalService),
    Remote(RemoteService),
}

pub struct ServiceState(pub ServiceImpl);

impl ServiceImpl {
    pub fn service(&self) -> Result<&(dyn Service + Send + Sync)> {
        match self {
            ServiceImpl::Local(local) => Ok(local),
            ServiceImpl::Remote(remote) => Ok(remote),
        }
    }

    pub async fn close(&self) -> Result<()> {
        self.service()?.close().await
    }
}

#[async_trait]
pub trait Service: Send + Sync {
    async fn new(config: &EnvConfig) -> Result<Self>
    where
        Self: Sized;
    async fn close(&self) -> Result<()>;
    async fn create_speaker_info(&self, payload: CreateSpeakerPayload) -> Result<SpeakerInfo>;
    async fn import_model_as_speaker(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo>;
    async fn list_speaker_infos(&self) -> Result<Vec<SpeakerInfo>>;
    async fn update_speaker_info(&self, payload: UpdateSpeakerPayload) -> Result<SpeakerInfo>;
    async fn delete_speaker_info(&self, speaker_id: i64) -> Result<bool>;
    async fn list_model_infos(&self) -> Result<Vec<ModelInfo>>;
    async fn install_model(&self, model_id: i64) -> Result<ModelMutationResult>;
    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult>;
    async fn list_history_records(&self) -> Result<Vec<HistoryRecord>>;
    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord>;
    async fn read_text_to_speech_audio(&self, history_id: i64) -> Result<TextToSpeechAudioAsset>;
    async fn read_voice_clone_audio(&self, history_id: i64) -> Result<VoiceCloneAudioAsset>;
    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool>;
    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<HistoryRecord>;
    async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult>;
    async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult>;
    async fn cancel_model_training_task(&self, history_id: i64) -> Result<bool>;
    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult>;
}

pub async fn init_service(config: EnvConfig) -> Result<ServiceImpl> {
    match config.mode() {
        StorageMode::Local => {
            let local_service = LocalService::new(&config)
                .await
                .context("failed to initialize local storage")?;
            Ok(ServiceImpl::Local(local_service))
        }
        StorageMode::Remote => {
            let remote_service = RemoteService::new(&config)
                .await
                .context("failed to initialize remote storage")?;
            Ok(ServiceImpl::Remote(remote_service))
        }
    }
}
