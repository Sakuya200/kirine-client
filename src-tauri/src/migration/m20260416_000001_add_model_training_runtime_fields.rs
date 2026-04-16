use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("model_training_tasks", "gradient_accumulation_steps")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ModelTrainingTasks::Table)
                        .add_column(
                            ColumnDef::new(ModelTrainingTasks::GradientAccumulationSteps)
                                .integer()
                                .not_null()
                                .default(4),
                        )
                        .to_owned(),
                )
                .await?;
        }

        if !manager
            .has_column("model_training_tasks", "enable_gradient_checkpointing")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ModelTrainingTasks::Table)
                        .add_column(
                            ColumnDef::new(ModelTrainingTasks::EnableGradientCheckpointing)
                                .boolean()
                                .not_null()
                                .default(false),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager
            .has_column("model_training_tasks", "enable_gradient_checkpointing")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ModelTrainingTasks::Table)
                        .drop_column(ModelTrainingTasks::EnableGradientCheckpointing)
                        .to_owned(),
                )
                .await?;
        }

        if manager
            .has_column("model_training_tasks", "gradient_accumulation_steps")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ModelTrainingTasks::Table)
                        .drop_column(ModelTrainingTasks::GradientAccumulationSteps)
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ModelTrainingTasks {
    Table,
    GradientAccumulationSteps,
    EnableGradientCheckpointing,
}
