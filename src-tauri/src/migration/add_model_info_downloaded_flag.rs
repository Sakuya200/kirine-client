use anyhow::Context;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::{
    config::application_dir,
    service::{
        pipeline::{
            qwen3_tts::qwen3_tts_prepared_model_download_paths,
            script_paths::resolve_src_model_root,
            vox_cpm2::vox_cpm2_prepared_model_download_paths,
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
    let has_column = manager
        .has_column("model_info", "downloaded")
        .await?;
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

    for (base_model, model_scale, downloaded) in [
        (
            "qwen3_tts",
            "1.7B",
            all_paths_exist(&qwen3_tts_prepared_model_download_paths(
                &src_model_root,
                "1.7B",
            )?),
        ),
        (
            "qwen3_tts",
            "0.6B",
            all_paths_exist(&qwen3_tts_prepared_model_download_paths(
                &src_model_root,
                "0.6B",
            )?),
        ),
        (
            "vox_cpm2",
            "2B",
            all_paths_exist(&vox_cpm2_prepared_model_download_paths(&src_model_root, "2B")?),
        ),
    ] {
        db.execute(Statement::from_sql_and_values(
            backend,
            "UPDATE model_info SET downloaded = ? WHERE base_model = ? AND model_scale = ? AND deleted = 0",
            vec![downloaded.into(), base_model.into(), model_scale.into()],
        ))
        .await
        .with_context(|| format!("failed to backfill downloaded flag for {base_model}:{model_scale}"))?;
    }

    ensure_seed_directories_exist(&src_model_root)?;

    Ok(())
}

fn all_paths_exist(paths: &[std::path::PathBuf]) -> bool {
    paths.iter().all(|path| path.exists())
}

fn ensure_seed_directories_exist(src_model_root: &std::path::Path) -> Result<()> {
    let path = src_model_root.join("base-models");
    if !path.exists() {
        std::fs::create_dir_all(&path)
            .with_context(|| format!("failed to create model artifacts directory: {}", path.display()))?;
    }

    Ok(())
}