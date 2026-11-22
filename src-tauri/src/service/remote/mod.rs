mod generated_audio;
mod history;
mod model;
mod speaker;
mod streaming;
mod training;
mod tts;
mod voice_clone;
mod voice_design;

use async_trait::async_trait;

use anyhow::Context as _;

use crate::{client::ApiClient, config::EnvConfig, service::Service, Result};

#[derive(Debug, Clone)]
pub struct RemoteService {
    client: ApiClient,
}

/// 远程模式暂只覆盖部分业务域；流式语音等能力在此层直接返回占位错误。
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

    fn speaker_service(&self) -> &dyn crate::service::SpeakerService {
        self
    }

    fn model_service(&self) -> &dyn crate::service::ModelService {
        self
    }

    fn history_service(&self) -> &dyn crate::service::HistoryService {
        self
    }

    fn generated_audio_service(&self) -> &dyn crate::service::GeneratedAudioService {
        self
    }

    fn text_to_speech_service(&self) -> &dyn crate::service::TextToSpeechService {
        self
    }

    fn model_training_service(&self) -> &dyn crate::service::ModelTrainingService {
        self
    }

    fn voice_clone_service(&self) -> &dyn crate::service::VoiceCloneService {
        self
    }

    fn voice_design_service(&self) -> &dyn crate::service::VoiceDesignService {
        self
    }

    fn streaming_speech_service(&self) -> &dyn crate::service::StreamingSpeechService {
        self
    }
}
