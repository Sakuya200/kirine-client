use std::{collections::HashSet, fs};

use anyhow::{bail, Context};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;

use crate::{
    config::supported_models_path,
    service::{
        local::entity::{model_info as model_info_entity, speaker as speaker_entity},
        models::{AppLanguage, SpeakerSource, SpeakerStatus},
    },
    utils::time::now_string,
    Result,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportedModelsConfig {
    models: Vec<SupportedModelDefinition>,
    speakers: Vec<SupportedSpeakerDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportedModelDefinition {
    base_model: String,
    model_name: String,
    model_scale: String,
    required_model_name_list: Vec<String>,
    required_model_repo_id_list: Vec<String>,
    supported_feature_list: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportedSpeakerDefinition {
    name: String,
    languages: Vec<AppLanguage>,
    base_model: String,
    description: String,
}

pub(crate) async fn sync_supported_models(orm: &DatabaseConnection) -> Result<()> {
    let config_path = supported_models_path().context("解析 supported_models.json 路径失败")?;
    let file_content = fs::read_to_string(&config_path)
        .with_context(|| format!("读取 supported_models.json 失败: {}", config_path.display()))?;
    let config: SupportedModelsConfig = serde_json::from_str(&file_content)
        .with_context(|| format!("解析 supported_models.json 失败: {}", config_path.display()))?;

    validate_supported_models(&config)?;

    let txn = orm.begin().await?;
    let now = now_string()?;

    let mut active_model_keys = HashSet::new();
    for definition in &config.models {
        active_model_keys.insert(format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.model_scale.trim()
        ));
        upsert_model_definition(&txn, definition, &now).await?;
    }

    let mut active_speaker_keys = HashSet::new();
    for definition in &config.speakers {
        active_speaker_keys.insert(format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.name.trim()
        ));
        upsert_speaker_definition(&txn, definition, &now).await?;
    }

    let existing_models = model_info_entity::Entity::find().all(&txn).await?;
    for row in existing_models {
        let key = format!("{}:{}", row.base_model.trim(), row.model_scale.trim());
        if active_model_keys.contains(&key) {
            continue;
        }

        let mut active_model: model_info_entity::ActiveModel = row.into();
        active_model.deleted = Set(1);
        active_model.modify_time = Set(now.clone());
        active_model.update(&txn).await?;
    }

    let existing_preset_speakers = speaker_entity::Entity::find()
        .filter(speaker_entity::Column::Source.eq(SpeakerSource::Preset.as_str()))
        .all(&txn)
        .await?;
    for row in existing_preset_speakers {
        let key = format!("{}:{}", row.base_model.trim(), row.name.trim());
        if active_speaker_keys.contains(&key) {
            continue;
        }

        let mut active_model: speaker_entity::ActiveModel = row.into();
        active_model.deleted = Set(1);
        active_model.modify_time = Set(now.clone());
        active_model.update(&txn).await?;
    }

    txn.commit().await?;
    Ok(())
}

fn validate_supported_models(config: &SupportedModelsConfig) -> Result<()> {
    if config.models.is_empty() {
        bail!("supported_models.json 中至少需要定义一个模型");
    }

    let mut model_keys = HashSet::new();
    for definition in &config.models {
        let key = format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.model_scale.trim()
        );
        if !model_keys.insert(key.clone()) {
            bail!("supported_models.json 中存在重复模型定义: {key}");
        }
    }

    let mut speaker_keys = HashSet::new();
    for definition in &config.speakers {
        let key = format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.name.trim()
        );
        if !speaker_keys.insert(key.clone()) {
            bail!("supported_models.json 中存在重复预置说话人定义: {key}");
        }
    }

    Ok(())
}

async fn upsert_model_definition<C>(
    connection: &C,
    definition: &SupportedModelDefinition,
    now: &str,
) -> Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    let required_model_name_list_json =
        serde_json::to_string(&definition.required_model_name_list)?;
    let required_model_repo_id_list_json =
        serde_json::to_string(&definition.required_model_repo_id_list)?;
    let supported_feature_list_json = serde_json::to_string(&definition.supported_feature_list)?;

    let existing = model_info_entity::Entity::find()
        .filter(model_info_entity::Column::BaseModel.eq(definition.base_model.trim()))
        .filter(model_info_entity::Column::ModelScale.eq(definition.model_scale.trim()))
        .one(connection)
        .await?;

    if let Some(row) = existing {
        let downloaded = row.downloaded;
        let create_time = row.create_time.clone();
        let mut active_model: model_info_entity::ActiveModel = row.into();
        active_model.base_model = Set(definition.base_model.trim().to_string());
        active_model.model_name = Set(definition.model_name.trim().to_string());
        active_model.model_scale = Set(definition.model_scale.trim().to_string());
        active_model.required_model_name_list_json = Set(required_model_name_list_json);
        active_model.required_model_repo_id_list_json = Set(required_model_repo_id_list_json);
        active_model.supported_feature_list_json = Set(supported_feature_list_json);
        active_model.downloaded = Set(downloaded);
        active_model.create_time = Set(create_time);
        active_model.modify_time = Set(now.to_string());
        active_model.deleted = Set(0);
        active_model.update(connection).await?;
    } else {
        model_info_entity::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            base_model: Set(definition.base_model.trim().to_string()),
            model_name: Set(definition.model_name.trim().to_string()),
            model_scale: Set(definition.model_scale.trim().to_string()),
            required_model_name_list_json: Set(required_model_name_list_json),
            required_model_repo_id_list_json: Set(required_model_repo_id_list_json),
            supported_feature_list_json: Set(supported_feature_list_json),
            create_time: Set(now.to_string()),
            modify_time: Set(now.to_string()),
            downloaded: Set(false),
            deleted: Set(0),
        }
        .insert(connection)
        .await?;
    }

    Ok(())
}

async fn upsert_speaker_definition<C>(
    connection: &C,
    definition: &SupportedSpeakerDefinition,
    now: &str,
) -> Result<()>
where
    C: sea_orm::ConnectionTrait,
{
    let languages_json = serde_json::to_string(&definition.languages)?;

    let existing = speaker_entity::Entity::find()
        .filter(speaker_entity::Column::BaseModel.eq(definition.base_model.trim()))
        .filter(speaker_entity::Column::Name.eq(definition.name.trim()))
        .filter(speaker_entity::Column::Source.eq(SpeakerSource::Preset.as_str()))
        .one(connection)
        .await?;

    if let Some(row) = existing {
        let create_time = row.create_time.clone();
        let mut active_model: speaker_entity::ActiveModel = row.into();
        active_model.name = Set(definition.name.trim().to_string());
        active_model.languages_json = Set(languages_json);
        active_model.samples = Set(0);
        active_model.base_model = Set(definition.base_model.trim().to_string());
        active_model.description = Set(definition.description.trim().to_string());
        active_model.status = Set(SpeakerStatus::Ready.as_str().to_string());
        active_model.source = Set(SpeakerSource::Preset.as_str().to_string());
        active_model.create_time = Set(create_time);
        active_model.modify_time = Set(now.to_string());
        active_model.deleted = Set(0);
        active_model.update(connection).await?;
    } else {
        speaker_entity::ActiveModel {
            id: sea_orm::ActiveValue::NotSet,
            name: Set(definition.name.trim().to_string()),
            languages_json: Set(languages_json),
            samples: Set(0),
            base_model: Set(definition.base_model.trim().to_string()),
            description: Set(definition.description.trim().to_string()),
            status: Set(SpeakerStatus::Ready.as_str().to_string()),
            source: Set(SpeakerSource::Preset.as_str().to_string()),
            create_time: Set(now.to_string()),
            modify_time: Set(now.to_string()),
            deleted: Set(0),
        }
        .insert(connection)
        .await?;
    }

    Ok(())
}
