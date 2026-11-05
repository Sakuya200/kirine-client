use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{
            CreateSpeakerPayload, ImportModelAsSpeakerPayload, PageRequest, SpeakerAvatarAsset,
            SpeakerFilter, SpeakerInfo, SpeakerPageResult, UpdateSpeakerPayload,
        },
        speaker::SpeakerService,
    },
    Result,
};
use anyhow::bail;

#[async_trait]
impl SpeakerService for RemoteService {
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

    async fn get_speaker_avatar(&self, speaker_id: i64) -> Result<SpeakerAvatarAsset> {
        bail!("远程模式暂不支持读取说话人头像: {speaker_id}")
    }
}
