use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260604_000008_add_voice_design_tasks"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(VoiceDesignTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(VoiceDesignTasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::HistoryId)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::BaseModel)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::ModelVersion)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::Language)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::Format)
                            .string()
                            .not_null()
                            .default("wav"),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::ExportAudioName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VoiceDesignTasks::Prompt).text().not_null())
                    .col(ColumnDef::new(VoiceDesignTasks::Text).text().not_null())
                    .col(
                        ColumnDef::new(VoiceDesignTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::CharCount)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::FileName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(VoiceDesignTasks::OutputFilePath).text())
                    .col(
                        ColumnDef::new(VoiceDesignTasks::CreateTime)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::ModifyTime)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(VoiceDesignTasks::Deleted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_voice_design_tasks_history")
                            .from(VoiceDesignTasks::Table, VoiceDesignTasks::HistoryId)
                            .to(TaskHistory::Table, TaskHistory::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_voice_design_tasks_history_id")
                    .table(VoiceDesignTasks::Table)
                    .col(VoiceDesignTasks::HistoryId)
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
                    .name("idx_voice_design_tasks_history_id")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(VoiceDesignTasks::Table).to_owned())
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
enum VoiceDesignTasks {
    Table,
    Id,
    HistoryId,
    BaseModel,
    ModelVersion,
    Language,
    Format,
    ExportAudioName,
    Prompt,
    Text,
    ModelParamsJson,
    CharCount,
    FileName,
    OutputFilePath,
    CreateTime,
    ModifyTime,
    Deleted,
}
