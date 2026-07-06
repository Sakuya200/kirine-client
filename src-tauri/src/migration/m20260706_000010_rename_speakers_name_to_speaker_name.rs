use crate::migration::rename_column_if_needed;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260706_000010_rename_speakers_name_to_speaker_name"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(manager, "speakers", "name", "speaker_name").await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(manager, "speakers", "speaker_name", "name").await
    }
}
