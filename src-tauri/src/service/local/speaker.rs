use std::{fs, io, path::{Path, PathBuf}};

use anyhow::{bail, Context};

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait,
    EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

use crate::{
    common::local_paths::serialize_runtime_model_path,
    service::{
        local::entity::speaker as speaker_entity,
        models::{
            AppLanguage, CreateSpeakerPayload, ImportModelAsSpeakerPayload, ModelInfo,
            SpeakerInfo, SpeakerSource, SpeakerStatus, UpdateSpeakerPayload,
        },
        pipeline::{
            model_paths::speaker_model_dir,
            resolve_inference_model_path,
            script_paths::resolve_src_model_root,
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

    pub(crate) async fn import_model_as_speaker_impl(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        let create_time = now_string()?;
        let name = payload.name.trim();
        let description = payload.description.trim();
        let base_model = payload.base_model.trim();
        let model_scale = payload.model_scale.trim();
        let source_model_dir = PathBuf::from(payload.source_model_dir_path.trim());

        if name.is_empty() {
            bail!("说话人名称不能为空");
        }
        if description.is_empty() {
            bail!("说话人描述不能为空");
        }
        if base_model.is_empty() {
            bail!("基础模型类型不能为空");
        }
        if model_scale.is_empty() {
            bail!("模型参数大小不能为空");
        }
        if !source_model_dir.is_dir() {
            bail!("模型目录不存在或不是目录: {}", source_model_dir.display());
        }

        self.find_supported_model_variant(base_model, model_scale).await?;
        resolve_inference_model_path(base_model, &source_model_dir)
            .with_context(|| format!("导入目录不满足 {} {} 的推理模型结构", base_model, model_scale))?;

        let src_model_root = resolve_src_model_root(self.app_dir())?;
        let languages_json = serde_json::to_string(&vec![payload.language])?;
        let txn = self.orm().begin().await?;

        let inserted = speaker_entity::ActiveModel {
            id: NotSet,
            name: Set(name.to_string()),
            languages_json: Set(languages_json),
            samples: Set(0),
            base_model: Set(base_model.to_string()),
            description: Set(description.to_string()),
            model_path: Set(Some(String::new())),
            status: Set(SpeakerStatus::Ready.as_str().to_string()),
            source: Set(SpeakerSource::Local.as_str().to_string()),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;

        let managed_model_dir = speaker_model_dir(
            Path::new(self.model_dir()),
            inserted.id,
            &super::sanitize_path_segment(name),
        );
        if managed_model_dir.exists() {
            bail!("目标模型目录已存在，请更换说话人名称后重试: {}", managed_model_dir.display());
        }

        if let Err(err) = copy_directory_recursively(&source_model_dir, &managed_model_dir) {
            let _ = fs::remove_dir_all(&managed_model_dir);
            return Err(err);
        }

        resolve_inference_model_path(base_model, &managed_model_dir)
            .with_context(|| format!("复制后的模型目录不满足 {} {} 的推理模型结构", base_model, model_scale))?;

        let mut active_model: speaker_entity::ActiveModel = inserted.clone().into();
        active_model.model_path = Set(Some(serialize_runtime_model_path(
            Path::new(self.model_dir()),
            &src_model_root,
            &managed_model_dir,
        )));
        active_model.update(&txn).await?;

        txn.commit().await?;
        map_speaker_model(inserted)
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

impl LocalService {
    async fn find_supported_model_variant(&self, base_model: &str, model_scale: &str) -> Result<ModelInfo> {
        use crate::service::local::entity::model_info as model_info_entity;

        let row = model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .filter(model_info_entity::Column::BaseModel.eq(base_model))
            .filter(model_info_entity::Column::ModelScale.eq(model_scale))
            .one(self.orm())
            .await?
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("当前应用不支持该模型类型或规模: {base_model} {model_scale}")))?;

        Ok(ModelInfo {
            id: row.id,
            base_model: row.base_model,
            model_name: row.model_name,
            model_scale: row.model_scale,
            required_model_name_list: serde_json::from_str(&row.required_model_name_list_json)?,
            required_model_repo_id_list: serde_json::from_str(&row.required_model_repo_id_list_json)?,
            supported_feature_list: serde_json::from_str(&row.supported_feature_list_json)?,
            downloaded: row.downloaded,
            create_time: row.create_time,
            modify_time: row.modify_time,
        })
    }
}

fn copy_directory_recursively(source_dir: &Path, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir)
        .with_context(|| format!("failed to create target model directory: {}", target_dir.display()))?;

    for entry in fs::read_dir(source_dir)
        .with_context(|| format!("failed to inspect source model directory: {}", source_dir.display()))?
    {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_directory_recursively(&source_path, &target_path)?;
        } else if file_type.is_file() {
            fs::copy(&source_path, &target_path).with_context(|| {
                format!(
                    "failed to copy model file from {} to {}",
                    source_path.display(),
                    target_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn map_speaker_model(model: speaker_entity::Model) -> Result<SpeakerInfo> {
    let languages = serde_json::from_str::<Vec<AppLanguage>>(&model.languages_json)?;
    Ok(SpeakerInfo {
        id: model.id,
        name: model.name,
        languages,
        samples: model.samples as u32,
        base_model: model.base_model,
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
