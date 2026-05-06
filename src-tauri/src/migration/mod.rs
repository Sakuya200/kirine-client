use anyhow::{bail, Context};
use sea_orm::DatabaseConnection;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use tracing::error;

use crate::Result;

mod create_local_schema;

const LOCAL_SCHEMA_VERSION: &str = "19";

pub(crate) struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(create_local_schema::Migration)]
    }
}

pub(crate) async fn run_local_migrations(db: &DatabaseConnection) -> Result<()> {
    Migrator::up(db, None)
        .await
        .context("failed to run SeaORM local migrations")?;
    let result = db.execute(Statement::from_string(
        DbBackend::Sqlite,
        format!(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('local_schema_version', '{LOCAL_SCHEMA_VERSION}')"
        ),
    ))
    .await;

    if let Err(err) = result {
        error!(error = %err, "failed to update local schema version in app_meta table");
        bail!("数据库版本更新失败，请尝试重启应用或联系开发者");
    }

    Ok(())
}
