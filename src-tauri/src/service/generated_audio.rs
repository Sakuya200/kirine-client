use async_trait::async_trait;

use crate::{
    service::models::{GeneratedAudioAsset, GeneratedAudioSource},
    Result,
};

#[async_trait]
pub trait GeneratedAudioService: Send + Sync {
    async fn read_generated_audio(
        &self,
        source: GeneratedAudioSource,
    ) -> Result<GeneratedAudioAsset>;
    async fn save_generated_audio_as(
        &self,
        source: GeneratedAudioSource,
        app: tauri::AppHandle,
    ) -> Result<bool>;
}
