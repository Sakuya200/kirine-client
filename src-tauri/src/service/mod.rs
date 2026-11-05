pub(crate) mod local;
pub mod models;
pub(crate) mod pipeline;
mod remote;

// 各业务服务接口定义
mod generated_audio;
mod history;
mod model;
mod speaker;
mod streaming;
mod training;
mod tts;
mod voice_clone;
mod voice_design;

use anyhow::Error;
pub(crate) use local::entity;
pub use local::LocalService;
pub use remote::RemoteService;

pub use generated_audio::GeneratedAudioService;
pub use history::HistoryService;
pub use model::ModelService;
pub use speaker::SpeakerService;
pub use streaming::StreamingSpeechService;
pub use training::ModelTrainingService;
pub use tts::TextToSpeechService;
pub use voice_clone::VoiceCloneService;
pub use voice_design::VoiceDesignService;

use crate::config::{EnvConfig, StorageMode};
use crate::{service, Result};
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub enum ServiceImpl {
    Local(LocalService),
    Remote(RemoteService),
}

pub struct ServiceState {
    service: ServiceImpl,
}

impl ServiceState {
    pub fn new(service: ServiceImpl) -> Self {
        Self { service }
    }

    pub fn service(&self) -> Result<&(dyn Service + Send + Sync)> {
        self.service.service()
    }

    pub fn local_service(&self) -> Result<&LocalService> {
        match &self.service {
            ServiceImpl::Local(local) => Ok(local),
            ServiceImpl::Remote(_) => Err(Error::msg("Not a local service")),
        }
    }
}

impl ServiceImpl {
    pub fn service(&self) -> Result<&(dyn Service + Send + Sync)> {
        match self {
            ServiceImpl::Local(local) => Ok(local),
            ServiceImpl::Remote(remote) => Ok(remote),
        }
    }

    pub fn speaker_service(&self) -> Result<&dyn SpeakerService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.speaker_service()),
            ServiceImpl::Remote(remote) => Ok(remote.speaker_service()),
        }
    }

    pub fn model_service(&self) -> Result<&dyn ModelService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.model_service()),
            ServiceImpl::Remote(remote) => Ok(remote.model_service()),
        }
    }

    pub fn history_service(&self) -> Result<&dyn HistoryService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.history_service()),
            ServiceImpl::Remote(remote) => Ok(remote.history_service()),
        }
    }

    pub fn generated_audio_service(&self) -> Result<&dyn GeneratedAudioService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.generated_audio_service()),
            ServiceImpl::Remote(remote) => Ok(remote.generated_audio_service()),
        }
    }

    pub fn text_to_speech_service(&self) -> Result<&dyn TextToSpeechService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.text_to_speech_service()),
            ServiceImpl::Remote(remote) => Ok(remote.text_to_speech_service()),
        }
    }

    pub fn model_training_service(&self) -> Result<&dyn ModelTrainingService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.model_training_service()),
            ServiceImpl::Remote(remote) => Ok(remote.model_training_service()),
        }
    }

    pub fn voice_clone_service(&self) -> Result<&dyn VoiceCloneService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.voice_clone_service()),
            ServiceImpl::Remote(remote) => Ok(remote.voice_clone_service()),
        }
    }

    pub fn voice_design_service(&self) -> Result<&dyn VoiceDesignService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.voice_design_service()),
            ServiceImpl::Remote(remote) => Ok(remote.voice_design_service()),
        }
    }

    pub fn streaming_speech_service(&self) -> Result<&dyn StreamingSpeechService> {
        match self {
            ServiceImpl::Local(local) => Ok(local.streaming_speech_service()),
            ServiceImpl::Remote(remote) => Ok(remote.streaming_speech_service()),
        }
    }
}

/// 生命周期入口：只负责构造/关闭具体实现，业务能力一律经各域服务接口访问。
#[async_trait]
pub trait Service: Send + Sync {
    async fn new(config: &EnvConfig) -> Result<Self>
    where
        Self: Sized;
    async fn close(&self) -> Result<()>;

    fn speaker_service(&self) -> &dyn SpeakerService;
    fn model_service(&self) -> &dyn ModelService;
    fn history_service(&self) -> &dyn HistoryService;
    fn generated_audio_service(&self) -> &dyn GeneratedAudioService;
    fn text_to_speech_service(&self) -> &dyn TextToSpeechService;
    fn model_training_service(&self) -> &dyn ModelTrainingService;
    fn voice_clone_service(&self) -> &dyn VoiceCloneService;
    fn voice_design_service(&self) -> &dyn VoiceDesignService;
    fn streaming_speech_service(&self) -> &dyn StreamingSpeechService;
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
