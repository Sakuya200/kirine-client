use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260511_000004_remove_task_history_error_message"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TaskHistory::Table)
                    .drop_column(TaskHistory::ErrorMessage)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TaskHistory::Table)
                    .add_column(ColumnDef::new(TaskHistory::ErrorMessage).text())
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum TaskHistory {
    Table,
    ErrorMessage,
}
