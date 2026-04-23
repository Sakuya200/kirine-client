use sea_orm::{ConnectionTrait, Statement, Value};
use sea_orm_migration::prelude::*;

use crate::{
    common::local_paths::src_model_relative_runtime_path,
    service::{
        models::{SpeakerSource, SpeakerStatus},
        pipeline::qwen3_tts::{
            qwen3_tts_preset_speakers, QWEN3_TTS_DEFAULT_CUSTOM_VOICE_MODEL_NAME,
        },
    },
};

pub struct Migration;

const DEFAULT_TIMESTAMP: &str = "2026-04-13 00:00:00";

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260413_000002_seed_qwen3_tts_preset_speakers"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();
        let model_path = src_model_relative_runtime_path(&format!(
            "base-models/{QWEN3_TTS_DEFAULT_CUSTOM_VOICE_MODEL_NAME}"
        ));
        let create_time = DEFAULT_TIMESTAMP;

        for (id, model_scale, model_names, repo_ids) in [
            (
                1_i64,
                "1.7B",
                r#"["Qwen3-TTS-12Hz-1.7B-Base","Qwen3-TTS-Tokenizer-12Hz","Qwen3-TTS-12Hz-1.7B-CustomVoice"]"#,
                r#"["Qwen/Qwen3-TTS-12Hz-1.7B-Base","Qwen/Qwen3-TTS-Tokenizer-12Hz","Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"]"#,
            ),
            (
                2_i64,
                "0.6B",
                r#"["Qwen3-TTS-12Hz-0.6B-Base","Qwen3-TTS-Tokenizer-12Hz","Qwen3-TTS-12Hz-0.6B-CustomVoice"]"#,
                r#"["Qwen/Qwen3-TTS-12Hz-0.6B-Base","Qwen/Qwen3-TTS-Tokenizer-12Hz","Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice"]"#,
            ),
        ] {
            connection
                .execute(Statement::from_string(
                    backend,
                    format!(
                        r#"INSERT OR REPLACE INTO model_info (id, base_model, model_name, model_scale, required_model_name_list_json, required_model_repo_id_list_json, supported_feature_list_json, create_time, modify_time, downloaded, deleted) VALUES ({id}, 'qwen3_tts', 'Qwen3-TTS', '{model_scale}', '{model_names}', '{repo_ids}', '["text-to-speech","voice-clone","model-training"]', '{DEFAULT_TIMESTAMP}', '{DEFAULT_TIMESTAMP}', 1, 0)"#
                    ),
                ))
                .await?;
        }

        for speaker in qwen3_tts_preset_speakers() {
            let languages_json = serde_json::to_string(speaker.languages)
                .map_err(|err| DbErr::Custom(err.to_string()))?;

            connection
                .execute(Statement::from_sql_and_values(
                    backend,
                    r#"
                    INSERT INTO speakers (
                        name, languages_json, samples, base_model, description, model_path,
                        status, source, create_time, modify_time, deleted
                    )
                    SELECT ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
                    WHERE NOT EXISTS (
                        SELECT 1 FROM speakers
                        WHERE name = ? AND base_model = ? AND deleted = 0
                    )
                    "#,
                    vec![
                        Value::from(speaker.name),
                        Value::from(languages_json),
                        Value::from(0_i64),
                        Value::from("qwen3_tts"),
                        Value::from(speaker.description),
                        Value::from(Some(model_path.clone())),
                        Value::from(SpeakerStatus::Ready.as_str()),
                        Value::from(SpeakerSource::Local.as_str()),
                        Value::from(create_time),
                        Value::from(create_time),
                        Value::from(0_i32),
                        Value::from(speaker.name),
                        Value::from("qwen3_tts"),
                    ],
                ))
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        connection
            .execute(Statement::from_string(
                backend,
                "DELETE FROM model_info WHERE base_model = 'qwen3_tts'".to_string(),
            ))
            .await?;

        for speaker in qwen3_tts_preset_speakers() {
            connection
                .execute(Statement::from_sql_and_values(
                    backend,
                    "DELETE FROM speakers WHERE name = ? AND base_model = ? AND deleted = 0",
                    vec![Value::from(speaker.name), Value::from("qwen3_tts")],
                ))
                .await?;
        }

        Ok(())
    }
}
