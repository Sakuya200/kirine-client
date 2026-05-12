use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260512_000005_rename_model_scale_to_model_version"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(manager, "model_info", "model_scale", "model_version").await?;
        rename_column_if_needed(manager, "tts_tasks", "model_scale", "model_version").await?;
        rename_column_if_needed(manager, "model_training_tasks", "model_scale", "model_version").await?;
        rename_column_if_needed(manager, "voice_clone_tasks", "model_scale", "model_version").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(manager, "model_info", "model_version", "model_scale").await?;
        rename_column_if_needed(manager, "tts_tasks", "model_version", "model_scale").await?;
        rename_column_if_needed(manager, "model_training_tasks", "model_version", "model_scale").await?;
        rename_column_if_needed(manager, "voice_clone_tasks", "model_version", "model_scale").await?;
        Ok(())
    }
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
        format!(
            "ALTER TABLE {table_name} RENAME COLUMN {old_column} TO {new_column}"
        ),
    ))
    .await?;

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
