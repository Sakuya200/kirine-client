use std::io;

use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::de::DeserializeOwned;

use crate::{
    service::{
        local::entity::model_info as model_info_entity,
        models::{HistoryTaskType, ModelInfo},
        LocalService,
    },
    Result,
};

impl LocalService {
    pub(crate) async fn list_model_infos_impl(&self) -> Result<Vec<ModelInfo>> {
        let rows = model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .order_by_asc(model_info_entity::Column::Id)
            .all(self.orm())
            .await?;

        rows.into_iter().map(map_model_info).collect()
    }
}

fn map_model_info(row: model_info_entity::Model) -> Result<ModelInfo> {
    Ok(ModelInfo {
        id: row.id,
        base_model: row.base_model,
        model_name: row.model_name,
        model_scale: row.model_scale,
        required_model_name_list: parse_json_field(&row.required_model_name_list_json)?,
        required_model_repo_id_list: parse_json_field(&row.required_model_repo_id_list_json)?,
        supported_feature_list: parse_json_field::<Vec<String>>(&row.supported_feature_list_json)?
            .into_iter()
            .map(|value| {
                value
                    .parse::<HistoryTaskType>()
                    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
            })
            .collect::<std::result::Result<Vec<_>, _>>()?,
        create_time: row.create_time,
        modify_time: row.modify_time,
    })
}

fn parse_json_field<T>(value: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(value).or_else(|first_err| {
        let normalized = value.replace(r#"\""#, r#"""#);
        serde_json::from_str(&normalized).map_err(|_| first_err.into())
    })
}
