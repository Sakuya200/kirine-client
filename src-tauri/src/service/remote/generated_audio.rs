use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        generated_audio::GeneratedAudioService,
        models::{GeneratedAudioAsset, GeneratedAudioSource},
    },
    Result,
};
use anyhow::bail;

#[async_trait]
impl GeneratedAudioService for RemoteService {
    async fn read_generated_audio(
        &self,
        source: GeneratedAudioSource,
    ) -> Result<GeneratedAudioAsset> {
        match source {
            GeneratedAudioSource::TextToSpeech { history_id } => {
                let asset = self.client.read_text_to_speech_audio(history_id).await?;
                Ok(GeneratedAudioAsset {
                    file_name: asset.file_name,
                    content_type: asset.content_type,
                    bytes: asset.bytes,
                })
            }
            GeneratedAudioSource::VoiceClone { history_id } => {
                let asset = self.client.read_voice_clone_audio(history_id).await?;
                Ok(GeneratedAudioAsset {
                    file_name: asset.file_name,
                    content_type: asset.content_type,
                    bytes: asset.bytes,
                })
            }
            GeneratedAudioSource::VoiceDesign { history_id } => {
                let asset = self.client.read_voice_design_audio(history_id).await?;
                Ok(GeneratedAudioAsset {
                    file_name: asset.file_name,
                    content_type: asset.content_type,
                    bytes: asset.bytes,
                })
            }
            GeneratedAudioSource::StreamingSpeech { .. } => {
                bail!("远程存储模式暂不支持流式语音音频播放")
            }
        }
    }

    async fn save_generated_audio_as(
        &self,
        source: GeneratedAudioSource,
        app: tauri::AppHandle,
    ) -> Result<bool> {
        match source {
            GeneratedAudioSource::TextToSpeech { history_id } => {
                let asset = self.client.read_text_to_speech_audio(history_id).await?;
                crate::utils::audio::save_audio_bytes_as(&app, &asset.file_name, &asset.bytes)
                    .map_err(|err| anyhow::anyhow!(err))
            }
            GeneratedAudioSource::VoiceClone { history_id } => {
                let asset = self.client.read_voice_clone_audio(history_id).await?;
                crate::utils::audio::save_audio_bytes_as(&app, &asset.file_name, &asset.bytes)
                    .map_err(|err| anyhow::anyhow!(err))
            }
            GeneratedAudioSource::VoiceDesign { history_id } => {
                let asset = self.client.read_voice_design_audio(history_id).await?;
                crate::utils::audio::save_audio_bytes_as(&app, &asset.file_name, &asset.bytes)
                    .map_err(|err| anyhow::anyhow!(err))
            }
            GeneratedAudioSource::StreamingSpeech { .. } => {
                bail!("远程存储模式暂不支持流式语音音频下载")
            }
        }
    }
}
