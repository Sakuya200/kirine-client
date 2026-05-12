use crate::migration::rename_column_if_needed;
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
        rename_column_if_needed(
            manager,
            "model_training_tasks",
            "model_scale",
            "model_version",
        )
        .await?;
        rename_column_if_needed(manager, "voice_clone_tasks", "model_scale", "model_version")
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        rename_column_if_needed(manager, "model_info", "model_version", "model_scale").await?;
        rename_column_if_needed(manager, "tts_tasks", "model_version", "model_scale").await?;
        rename_column_if_needed(
            manager,
            "model_training_tasks",
            "model_version",
            "model_scale",
        )
        .await?;
        rename_column_if_needed(manager, "voice_clone_tasks", "model_version", "model_scale")
            .await?;
        Ok(())
    }
}
