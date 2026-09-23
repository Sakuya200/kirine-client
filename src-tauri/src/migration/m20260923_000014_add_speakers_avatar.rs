use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::migration::column_exists;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260923_000014_add_speakers_avatar"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        // 新增可空列 avatar / avatar_content_type：说话人头像图像内容直接入库，
        // avatar_content_type 记录 MIME 类型（image/png 等）；两者均 NULL 表示未设置头像。
        if !column_exists(db, "speakers", "avatar").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE speakers ADD COLUMN avatar BLOB".to_string(),
            ))
            .await?;
        }
        if !column_exists(db, "speakers", "avatar_content_type").await? {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE speakers ADD COLUMN avatar_content_type TEXT".to_string(),
            ))
            .await?;
        }

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
