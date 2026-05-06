use anyhow::Context;
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use sea_orm_migration::prelude::*;

use crate::{
    config::application_dir,
    service::{
        entity::model_info as model_info_entity,
        pipeline::{
            model_artifacts::{all_model_artifact_dirs_ready, MODEL_ARTIFACTS_DIR},
            script_paths::resolve_src_model_root,
        },
    },
    Result,
};

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260423_000001_add_model_info_downloaded_flag"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        add_downloaded_column(manager).await?;
        backfill_downloaded_flags(connection, backend)
            .await
            .map_err(|err| DbErr::Custom(err.to_string()))?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn add_downloaded_column(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let has_column = manager.has_column("model_info", "downloaded").await?;
    if has_column {
        return Ok(());
    }

    manager
        .alter_table(
            Table::alter()
                .table(Alias::new("model_info"))
                .add_column(
                    ColumnDef::new(Alias::new("downloaded"))
                        .boolean()
                        .not_null()
                        .default(false),
                )
                .to_owned(),
        )
        .await
}

async fn backfill_downloaded_flags<C>(db: &C, backend: DbBackend) -> Result<()>
where
    C: ConnectionTrait,
{
    let app_dir = application_dir()?;
    let src_model_root = match resolve_src_model_root(&app_dir) {
        Ok(path) => path,
        Err(_) => app_dir.join("src-model"),
    };

    let rows = model_info_entity::Entity::find()
        .filter(model_info_entity::Column::Deleted.eq(0))
        .all(db)
        .await?;

    for row in rows {
        let required_model_name_list: Vec<String> =
            serde_json::from_str(&row.required_model_name_list_json).with_context(|| {
                format!(
                    "failed to parse required_model_name_list_json for {}:{}",
                    row.base_model, row.model_scale
                )
            })?;
        let download_paths = required_model_name_list
            .iter()
            .map(|artifact_name| src_model_root.join(MODEL_ARTIFACTS_DIR).join(artifact_name))
            .collect::<Vec<_>>();
        let downloaded = all_model_artifact_dirs_ready(&download_paths);

        db.execute(Statement::from_sql_and_values(
            backend,
            "UPDATE model_info SET downloaded = ? WHERE base_model = ? AND model_scale = ? AND deleted = 0",
            vec![
                downloaded.into(),
                row.base_model.clone().into(),
                row.model_scale.clone().into(),
            ],
        ))
        .await
        .with_context(|| {
            format!(
                "failed to backfill downloaded flag for {}:{}",
                row.base_model, row.model_scale
            )
        })?;
    }

    ensure_seed_directories_exist(&src_model_root)?;

    Ok(())
}

fn ensure_seed_directories_exist(src_model_root: &std::path::Path) -> Result<()> {
    let path = src_model_root.join("base-models");
    if !path.exists() {
        std::fs::create_dir_all(&path).with_context(|| {
            format!(
                "failed to create model artifacts directory: {}",
                path.display()
            )
        })?;
    }

    Ok(())
}
