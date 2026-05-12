use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000003_add_model_download_type"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if column_exists(manager.get_connection(), "model_info", "download_type").await? {
            return Ok(());
        }

        manager
            .get_connection()
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE model_info ADD COLUMN download_type TEXT NOT NULL DEFAULT 'HF-Like'"
                    .to_string(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
