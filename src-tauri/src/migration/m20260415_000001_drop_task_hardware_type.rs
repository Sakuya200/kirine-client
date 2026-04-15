use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.has_column("tts_tasks", "hardware_type").await? {
            manager
                .alter_table(
                    Table::alter()
                        .table(TtsTasks::Table)
                        .drop_column(TtsTasks::HardwareType)
                        .to_owned(),
                )
                .await?;
        }

        if manager
            .has_column("model_training_tasks", "hardware_type")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ModelTrainingTasks::Table)
                        .drop_column(ModelTrainingTasks::HardwareType)
                        .to_owned(),
                )
                .await?;
        }

        if manager
            .has_column("voice_clone_tasks", "hardware_type")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(VoiceCloneTasks::Table)
                        .drop_column(VoiceCloneTasks::HardwareType)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TtsTasks::Table)
                    .add_column(
                        ColumnDef::new(TtsTasks::HardwareType)
                            .string()
                            .not_null()
                            .default("cuda"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(ModelTrainingTasks::Table)
                    .add_column(
                        ColumnDef::new(ModelTrainingTasks::HardwareType)
                            .string()
                            .not_null()
                            .default("cuda"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(VoiceCloneTasks::Table)
                    .add_column(
                        ColumnDef::new(VoiceCloneTasks::HardwareType)
                            .string()
                            .not_null()
                            .default("cuda"),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum TtsTasks {
    Table,
    HardwareType,
}

#[derive(DeriveIden)]
enum ModelTrainingTasks {
    Table,
    HardwareType,
}

#[derive(DeriveIden)]
enum VoiceCloneTasks {
    Table,
    HardwareType,
}