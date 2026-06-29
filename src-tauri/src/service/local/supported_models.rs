use std::{collections::HashSet, fs};

use anyhow::{bail, Context};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde::Deserialize;

use crate::{
    config::{discover_model_config_file_paths, MODEL_CONFIG_FILE_NAME},
    service::{
        local::entity::{model_info as model_info_entity, speaker as speaker_entity},
        models::{AppLanguage, ModelDownloadType, SpeakerSource, SpeakerStatus},
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
    model_version: String,
    #[serde(default = "default_model_download_type")]
    download_type: ModelDownloadType,
    required_model_name_list: Vec<String>,
    required_model_repo_id_list: Vec<String>,
    supported_feature_list: Vec<String>,
    supported_devices: Vec<String>,
    #[serde(default = "default_supported_languages")]
    supported_languages: Vec<AppLanguage>,
}

fn default_model_download_type() -> ModelDownloadType {
    ModelDownloadType::HfLike
}

fn default_supported_languages() -> Vec<AppLanguage> {
    vec![
        AppLanguage::Chinese,
        AppLanguage::English,
        AppLanguage::Japanese,
    ]
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportedSpeakerDefinition {
    name: String,
    base_model: String,
    description: String,
}

pub(crate) async fn sync_supported_models(orm: &DatabaseConnection) -> Result<()> {
    let config = load_supported_models_catalog()?;

    validate_supported_models(&config)?;

    let txn = orm.begin().await?;
    let now = now_string()?;

    let mut active_model_keys = HashSet::new();
    for definition in &config.models {
        active_model_keys.insert(format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.model_version.trim()
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
        let key = format!("{}:{}", row.base_model.trim(), row.model_version.trim());
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

fn load_supported_models_catalog() -> Result<SupportedModelsConfig> {
    let config_paths = discover_model_config_file_paths(MODEL_CONFIG_FILE_NAME)
        .context("发现模型配置文件失败")?;

    let mut models = Vec::new();
    let mut speakers = Vec::new();
    let mut model_keys = HashSet::new();
    let mut speaker_keys = HashSet::new();

    for config_path in config_paths {
        let file_content = fs::read_to_string(&config_path)
            .with_context(|| format!("读取模型配置文件失败: {}", config_path.display()))?;
        let file_config = serde_json::from_str::<SupportedModelsConfig>(&file_content)
            .with_context(|| format!("解析模型配置文件失败: {}", config_path.display()))?;

        for model in file_config.models {
            let key = format!(
                "{}:{}",
                model.base_model.trim(),
                model.model_version.trim()
            );
            if !model_keys.insert(key.clone()) {
                tracing::warn!(
                    key = %key,
                    source = %config_path.display(),
                    "检测到重复模型定义，已跳过"
                );
                continue;
            }
            models.push(model);
        }

        for speaker in file_config.speakers {
            let key = format!("{}:{}", speaker.base_model.trim(), speaker.name.trim());
            if !speaker_keys.insert(key.clone()) {
                tracing::warn!(
                    key = %key,
                    source = %config_path.display(),
                    "检测到重复预置说话人定义，已跳过"
                );
                continue;
            }
            speakers.push(speaker);
        }
    }

    if models.is_empty() {
        bail!(
            "未从 {} 聚合到任何模型定义",
            MODEL_CONFIG_FILE_NAME
        );
    }

    Ok(SupportedModelsConfig { models, speakers })
}

fn validate_supported_models(config: &SupportedModelsConfig) -> Result<()> {
    if config.models.is_empty() {
        bail!("模型配置中至少需要定义一个模型");
    }

    let mut model_keys = HashSet::new();
    for definition in &config.models {
        let key = format!(
            "{}:{}",
            definition.base_model.trim(),
            definition.model_version.trim()
        );
        if !model_keys.insert(key.clone()) {
            bail!("模型配置中存在重复模型定义: {key}");
        }

        if definition.download_type == ModelDownloadType::HfLike
            && definition.required_model_name_list.len()
                != definition.required_model_repo_id_list.len()
        {
            bail!(
                "模型配置中模型 {key} 的 requiredModelNameList 与 requiredModelRepoIdList 长度不一致"
            );
        }

        if definition.supported_devices.is_empty() {
            bail!("模型配置中模型 {key} 的 supportedDevices 不能为空");
        }

        let mut normalized_supported_devices = HashSet::new();
        for device in &definition.supported_devices {
            let normalized = device.trim().to_ascii_lowercase();
            if normalized != "cpu" && normalized != "cuda" {
                bail!("模型配置中模型 {key} 的 supportedDevices 包含非法值: {device}");
            }
            if !normalized_supported_devices.insert(normalized) {
                bail!("模型配置中模型 {key} 的 supportedDevices 包含重复值");
            }
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
            bail!("模型配置中存在重复预置说话人定义: {key}");
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
    let supported_devices_json = serde_json::to_string(
        &definition
            .supported_devices
            .iter()
            .map(|item| item.trim().to_ascii_lowercase())
            .collect::<Vec<String>>(),
    )?;
    let supported_languages_json = serde_json::to_string(&definition.supported_languages)?;

    let existing = model_info_entity::Entity::find()
        .filter(model_info_entity::Column::BaseModel.eq(definition.base_model.trim()))
        .filter(model_info_entity::Column::ModelVersion.eq(definition.model_version.trim()))
        .one(connection)
        .await?;

    if let Some(row) = existing {
        let downloaded = row.downloaded;
        let create_time = row.create_time.clone();
        let mut active_model: model_info_entity::ActiveModel = row.into();
        active_model.base_model = Set(definition.base_model.trim().to_string());
        active_model.model_name = Set(definition.model_name.trim().to_string());
        active_model.model_version = Set(definition.model_version.trim().to_string());
        active_model.download_type = Set(definition.download_type.as_str().to_string());
        active_model.required_model_name_list_json = Set(required_model_name_list_json);
        active_model.required_model_repo_id_list_json = Set(required_model_repo_id_list_json);
        active_model.supported_feature_list_json = Set(supported_feature_list_json);
        active_model.supported_devices = Set(supported_devices_json);
        active_model.supported_languages = Set(supported_languages_json);
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
            model_version: Set(definition.model_version.trim().to_string()),
            download_type: Set(definition.download_type.as_str().to_string()),
            required_model_name_list_json: Set(required_model_name_list_json),
            required_model_repo_id_list_json: Set(required_model_repo_id_list_json),
            supported_feature_list_json: Set(supported_feature_list_json),
            supported_devices: Set(supported_devices_json),
            supported_languages: Set(supported_languages_json),
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
