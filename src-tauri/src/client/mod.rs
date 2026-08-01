//! 远端 HTTP 接口客户端（初步实现 / 占位）。
//!
//! 当前阶段仅占位：每个接口预定义路径常量（见 [`paths`]），与 `api_url` 拼接成完整地址，
//! 打印请求方法 / 完整路径 / 参数后返回占位错误（HTTP 调用尚未接入）。
//! 分页查询统一使用 [`PageRequest`] / [`PageResponse`]（说话人列表用 [`SpeakerPageResponse`]）。

mod entity;
mod paths;

pub use entity::{PageRequest, PageResponse, SpeakerPageResponse};

use crate::{
    config::HardwareType,
    service::models::{
        CreateModelTrainingTaskPayload, CreateSpeakerPayload, CreateTextToSpeechTaskPayload,
        CreateVoiceCloneTaskPayload, CreateVoiceDesignTaskPayload, HistoryFilter, HistoryRecord,
        HistoryRecordSummary, HistoryTaskType, ImportModelAsSpeakerPayload, ModelFilter, ModelInfo,
        ModelMutationResult, ModelTrainingTaskResult, SpeakerFilter, SpeakerInfo,
        TextToSpeechAudioAsset, TextToSpeechTaskResult, UpdateSpeakerPayload,
        UpdateTaskStatusPayload, VoiceCloneAudioAsset, VoiceCloneTaskResult, VoiceDesignAudioAsset,
        VoiceDesignTaskResult,
    },
    Result,
};
use anyhow::bail;
use serde::Serialize;
use serde_json::json;
use tracing::info;

/// 远端 API 客户端：持有 `api_url` 与可选鉴权 `api_token`。
#[derive(Debug, Clone)]
pub struct ApiClient {
    api_url: String,
    #[allow(dead_code)]
    api_token: Option<String>,
}

impl ApiClient {
    /// 构造客户端；`api_url` 去除尾部 `/`，为空时报错。
    pub fn new(api_url: String, api_token: Option<String>) -> Result<Self> {
        let api_url = api_url.trim().trim_end_matches('/').to_string();
        if api_url.is_empty() {
            bail!("api_url 不能为空（请在 config.toml 配置 [remote].api_url）");
        }
        Ok(ApiClient { api_url, api_token })
    }

    /// 拼接完整请求地址：`api_url + path`。
    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_url, path)
    }

    /// 占位实现：打印请求方法 / 完整地址 / 参数后，返回「HTTP 调用尚未接入」错误。
    async fn placeholder<T>(&self, method: &str, path: &str, params: &str) -> Result<T> {
        let url = self.url(path);
        info!(
            %method, %url, %params,
            "[client] 远端接口占位：HTTP 调用尚未接入"
        );
        bail!(
            "client HTTP 调用尚未接入: {} {} | 参数: {}",
            method,
            url,
            params
        )
    }

    /// 序列化参数为 JSON 字符串（序列化失败时回退为占位文本）。
    fn json<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap_or_else(|_| "<serialize failed>".to_string())
    }

    // ================================ Speaker ================================

    pub async fn create_speaker_info(&self, payload: CreateSpeakerPayload) -> Result<SpeakerInfo> {
        self.placeholder("POST", paths::SPEAKERS, &Self::json(&payload))
            .await
    }

    pub async fn import_model_as_speaker(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        self.placeholder("POST", paths::SPEAKERS_IMPORT, &Self::json(&payload))
            .await
    }

    pub async fn list_speaker_infos(
        &self,
        request: PageRequest<SpeakerFilter>,
    ) -> Result<SpeakerPageResponse> {
        self.placeholder("GET", paths::SPEAKERS, &Self::json(&request))
            .await
    }

    pub async fn update_speaker_info(&self, payload: UpdateSpeakerPayload) -> Result<SpeakerInfo> {
        let path = paths::with_id(paths::SPEAKER_BY_ID, payload.id);
        self.placeholder("PUT", &path, &Self::json(&payload)).await
    }

    pub async fn delete_speaker_info(&self, speaker_id: i64) -> Result<bool> {
        let path = paths::with_id(paths::SPEAKER_BY_ID, speaker_id);
        let params = json!({ "speakerId": speaker_id }).to_string();
        self.placeholder("DELETE", &path, &params).await
    }

    // ================================= Model =================================

    pub async fn list_model_infos(
        &self,
        request: PageRequest<ModelFilter>,
    ) -> Result<PageResponse<ModelInfo>> {
        self.placeholder("GET", paths::MODELS, &Self::json(&request))
            .await
    }

    pub async fn get_device_type(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<HardwareType> {
        let params = json!({ "baseModel": base_model, "modelVersion": model_version }).to_string();
        self.placeholder("GET", paths::MODELS_DEVICE_TYPE, &params)
            .await
    }

    pub async fn install_model(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult> {
        let path = paths::with_id(paths::MODEL_INSTALL, model_id);
        let params = json!({ "modelId": model_id, "device": device }).to_string();
        self.placeholder("POST", &path, &params).await
    }

    pub async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult> {
        let path = paths::with_id(paths::MODEL_BY_ID, model_id);
        let params = json!({ "modelId": model_id }).to_string();
        self.placeholder("DELETE", &path, &params).await
    }

    pub async fn set_model_current_device(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelInfo> {
        let path = paths::with_id(paths::MODEL_CURRENT_DEVICE, model_id);
        let params = json!({ "modelId": model_id, "device": device }).to_string();
        self.placeholder("PUT", &path, &params).await
    }

    // ================================ History ================================

    pub async fn list_history_records(
        &self,
        request: PageRequest<HistoryFilter>,
    ) -> Result<PageResponse<HistoryRecordSummary>> {
        self.placeholder("GET", paths::HISTORY, &Self::json(&request))
            .await
    }

    pub async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        let path = paths::with_id(paths::HISTORY_BY_ID, history_id);
        let params = json!({ "historyId": history_id }).to_string();
        self.placeholder("GET", &path, &params).await
    }

    pub async fn read_text_to_speech_audio(
        &self,
        history_id: i64,
    ) -> Result<TextToSpeechAudioAsset> {
        let path = paths::with_id(paths::HISTORY_AUDIO_TTS, history_id);
        let params = json!({ "historyId": history_id }).to_string();
        self.placeholder("GET", &path, &params).await
    }

    pub async fn read_voice_clone_audio(&self, history_id: i64) -> Result<VoiceCloneAudioAsset> {
        let path = paths::with_id(paths::HISTORY_AUDIO_VOICE_CLONE, history_id);
        let params = json!({ "historyId": history_id }).to_string();
        self.placeholder("GET", &path, &params).await
    }

    pub async fn read_voice_design_audio(&self, history_id: i64) -> Result<VoiceDesignAudioAsset> {
        let path = paths::with_id(paths::HISTORY_AUDIO_VOICE_DESIGN, history_id);
        let params = json!({ "historyId": history_id }).to_string();
        self.placeholder("GET", &path, &params).await
    }

    pub async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        let path = paths::with_id(paths::HISTORY_BY_ID, history_id);
        let params = json!({ "historyId": history_id, "taskType": task_type }).to_string();
        self.placeholder("DELETE", &path, &params).await
    }

    pub async fn update_task_status(
        &self,
        payload: UpdateTaskStatusPayload,
    ) -> Result<HistoryRecord> {
        let path = paths::with_id(paths::HISTORY_STATUS, payload.task_id);
        self.placeholder("PUT", &path, &Self::json(&payload)).await
    }

    pub async fn create_text_to_speech_task(
        &self,
        payload: CreateTextToSpeechTaskPayload,
    ) -> Result<TextToSpeechTaskResult> {
        self.placeholder("POST", paths::HISTORY_TEXT_TO_SPEECH, &Self::json(&payload))
            .await
    }

    pub async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        self.placeholder("POST", paths::HISTORY_MODEL_TRAINING, &Self::json(&payload))
            .await
    }

    pub async fn cancel_history_task(&self, history_id: i64) -> Result<bool> {
        let path = paths::with_id(paths::HISTORY_CANCEL, history_id);
        let params = json!({ "historyId": history_id }).to_string();
        self.placeholder("POST", &path, &params).await
    }

    pub async fn create_voice_clone_task(
        &self,
        payload: CreateVoiceCloneTaskPayload,
    ) -> Result<VoiceCloneTaskResult> {
        self.placeholder("POST", paths::HISTORY_VOICE_CLONE, &Self::json(&payload))
            .await
    }

    pub async fn create_voice_design_task(
        &self,
        payload: CreateVoiceDesignTaskPayload,
    ) -> Result<VoiceDesignTaskResult> {
        self.placeholder("POST", paths::HISTORY_VOICE_DESIGN, &Self::json(&payload))
            .await
    }
}
