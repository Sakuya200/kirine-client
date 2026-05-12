use anyhow::{bail, Context};
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use tracing::error;

use crate::Result;

mod create_local_schema;
mod m20260508_000002_make_tts_speaker_nullable;
mod m20260508_000003_add_model_download_type;
mod m20260511_000004_remove_task_history_error_message;
mod m20260512_000005_rename_model_scale_to_model_version;

const LOCAL_SCHEMA_VERSION: &str = "22";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_local_schema::Migration),
            Box::new(m20260508_000002_make_tts_speaker_nullable::Migration),
            Box::new(m20260508_000003_add_model_download_type::Migration),
            Box::new(m20260511_000004_remove_task_history_error_message::Migration),
            Box::new(m20260512_000005_rename_model_scale_to_model_version::Migration),
        ]
    }
}

pub(crate) async fn run_local_migrations(db: &DatabaseConnection) -> Result<()> {
    Migrator::up(db, None)
        .await
        .context("failed to run SeaORM local migrations")?;
    let result = db.execute(Statement::from_string(
        DbBackend::Sqlite,
        format!(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('local_schema_version', '{LOCAL_SCHEMA_VERSION}')"
        ),
    ))
    .await;

    if let Err(err) = result {
        error!(error = %err, "failed to update local schema version in app_meta table");
        bail!("数据库版本更新失败，请尝试重启应用或联系开发者");
    }

    Ok(())
}

async fn column_exists(
    db: &impl ConnectionTrait,
    table_name: &str,
    column_name: &str,
) -> Result<bool, DbErr> {
    let rows = db
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info('{table_name}')"),
        ))
        .await?;

    for row in rows {
        let name: String = row.try_get("", "name")?;
        if name == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

async fn rename_column_if_needed(
    manager: &SchemaManager<'_>,
    table_name: &str,
    old_column: &str,
    new_column: &str,
) -> Result<(), DbErr> {
    let db = manager.get_connection();
    let has_old = column_exists(db, table_name, old_column).await?;
    let has_new = column_exists(db, table_name, new_column).await?;

    if !has_old || has_new {
        return Ok(());
    }

    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        format!("ALTER TABLE {table_name} RENAME COLUMN {old_column} TO {new_column}"),
    ))
    .await?;

    Ok(())
}
