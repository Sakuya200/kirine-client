use anyhow::Context;
use sea_orm::DatabaseConnection;
use sea_orm_migration::prelude::*;

use crate::Result;

mod create_local_schema;
mod seed_qwen3_tts_preset_speakers;

const LOCAL_SCHEMA_VERSION: &str = "10";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(create_local_schema::Migration),
            Box::new(seed_qwen3_tts_preset_speakers::Migration),
        ]
    }
}

pub(crate) async fn run_local_migrations(db: &DatabaseConnection) -> Result<()> {
    Migrator::up(db, None)
        .await
        .context("failed to run SeaORM local migrations")?;
    Ok(())
}
