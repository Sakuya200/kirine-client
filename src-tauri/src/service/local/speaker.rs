use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};

use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait,
    Condition, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::{
    config::HardwareType,
    service::{
        local::entity::speaker as speaker_entity,
        models::{
            CreateSpeakerPayload, ImportModelAsSpeakerPayload, ModelDownloadType, ModelInfo,
            PageRequest, SpeakerFilter, SpeakerInfo, SpeakerPageResult, SpeakerSource,
            SpeakerStatus, UpdateSpeakerPayload,
        },
        pipeline::model_paths::speaker_model_dir,
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
        let name = payload.speaker_name.trim();
        let description = payload.description.trim();
        let status = payload.status;
        let source = payload.source;

        let inserted = speaker_entity::ActiveModel {
            id: NotSet,
            speaker_name: Set(name.to_string()),
            samples: Set(payload.samples as i64),
            base_model: Set(payload.base_model.as_str().to_string()),
            description: Set(description.to_string()),
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

    pub(crate) async fn list_speaker_infos_impl(
        &self,
        request: PageRequest<SpeakerFilter>,
    ) -> Result<SpeakerPageResult> {
        let page = request.page.max(1);
        let page_size = request.page_size.max(1);

        // 统一构造筛选条件，分页查询与各统计查询共用，避免重复拼装
        let mut condition = Condition::all();
        if let Some(filter) = &request.filter {
            if let Some(keyword) = filter
                .keyword
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                let pattern = format!("%{keyword}%");
                condition = condition.add(speaker_entity::Column::SpeakerName.like(&pattern));
            }
            if let Some(status) = filter.status {
                condition = condition.add(speaker_entity::Column::Status.eq(status.as_str()));
            }
        }

        // 与 model_info / history 一致：使用 sea-orm paginate 取当前页与总数
        let paginator = speaker_entity::Entity::find()
            .filter(speaker_entity::Column::Deleted.eq(0))
            .filter(condition.clone())
            .order_by_desc(speaker_entity::Column::ModifyTime)
            .order_by_desc(speaker_entity::Column::CreateTime)
            .paginate(self.orm(), page_size as u64);
        let total = paginator.num_items().await?;
        let rows = paginator.fetch_page((page - 1) as u64).await?;

        // 统计基于同一筛选条件，用聚焦的 count / 聚合查询避免一次性载入全量
        let base_query = || {
            speaker_entity::Entity::find()
                .filter(speaker_entity::Column::Deleted.eq(0))
                .filter(condition.clone())
        };

        let ready_count = base_query()
            .filter(speaker_entity::Column::Status.eq(SpeakerStatus::Ready.as_str()))
            .count(self.orm())
            .await?;
        let training_count = base_query()
            .filter(speaker_entity::Column::Status.eq(SpeakerStatus::Training.as_str()))
            .count(self.orm())
            .await?;
        let disabled_count = base_query()
            .filter(speaker_entity::Column::Status.eq(SpeakerStatus::Disabled.as_str()))
            .count(self.orm())
            .await?;
        let total_samples = base_query()
            .select_only()
            .column_as(
                Expr::col(speaker_entity::Column::Samples).sum(),
                "total_samples",
            )
            .into_model::<SpeakerSampleSum>()
            .one(self.orm())
            .await?
            .and_then(|row| row.total_samples)
            .unwrap_or(0) as u64;

        let items: Result<Vec<SpeakerInfo>> = rows.into_iter().map(map_speaker_model).collect();
        let items = items?;

        let total_pages = if page_size == 0 {
            0
        } else {
            (total as u32 + page_size - 1) / page_size
        };

        Ok(SpeakerPageResult {
            items,
            total,
            page,
            page_size,
            total_pages,
            ready_count,
            training_count,
            disabled_count,
            total_samples,
        })
    }

    pub(crate) async fn import_model_as_speaker_impl(
        &self,
        payload: ImportModelAsSpeakerPayload,
    ) -> Result<SpeakerInfo> {
        let create_time = now_string()?;
        let name = payload.speaker_name.trim();
        let description = payload.description.trim();
        let base_model = payload.base_model.trim();
        let model_version = payload.model_version.trim();
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
        if model_version.is_empty() {
            bail!("模型版本不能为空");
        }
        if !source_model_dir.is_dir() {
            bail!("模型目录不存在或不是目录: {}", source_model_dir.display());
        }

        self.find_supported_model_variant(base_model, model_version)
            .await?;

        let txn = self.orm().begin().await?;

        let inserted = speaker_entity::ActiveModel {
            id: NotSet,
            speaker_name: Set(name.to_string()),
            samples: Set(0),
            base_model: Set(base_model.to_string()),
            description: Set(description.to_string()),
            status: Set(SpeakerStatus::Ready.as_str().to_string()),
            source: Set(SpeakerSource::Local.as_str().to_string()),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;

        let managed_model_dir = speaker_model_dir(Path::new(self.model_dir()), inserted.id);
        if managed_model_dir.exists() {
            bail!(
                "目标模型目录已存在，请更换说话人名称后重试: {}",
                managed_model_dir.display()
            );
        }

        if let Err(err) = copy_directory_recursively(&source_model_dir, &managed_model_dir) {
            let _ = fs::remove_dir_all(&managed_model_dir);
            return Err(err);
        }
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
        active_model.speaker_name = Set(payload.speaker_name.trim().to_string());
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
    pub(crate) async fn find_supported_model_variant(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<ModelInfo> {
        use crate::service::local::entity::model_info as model_info_entity;

        let row = model_info_entity::Entity::find()
            .filter(model_info_entity::Column::Deleted.eq(0))
            .filter(model_info_entity::Column::BaseModel.eq(base_model))
            .filter(model_info_entity::Column::ModelVersion.eq(model_version))
            .one(self.orm())
            .await?
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("当前应用不支持该模型类型或版本: {base_model} {model_version}"),
                )
            })?;

        Ok(ModelInfo {
            id: row.id,
            base_model: row.base_model,
            model_name: row.model_name,
            model_version: row.model_version,
            download_type: row
                .download_type
                .parse()
                .unwrap_or(ModelDownloadType::HfLike),
            required_model_name_list: serde_json::from_str(&row.required_model_name_list_json)?,
            required_model_repo_id_list: serde_json::from_str(
                &row.required_model_repo_id_list_json,
            )?,
            supported_feature_list: serde_json::from_str(&row.supported_feature_list_json)?,
            supported_devices: serde_json::from_str(&row.supported_devices)?,
            current_device: row
                .current_device
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .and_then(|value| value.parse::<HardwareType>().ok()),
            supported_languages: serde_json::from_str(&row.supported_languages)?,
            downloaded: row.downloaded,
            create_time: row.create_time,
            modify_time: row.modify_time,
        })
    }
}

fn copy_directory_recursively(source_dir: &Path, target_dir: &Path) -> Result<()> {
    fs::create_dir_all(target_dir).with_context(|| {
        format!(
            "failed to create target model directory: {}",
            target_dir.display()
        )
    })?;

    for entry in fs::read_dir(source_dir).with_context(|| {
        format!(
            "failed to inspect source model directory: {}",
            source_dir.display()
        )
    })? {
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

#[derive(FromQueryResult)]
struct SpeakerSampleSum {
    total_samples: Option<i64>,
}

fn map_speaker_model(model: speaker_entity::Model) -> Result<SpeakerInfo> {
    Ok(SpeakerInfo {
        id: model.id,
        speaker_name: model.speaker_name,
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
