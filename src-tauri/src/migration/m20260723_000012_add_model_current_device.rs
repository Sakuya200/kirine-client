use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260723_000012_add_model_current_device"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 新增可空列 current_device：存储用户在模型管理页选择的当前设备。
        // 无默认值（对应「不设默认，要求先选」）；单设备模型的回填由 sync_supported_models
        // 的 upsert 在每次启动时完成，故迁移本身只负责加列。
        if !column_exists(db, "model_info", "current_device").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE model_info ADD COLUMN current_device TEXT".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
