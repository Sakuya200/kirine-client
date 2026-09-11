pub(crate) mod local;
pub mod models;
mod remote;
use crate::{
    config::{EnvConfig, HardwareType, StorageMode},
    service::models::{
        CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateStreamingSpeechTaskPayload,
        CreateTextToSpeechTaskPayload, CreateVoiceCloneTaskPayload, CreateVoiceDesignTaskPayload,
        GeneratedAudioAsset, GeneratedAudioSource, HistoryFilter, HistoryRecord,
        HistoryRecordSummary, HistoryTaskType, ImportModelAsSpeakerPayload, ModelFilter, ModelInfo,
        ModelMutationResult, ModelTrainingTaskResult, Page, PageRequest,
        SendStreamingMessagePayload, SpeakerFilter, SpeakerInfo, SpeakerPageResult,
        StreamingReplaySnapshot, StreamingSpeakerAvatarAsset, StreamingSpeechTaskResult,
        TextToSpeechTaskResult, UpdateSpeakerPayload, UpdateStreamingSpeakersPayload,
        UpdateStreamingSpeakersResult, UpdateTaskStatusPayload, VoiceCloneTaskResult,
        VoiceDesignTaskResult,
    },
    Result,
};
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
    async fn list_speaker_infos(
        &self,
        request: PageRequest<SpeakerFilter>,
    ) -> Result<SpeakerPageResult>;
    async fn update_speaker_info(&self, payload: UpdateSpeakerPayload) -> Result<SpeakerInfo>;
    async fn delete_speaker_info(&self, speaker_id: i64) -> Result<bool>;
    async fn list_model_infos(&self, request: PageRequest<ModelFilter>) -> Result<Page<ModelInfo>>;
    async fn get_device_type(&self, base_model: &str, model_version: &str) -> Result<HardwareType>;
    async fn install_model(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult>;
    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult>;
    async fn set_model_current_device(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelInfo>;
    async fn list_history_records(
        &self,
        request: PageRequest<HistoryFilter>,
    ) -> Result<Page<HistoryRecordSummary>>;
    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord>;
    async fn read_generated_audio(
        &self,
        source: GeneratedAudioSource,
    ) -> Result<GeneratedAudioAsset>;
    async fn save_generated_audio_as(
        &self,
        source: GeneratedAudioSource,
        app: tauri::AppHandle,
    ) -> Result<bool>;
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
    async fn cancel_history_task(&self, history_id: i64) -> Result<bool>;
    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult>;
    async fn create_voice_design_task(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult>;
    async fn create_streaming_speech_task(
        &self,
        payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult>;
    async fn send_streaming_message(
        &self,
        payload: SendStreamingMessagePayload,
        on_event: tauri::ipc::Channel<tauri::ipc::InvokeResponseBody>,
    ) -> Result<()>;
    async fn cancel_streaming_task(&self, task_id: i64) -> Result<bool>;
    async fn get_streaming_replay_snapshot(
        &self,
        history_id: i64,
    ) -> Result<StreamingReplaySnapshot>;
    async fn read_streaming_speaker_avatar(
        &self,
        history_id: i64,
        speaker_name: String,
    ) -> Result<StreamingSpeakerAvatarAsset>;
    async fn update_streaming_speakers(
        &self,
        payload: UpdateStreamingSpeakersPayload,
    ) -> Result<UpdateStreamingSpeakersResult>;
}

pub async fn init_service(config: EnvConfig) -> Result<ServiceImpl> {
    match config.mode() {
        StorageMode::Local => {
            let local_service = LocalService::new(&config)
                .await
                .map_err(|e| anyhow::anyhow!("failed to initialize local service: {}", e))?;
            Ok(ServiceImpl::Local(local_service))
        }
        StorageMode::Remote => {
            let remote_service = RemoteService::new(&config)
                .await
                .map_err(|e| anyhow::anyhow!("failed to initialize remote service: {}", e))?;
            Ok(ServiceImpl::Remote(remote_service))
        }
    }
}
