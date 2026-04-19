use sea_orm::{Statement, Value};
use sea_orm_migration::prelude::*;

use crate::{
    common::local_paths::src_model_relative_runtime_path,
    service::models::{SpeakerSource, SpeakerStatus},
};

const DEFAULT_TIMESTAMP: &str = "2026-04-17 00:00:00";
const VOX_CPM2_SPEAKER_NAME: &str = "VoxCPM2_Speaker";

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260419_000001_seed_vox_cpm2_model_info"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        connection
            .execute(Statement::from_string(
                backend,
                format!(
                    r#"INSERT OR REPLACE INTO model_info (id, base_model, model_name, model_scale, required_model_name_list_json, required_model_repo_id_list_json, supported_feature_list_json, create_time, modify_time, deleted) VALUES (3, 'vox_cpm2', 'VoxCPM2', '2B', '["VoxCPM2"]', '["openbmb/VoxCPM2"]', '["text-to-speech","voice-clone","model-training"]', '{DEFAULT_TIMESTAMP}', '{DEFAULT_TIMESTAMP}', 0)"#
                ),
            ))
            .await?;

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
                    Value::from(VOX_CPM2_SPEAKER_NAME),
                    Value::from(r#"["chinese","english","japanese"]"#),
                    Value::from(0_i64),
                    Value::from("vox_cpm2"),
                    Value::from("VoxCPM2 内置说话人，可直接用于语音合成或作为微调起点。"),
                    Value::from(Some(src_model_relative_runtime_path("base-models/VoxCPM2"))),
                    Value::from(SpeakerStatus::Ready.as_str()),
                    Value::from(SpeakerSource::Local.as_str()),
                    Value::from(DEFAULT_TIMESTAMP),
                    Value::from(DEFAULT_TIMESTAMP),
                    Value::from(0_i32),
                    Value::from(VOX_CPM2_SPEAKER_NAME),
                    Value::from("vox_cpm2"),
                ],
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let backend = manager.get_database_backend();
        let connection = manager.get_connection();

        connection
            .execute(Statement::from_sql_and_values(
                backend,
                "DELETE FROM speakers WHERE name = ? AND base_model = ? AND deleted = 0",
                vec![Value::from(VOX_CPM2_SPEAKER_NAME), Value::from("vox_cpm2")],
            ))
            .await?;
        connection
            .execute(Statement::from_string(
                backend,
                "DELETE FROM model_info WHERE id = 3 OR base_model = 'vox_cpm2'".to_string(),
            ))
            .await?;
        Ok(())
    }
}
