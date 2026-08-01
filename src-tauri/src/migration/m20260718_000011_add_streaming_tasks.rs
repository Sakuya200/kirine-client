use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260718_000011_add_streaming_tasks"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StreamingTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StreamingTasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::HistoryId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::BaseModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::ModelVersion)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(StreamingTasks::Language).string().not_null())
                    .col(
                        ColumnDef::new(StreamingTasks::Device)
                            .string()
                            .not_null()
                            .default("cpu"),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::ContextFilePath)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::InputCacheFilePath)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::OutputAudioDir)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::MessageCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::CreateTime)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::ModifyTime)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::Deleted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_streaming_tasks_history")
                            .from(StreamingTasks::Table, StreamingTasks::HistoryId)
                            .to(TaskHistory::Table, TaskHistory::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_streaming_tasks_history_id")
                    .table(StreamingTasks::Table)
                    .col(StreamingTasks::HistoryId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_streaming_tasks_history_id")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(StreamingTasks::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum TaskHistory {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum StreamingTasks {
    Table,
    Id,
    HistoryId,
    BaseModel,
    ModelVersion,
    Language,
    Device,
    ModelParamsJson,
    ContextFilePath,
    InputCacheFilePath,
    OutputAudioDir,
    MessageCount,
    CreateTime,
    ModifyTime,
    Deleted,
}
