mod biz;

use std::io;

use anyhow::Context;
use async_trait::async_trait;

use crate::{
    config::EnvConfig,
    service::{
        models::{
            CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateTextToSpeechTaskPayload,
            CreateVoiceCloneTaskPayload, HistoryRecord, HistoryTaskType, ModelInfo,
            ModelTrainingTaskResult, SpeakerInfo, TextToSpeechAudioAsset, TextToSpeechTaskResult,
            UpdateSpeakerPayload, UpdateTaskStatusPayload, VoiceCloneAudioAsset,
            VoiceCloneTaskResult,
        },
        Service,
    },
    Result,
};
use tracing::error;

#[derive(Debug, Clone)]
pub struct RemoteService {
    api_url: String,
}

#[async_trait]
impl Service for RemoteService {
    async fn new(config: &EnvConfig) -> Result<RemoteService> {
        if let Some(api_url) = config.api_url() {
            Ok(RemoteService {
                api_url: api_url.to_string(),
            })
        } else {
            // 报错缺少 API URL 配置
            error!("缺少 API URL 配置");
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "缺少 API URL 配置",
            ))
            .context("remote storage requires remote.api_url in config.toml")
        }
    }
    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn create_speaker_info(&self, _payload: CreateSpeakerPayload) -> Result<SpeakerInfo> {
        biz::unsupported("create_speaker_info")
    }

    async fn list_speaker_infos(&self) -> Result<Vec<SpeakerInfo>> {
        biz::unsupported("list_speaker_infos")
    }

    async fn update_speaker_info(&self, _payload: UpdateSpeakerPayload) -> Result<SpeakerInfo> {
        biz::unsupported("update_speaker_info")
    }

    async fn delete_speaker_info(&self, _speaker_id: i64) -> Result<bool> {
        biz::unsupported("delete_speaker_info")
    }

    async fn list_model_infos(&self) -> Result<Vec<ModelInfo>> {
        biz::unsupported("list_model_infos")
    }

    async fn list_history_records(&self) -> Result<Vec<HistoryRecord>> {
        biz::unsupported("list_history_records")
    }

    async fn get_history_record(&self, _history_id: i64) -> Result<HistoryRecord> {
        biz::unsupported("get_history_record")
    }

    async fn read_text_to_speech_audio(&self, _history_id: i64) -> Result<TextToSpeechAudioAsset> {
        biz::unsupported("read_text_to_speech_audio")
    }

    async fn read_voice_clone_audio(&self, _history_id: i64) -> Result<VoiceCloneAudioAsset> {
        biz::unsupported("read_voice_clone_audio")
    }

    async fn delete_history_record(
        &self,
        _history_id: i64,
        _task_type: HistoryTaskType,
    ) -> Result<bool> {
        biz::unsupported("delete_history_record")
    }

    async fn update_task_status(&self, _payload: UpdateTaskStatusPayload) -> Result<HistoryRecord> {
        biz::unsupported("update_task_status")
    }

    async fn create_text_to_speech_task(
        &self,
        _payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        biz::unsupported("create_text_to_speech_task")
    }

    async fn create_model_training_task(
        &self,
        _payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        biz::unsupported("create_model_training_task")
    }

    async fn create_voice_clone_task(
        &self,
        _payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        biz::unsupported("create_voice_clone_task")
    }
}
