use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260626_000009_add_model_supported_languages_and_drop_speaker_languages"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // model_info.supported_languages — 模型支持的语言列表 (JSON 数组)
        if !column_exists(db, "model_info", "supported_languages").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE model_info ADD COLUMN supported_languages TEXT NOT NULL DEFAULT '[\"chinese\",\"english\",\"japanese\"]'"
                    .to_string(),
            ))
            .await?;
        }

        // speakers.languages_json — 该字段无实际作用，移除
        if column_exists(db, "speakers", "languages_json").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE speakers DROP COLUMN languages_json".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
