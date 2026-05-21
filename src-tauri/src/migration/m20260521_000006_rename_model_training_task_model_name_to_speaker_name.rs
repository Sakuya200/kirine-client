use crate::migration::rename_column_if_needed;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260521_000006_rename_model_training_task_model_name_to_speaker_name"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(
            manager,
            "model_training_tasks",
            "model_name",
            "speaker_name",
        )
        .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(
            manager,
            "model_training_tasks",
            "speaker_name",
            "model_name",
        )
        .await
    }
}
