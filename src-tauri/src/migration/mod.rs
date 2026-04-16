use anyhow::Context;
use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

use crate::Result;

mod create_local_schema;
mod m20260415_000001_drop_task_hardware_type;
mod m20260416_000001_add_model_training_runtime_fields;
mod m20260416_000002_refactor_model_metadata;
mod seed_qwen3_tts_preset_speakers;

const LOCAL_SCHEMA_VERSION: &str = "13";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_local_schema::Migration),
            Box::new(seed_qwen3_tts_preset_speakers::Migration),
            Box::new(m20260415_000001_drop_task_hardware_type::Migration),
            Box::new(m20260416_000001_add_model_training_runtime_fields::Migration),
            Box::new(m20260416_000002_refactor_model_metadata::Migration),
        ]
    }
}

pub(crate) async fn run_local_migrations(db: &DatabaseConnection) -> Result<()> {
    Migrator::up(db, None)
        .await
        .context("failed to run SeaORM local migrations")?;
    Ok(())
}
