use anyhow::Context;
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

use crate::Result;

mod create_local_schema;
mod seed_qwen3_tts_preset_speakers;
mod seed_vox_cpm2_model_info;

const LOCAL_SCHEMA_VERSION: &str = "16";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_local_schema::Migration),
            Box::new(seed_qwen3_tts_preset_speakers::Migration),
            Box::new(seed_vox_cpm2_model_info::Migration),
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
