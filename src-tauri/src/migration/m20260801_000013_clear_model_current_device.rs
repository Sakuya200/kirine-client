use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260801_000013_clear_model_current_device"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        if column_exists(db, "model_info", "current_device").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "UPDATE model_info SET current_device = NULL WHERE current_device IS NOT NULL"
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
