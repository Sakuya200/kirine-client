use sea_orm::{Statement, Value};
use sea_orm_migration::prelude::*;

const DEFAULT_TIMESTAMP: &str = "2026-04-24 00:00:00";
const MOSS_TTS_LOCAL_MODEL_INFO_ID: i64 = 4;
const MOSS_TTS_LOCAL_BASE_MODEL: &str = "moss_tts_local";
const MOSS_TTS_LOCAL_DISPLAY_NAME: &str = "MOSS-TTS Local";
const MOSS_TTS_LOCAL_MODEL_SCALE: &str = "1.7B";

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260424_000001_seed_moss_tts_local_model_info"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        connection
            .execute(Statement::from_sql_and_values(
                backend,
                r#"
                INSERT OR REPLACE INTO model_info (
                    id, base_model, model_name, model_scale,
                    required_model_name_list_json, required_model_repo_id_list_json,
                    supported_feature_list_json, create_time, modify_time, downloaded, deleted
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                vec![
                    Value::from(MOSS_TTS_LOCAL_MODEL_INFO_ID),
                    Value::from(MOSS_TTS_LOCAL_BASE_MODEL),
                    Value::from(MOSS_TTS_LOCAL_DISPLAY_NAME),
                    Value::from(MOSS_TTS_LOCAL_MODEL_SCALE),
                    Value::from(r#"["MOSS-TTS-Local-Transformer","MOSS-Audio-Tokenizer"]"#),
                    Value::from(r#"["OpenMOSS-Team/MOSS-TTS-Local-Transformer","OpenMOSS-Team/MOSS-Audio-Tokenizer"]"#),
                    Value::from(r#"["text-to-speech","voice-clone","model-training"]"#),
                    Value::from(DEFAULT_TIMESTAMP),
                    Value::from(DEFAULT_TIMESTAMP),
                    Value::from(0_i32),
                    Value::from(0_i32),
                ],
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        connection
            .execute(Statement::from_string(
                backend,
                format!(
                    "DELETE FROM model_info WHERE id = {MOSS_TTS_LOCAL_MODEL_INFO_ID} OR base_model = '{MOSS_TTS_LOCAL_BASE_MODEL}'"
                ),
            ))
            .await?;

        Ok(())
    }
}
