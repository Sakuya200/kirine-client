use std::io;

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait,
    EntityTrait, QueryFilter, QueryOrder,
};

use crate::{
    service::{
        local::entity::speaker as speaker_entity,
        models::{
            AppLanguage, CreateSpeakerPayload, SpeakerInfo, SpeakerSource, SpeakerStatus,
            UpdateSpeakerPayload,
        },
        LocalService,
    },
    utils::time::now_string,
    Result,
};

impl LocalService {
    pub(crate) async fn create_speaker_info_impl(
        &self,
        payload: CreateSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        let create_time = now_string()?;
        let languages = if payload.languages.is_empty() {
            vec![AppLanguage::Chinese]
        } else {
            payload.languages
        };
        let languages_json = serde_json::to_string(&languages)?;
        let name = payload.name.trim();
        let description = payload.description.trim();
        let status = payload.status;
        let source = payload.source;

        let inserted = speaker_entity::ActiveModel {
            id: NotSet,
            name: Set(name.to_string()),
            languages_json: Set(languages_json),
            samples: Set(payload.samples as i64),
            base_model: Set(payload.base_model.as_str().to_string()),
            description: Set(description.to_string()),
            model_path: Set(None),
            status: Set(status.as_str().to_string()),
            source: Set(source.as_str().to_string()),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        }
        .insert(self.orm())
        .await?;

        map_speaker_model(inserted)
    }

    pub(crate) async fn list_speaker_infos_impl(&self) -> Result<Vec<SpeakerInfo>> {
        speaker_entity::Entity::find()
            .filter(speaker_entity::Column::Deleted.eq(0))
            .order_by_desc(speaker_entity::Column::ModifyTime)
            .order_by_desc(speaker_entity::Column::CreateTime)
            .all(self.orm())
            .await?
            .into_iter()
            .map(map_speaker_model)
            .collect()
    }

    pub(crate) async fn update_speaker_info_impl(
        &self,
        payload: UpdateSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        let modify_time = now_string()?;
        let speaker = speaker_entity::Entity::find_by_id(payload.id)
            .filter(speaker_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到目标说话人"))?;

        let mut active_model: speaker_entity::ActiveModel = speaker.into();
        active_model.name = Set(payload.name.trim().to_string());
        active_model.description = Set(payload.description.trim().to_string());
        active_model.modify_time = Set(modify_time);

        let updated = active_model.update(self.orm()).await?;
        map_speaker_model(updated)
    }

    pub(crate) async fn delete_speaker_info_impl(&self, speaker_id: i64) -> Result<bool> {
        let modify_time = now_string()?;
        let result = speaker_entity::Entity::update_many()
            .col_expr(speaker_entity::Column::Deleted, Expr::value(1))
            .col_expr(speaker_entity::Column::ModifyTime, Expr::value(modify_time))
            .filter(speaker_entity::Column::Id.eq(speaker_id))
            .filter(speaker_entity::Column::Deleted.eq(0))
            .exec(self.orm())
            .await?;

        Ok(result.rows_affected > 0)
    }
}

fn map_speaker_model(model: speaker_entity::Model) -> Result<SpeakerInfo> {
    let languages = serde_json::from_str::<Vec<AppLanguage>>(&model.languages_json)?;
    Ok(SpeakerInfo {
        id: model.id,
        name: model.name,
        languages,
        samples: model.samples as u32,
        base_model: model
            .base_model
            .parse::<crate::config::BaseModel>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
        create_time: model.create_time,
        modify_time: model.modify_time,
        description: model.description,
        status: model
            .status
            .parse::<SpeakerStatus>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
        source: model
            .source
            .parse::<SpeakerSource>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
    })
}
