use async_trait::async_trait;

use crate::{
    service::models::{GeneratedAudioAsset, GeneratedAudioPathAsset, GeneratedAudioSource},
    Result,
};

#[async_trait]
pub trait GeneratedAudioService: Send + Sync {
    async fn read_generated_audio(
        &self,
        source: GeneratedAudioSource,
    ) -> Result<GeneratedAudioAsset>;
    /// 路径级定位（不读文件内容）：前端经 convertFileSrc + asset 协议零拷贝播放。
    async fn resolve_generated_audio_path(
        &self,
        source: GeneratedAudioSource,
    ) -> Result<GeneratedAudioPathAsset>;
    async fn save_generated_audio_as(
        &self,
        source: GeneratedAudioSource,
        app: tauri::AppHandle,
    ) -> Result<bool>;
}
