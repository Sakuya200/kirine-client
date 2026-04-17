use sea_orm::{ConnectionTrait, Statement, Value};
use sea_orm_migration::prelude::*;

use crate::{
    common::local_paths::src_model_relative_runtime_path,
    migration::LOCAL_SCHEMA_VERSION,
    service::{
        models::{SpeakerSource, SpeakerStatus},
        pipeline::qwen3_tts::{
            qwen3_tts_preset_speakers, QWEN3_TTS_DEFAULT_CUSTOM_VOICE_MODEL_NAME,
        },
    },
};

pub struct Migration;

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
        let create_time = "2026-04-13 00:00:00";

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

        connection
            .execute(Statement::from_string(
                backend,
                format!(
                    "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('local_schema_version', '{LOCAL_SCHEMA_VERSION}')"
                ),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

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

#[cfg(test)]
mod tests {
    use crate::test_support::LocalServiceHarness;

    #[tokio::test]
    async fn seeds_qwen3_tts_custom_voice_preset_speakers() {
        let harness = LocalServiceHarness::new("preset-speakers-migration")
            .await
            .expect("create local service harness");

        let speakers = harness.list_speakers().await.expect("list speakers");
        let speaker_names = speakers
            .iter()
            .map(|speaker| speaker.name.as_str())
            .collect::<Vec<_>>();

        assert!(speaker_names.contains(&"Vivian"));
        assert!(speaker_names.contains(&"Ryan"));
        assert!(speaker_names.contains(&"Ono_Anna"));
        assert!(speaker_names.contains(&"Sohee"));

        let model_path = harness
            .speaker_model_path_by_name("Vivian")
            .await
            .expect("query preset speaker model path")
            .expect("preset speaker model path should exist");

        assert_eq!(
            model_path,
            "%SRC_MODEL_ROOT_PATH%/base-models/Qwen3-TTS-12Hz-1.7B-CustomVoice"
        );

        harness.shutdown().await.expect("shutdown local service harness");
    }
}