use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260522_000007_add_task_device_and_model_supported_devices"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        if !column_exists(db, "task_history", "device").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE task_history ADD COLUMN device TEXT NOT NULL DEFAULT 'cpu'"
                    .to_string(),
            ))
            .await?;
        }

        if !column_exists(db, "model_info", "supported_devices").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE model_info ADD COLUMN supported_devices TEXT NOT NULL DEFAULT '[\"cpu\",\"cuda\"]'"
                    .to_string(),
            ))
            .await?;

            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "UPDATE model_info SET supported_devices='[\"cpu\"]' WHERE base_model='gpt_sovits_cpufast'"
                    .to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
