use sea_orm_migration::{prelude::*, sea_orm::Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

const DEFAULT_MODEL_PARAMS_JSON: &str = "{}";
const DEFAULT_TIMESTAMP: &str = "2026-04-16 00:00:00";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        add_task_columns(manager).await?;
        create_model_info_table(manager).await?;
        seed_default_model_info(manager).await?;
        drop_legacy_columns(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

async fn add_task_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !manager.has_column("tts_tasks", "model_scale").await? {
        manager
            .alter_table(
                Table::alter()
                    .table(TtsTasks::Table)
                    .add_column(
                        ColumnDef::new(TtsTasks::ModelScale)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
    }
    if !manager.has_column("tts_tasks", "export_audio_name").await? {
        manager
            .alter_table(
                Table::alter()
                    .table(TtsTasks::Table)
                    .add_column(
                        ColumnDef::new(TtsTasks::ExportAudioName)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
    }
    if !manager.has_column("tts_tasks", "model_params_json").await? {
        manager
            .alter_table(
                Table::alter()
                    .table(TtsTasks::Table)
                    .add_column(
                        ColumnDef::new(TtsTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default(DEFAULT_MODEL_PARAMS_JSON),
                    )
                    .to_owned(),
            )
            .await?;
    }

    if !manager
        .has_column("model_training_tasks", "model_scale")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(ModelTrainingTasks::Table)
                    .add_column(
                        ColumnDef::new(ModelTrainingTasks::ModelScale)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
    }
    if !manager
        .has_column("model_training_tasks", "model_params_json")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(ModelTrainingTasks::Table)
                    .add_column(
                        ColumnDef::new(ModelTrainingTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default(DEFAULT_MODEL_PARAMS_JSON),
                    )
                    .to_owned(),
            )
            .await?;
    }

    if !manager
        .has_column("voice_clone_tasks", "model_scale")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(VoiceCloneTasks::Table)
                    .add_column(
                        ColumnDef::new(VoiceCloneTasks::ModelScale)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
    }
    if !manager
        .has_column("voice_clone_tasks", "export_audio_name")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(VoiceCloneTasks::Table)
                    .add_column(
                        ColumnDef::new(VoiceCloneTasks::ExportAudioName)
                            .string()
                            .not_null()
                            .default(""),
                    )
                    .to_owned(),
            )
            .await?;
    }
    if !manager
        .has_column("voice_clone_tasks", "model_params_json")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(VoiceCloneTasks::Table)
                    .add_column(
                        ColumnDef::new(VoiceCloneTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default(DEFAULT_MODEL_PARAMS_JSON),
                    )
                    .to_owned(),
            )
            .await?;
    }

    Ok(())
}

async fn create_model_info_table(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if !manager.has_table("model_info").await? {
        manager
            .create_table(
                Table::create()
                    .table(ModelInfo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ModelInfo::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ModelInfo::BaseModel).string().not_null())
                    .col(ColumnDef::new(ModelInfo::ModelName).string().not_null())
                    .col(ColumnDef::new(ModelInfo::ModelScale).string().not_null())
                    .col(
                        ColumnDef::new(ModelInfo::RequiredModelNameListJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ModelInfo::RequiredModelRepoIdListJson)
                            .text()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ModelInfo::SupportedFeatureListJson)
                            .text()
                            .not_null(),
                    )
                    .col(ColumnDef::new(ModelInfo::CreateTime).string().not_null())
                    .col(ColumnDef::new(ModelInfo::ModifyTime).string().not_null())
                    .col(
                        ColumnDef::new(ModelInfo::Deleted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .to_owned(),
            )
            .await?;
    }

    manager
        .create_index(
            Index::create()
                .name("idx_model_info_base_model")
                .table(ModelInfo::Table)
                .col(ModelInfo::BaseModel)
                .col(ModelInfo::ModelScale)
                .unique()
                .if_not_exists()
                .to_owned(),
        )
        .await?;

    Ok(())
}

async fn seed_default_model_info(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(
            manager.get_database_backend(),
            format!(
                r#"INSERT OR REPLACE INTO model_info (id, base_model, model_name, model_scale, required_model_name_list_json, required_model_repo_id_list_json, supported_feature_list_json, create_time, modify_time, deleted) VALUES (1, 'qwen3_tts', 'Qwen3-TTS', '1.7B', '["Qwen3-TTS-12Hz-1.7B-Base","Qwen3-TTS-Tokenizer-12Hz","Qwen3-TTS-12Hz-1.7B-CustomVoice"]', '["Qwen/Qwen3-TTS-12Hz-1.7B-Base","Qwen/Qwen3-TTS-Tokenizer-12Hz","Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"]', '["text-to-speech","voice-clone","model-training"]', '{DEFAULT_TIMESTAMP}', '{DEFAULT_TIMESTAMP}', 0)"#
            ),
        ))
        .await?;

    Ok(())
}

async fn drop_legacy_columns(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.has_column("tts_tasks", "voice_prompt").await? {
        manager
            .alter_table(
                Table::alter()
                    .table(TtsTasks::Table)
                    .drop_column(TtsTasks::VoicePrompt)
                    .to_owned(),
            )
            .await?;
    }

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
    if manager
        .has_column("model_training_tasks", "batch_size")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(ModelTrainingTasks::Table)
                    .drop_column(ModelTrainingTasks::BatchSize)
                    .to_owned(),
            )
            .await?;
    }
    if manager
        .has_column("model_training_tasks", "epoch_count")
        .await?
    {
        manager
            .alter_table(
                Table::alter()
                    .table(ModelTrainingTasks::Table)
                    .drop_column(ModelTrainingTasks::EpochCount)
                    .to_owned(),
            )
            .await?;
    }

    Ok(())
}

#[derive(DeriveIden)]
enum TtsTasks {
    Table,
    ModelScale,
    ExportAudioName,
    ModelParamsJson,
    VoicePrompt,
}

#[derive(DeriveIden)]
enum ModelTrainingTasks {
    Table,
    ModelScale,
    ModelParamsJson,
    EpochCount,
    BatchSize,
    GradientAccumulationSteps,
    EnableGradientCheckpointing,
}

#[derive(DeriveIden)]
enum VoiceCloneTasks {
    Table,
    ModelScale,
    ExportAudioName,
    ModelParamsJson,
}

#[derive(DeriveIden)]
enum ModelInfo {
    Table,
    Id,
    BaseModel,
    ModelName,
    ModelScale,
    RequiredModelNameListJson,
    RequiredModelRepoIdListJson,
    SupportedFeatureListJson,
    CreateTime,
    ModifyTime,
    Deleted,
}
