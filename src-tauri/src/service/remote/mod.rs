use async_trait::async_trait;

use crate::{
    client::ApiClient,
    config::{EnvConfig, HardwareType},
    service::{
        models::{
            CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateStreamingSpeechTaskPayload,
            CreateTextToSpeechTaskPayload, CreateVoiceCloneTaskPayload, CreateVoiceDesignTaskPayload,
            HistoryFilter, HistoryRecord, HistoryRecordSummary, HistoryTaskType,
            ImportModelAsSpeakerPayload, ModelFilter, ModelInfo, ModelMutationResult,
            ModelTrainingTaskResult, Page, PageRequest, SendStreamingMessagePayload, SpeakerFilter,
            SpeakerInfo, SpeakerPageResult, StreamingSpeechTaskResult, TextToSpeechAudioAsset,
            TextToSpeechTaskResult, UpdateSpeakerPayload, UpdateTaskStatusPayload,
            VoiceCloneAudioAsset, VoiceCloneTaskResult, VoiceDesignAudioAsset, VoiceDesignTaskResult,
        },
        Service,
    },
    Result,
};
use anyhow::Context;

#[derive(Debug, Clone)]
pub struct RemoteService {
    client: ApiClient,
}

#[async_trait]
impl Service for RemoteService {
    async fn new(config: &EnvConfig) -> Result<RemoteService> {
        let api_url = config.api_url().unwrap_or("").to_string();
        let api_token = config.api_token().map(str::to_string);
        let client = ApiClient::new(api_url, api_token)
            .context("remote storage requires remote.api_url in config.toml")?;
        Ok(RemoteService { client })
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn create_speaker_info(&self, payload: CreateSpeakerPayload) -> Result<SpeakerInfo> {
        self.client.create_speaker_info(payload).await
    }

    async fn import_model_as_speaker(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        self.client.import_model_as_speaker(payload).await
    }

    async fn list_speaker_infos(
        &self,
        request: PageRequest<SpeakerFilter>,
    ) -> Result<SpeakerPageResult> {
        self.client
            .list_speaker_infos(request.into())
            .await
            .map(Into::into)
    }

    async fn update_speaker_info(&self, payload: UpdateSpeakerPayload) -> Result<SpeakerInfo> {
        self.client.update_speaker_info(payload).await
    }

    async fn delete_speaker_info(&self, speaker_id: i64) -> Result<bool> {
        self.client.delete_speaker_info(speaker_id).await
    }

    async fn list_model_infos(
        &self,
        request: PageRequest<ModelFilter>,
    ) -> Result<Page<ModelInfo>> {
        self.client
            .list_model_infos(request.into())
            .await
            .map(Into::into)
    }

    async fn get_device_type(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<HardwareType> {
        self.client.get_device_type(base_model, model_version).await
    }

    async fn install_model(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult> {
        self.client.install_model(model_id, device).await
    }

    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult> {
        self.client.uninstall_model(model_id).await
    }

    async fn set_model_current_device(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelInfo> {
        self.client.set_model_current_device(model_id, device).await
    }

    async fn list_history_records(
        &self,
        request: PageRequest<HistoryFilter>,
    ) -> Result<Page<HistoryRecordSummary>> {
        self.client
            .list_history_records(request.into())
            .await
            .map(Into::into)
    }

    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        self.client.get_history_record(history_id).await
    }

    async fn read_text_to_speech_audio(&self, history_id: i64) -> Result<TextToSpeechAudioAsset> {
        self.client.read_text_to_speech_audio(history_id).await
    }

    async fn read_voice_clone_audio(&self, history_id: i64) -> Result<VoiceCloneAudioAsset> {
        self.client.read_voice_clone_audio(history_id).await
    }

    async fn read_voice_design_audio(&self, history_id: i64) -> Result<VoiceDesignAudioAsset> {
        self.client.read_voice_design_audio(history_id).await
    }

    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        self.client.delete_history_record(history_id, task_type).await
    }

    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<HistoryRecord> {
        self.client.update_task_status(payload).await
    }

    async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        self.client.create_text_to_speech_task(payload).await
    }

    async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        self.client.create_model_training_task(payload).await
    }

    async fn cancel_history_task(&self, history_id: i64) -> Result<bool> {
        self.client.cancel_history_task(history_id).await
    }

    async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        self.client.create_voice_clone_task(payload).await
    }

    async fn create_voice_design_task(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult> {
        self.client.create_voice_design_task(payload).await
    }

    async fn create_streaming_speech_task(
        &self,
        _payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn send_streaming_message(
        &self,
        _payload: SendStreamingMessagePayload,
        _on_event: tauri::ipc::Channel<crate::hooks::streaming::AudioStreamEvent>,
    ) -> Result<()> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn cancel_streaming_task(&self, _task_id: i64) -> Result<bool> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }
}
