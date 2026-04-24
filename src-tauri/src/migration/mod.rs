use anyhow::Context;
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::Result;

mod add_model_info_downloaded_flag;
mod add_vox_cpm2_lora_feature_flag;
mod create_local_schema;
mod seed_moss_tts_local_model_info;
mod seed_qwen3_tts_preset_speakers;
mod seed_vox_cpm2_model_info;

const LOCAL_SCHEMA_VERSION: &str = "18";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_local_schema::Migration),
            Box::new(seed_qwen3_tts_preset_speakers::Migration),
            Box::new(seed_vox_cpm2_model_info::Migration),
            Box::new(add_vox_cpm2_lora_feature_flag::Migration),
            Box::new(add_model_info_downloaded_flag::Migration),
            Box::new(seed_moss_tts_local_model_info::Migration),
        ]
    }
}

pub(crate) async fn run_local_migrations(db: &DatabaseConnection) -> Result<()> {
    Migrator::up(db, None)
        .await
        .context("failed to run SeaORM local migrations")?;
    db.execute(Statement::from_string(
        DbBackend::Sqlite,
        format!(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('local_schema_version', '{LOCAL_SCHEMA_VERSION}')"
        ),
    ))
    .await
    .context("failed to persist local schema version after migrations")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test_support::LocalServiceHarness;

    #[tokio::test]
    async fn fresh_database_matches_current_runtime_schema() {
        let harness = LocalServiceHarness::new("migration-fresh-current")
            .await
            .expect("create harness");

        assert!(harness
            .table_has_column("model_info", "model_scale")
            .await
            .expect("check model_scale column"));
        assert!(harness
            .table_has_column("tts_tasks", "model_params_json")
            .await
            .expect("check tts model_params_json column"));
        assert!(harness
            .table_has_column("model_info", "downloaded")
            .await
            .expect("check model_info downloaded column"));

        let model_infos = harness.list_model_infos().await.expect("list model infos");
        let scales = model_infos
            .iter()
            .map(|item| format!("{}:{}", item.base_model, item.model_scale))
            .collect::<Vec<_>>();
        let vox_info = model_infos
            .iter()
            .find(|item| item.base_model == "vox_cpm2" && item.model_scale == "2B")
            .expect("vox model info should exist");

        assert!(scales.contains(&"qwen3_tts:1.7B".to_string()));
        assert!(scales.contains(&"qwen3_tts:0.6B".to_string()));
        assert!(scales.contains(&"vox_cpm2:2B".to_string()));
        assert!(scales.contains(&"moss_tts_local:1.7B".to_string()));
        let moss_info = model_infos
            .iter()
            .find(|item| item.base_model == "moss_tts_local" && item.model_scale == "1.7B")
            .expect("moss model info should exist");
        assert!(!moss_info.downloaded);
        assert!(moss_info
            .supported_feature_list
            .iter()
            .any(|feature| feature == "text-to-speech"));
        assert!(moss_info
            .supported_feature_list
            .iter()
            .any(|feature| feature == "voice-clone"));
        assert!(moss_info
            .supported_feature_list
            .iter()
            .any(|feature| feature == "model-training"));
        assert!(vox_info
            .supported_feature_list
            .iter()
            .any(|feature| feature == "lora"));

        let speakers = harness.list_speakers().await.expect("list speakers");
        let speaker_names = speakers
            .iter()
            .map(|speaker| speaker.name.as_str())
            .collect::<Vec<_>>();
        assert!(speaker_names.contains(&"VoxCPM2_Speaker"));

        let model_path = harness
            .speaker_model_path_by_name("VoxCPM2_Speaker")
            .await
            .expect("query vox preset speaker model path")
            .expect("vox preset speaker model path should exist");
        assert_eq!(model_path, "%SRC_MODEL_ROOT_PATH%/base-models/VoxCPM2");

        harness.shutdown().await.expect("shutdown harness");
    }
}
