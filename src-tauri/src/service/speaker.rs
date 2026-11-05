use async_trait::async_trait;

use crate::{
    service::models::{
        CreateSpeakerPayload, ImportModelAsSpeakerPayload, PageRequest, SpeakerAvatarAsset,
        SpeakerFilter, SpeakerInfo, SpeakerPageResult, UpdateSpeakerPayload,
    },
    Result,
};

#[async_trait]
pub trait SpeakerService: Send + Sync {
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
    async fn get_speaker_avatar(&self, speaker_id: i64) -> Result<SpeakerAvatarAsset>;
}
