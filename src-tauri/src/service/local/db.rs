use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use anyhow::Context;
use log::LevelFilter;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqliteSynchronous};

use crate::Result;

const SQLITE_MODE: &str = "rwc";

pub(crate) async fn connect_local_database(db_path: &Path) -> Result<DatabaseConnection> {
    let parent_dir = db_path.parent().with_context(|| {
        format!(
            "failed to resolve parent directory for sqlite database: {}",
            db_path.display()
        )
    })?;
    std::fs::create_dir_all(parent_dir).with_context(|| {
        format!(
            "failed to create sqlite data directory: {}",
            parent_dir.display()
        )
    })?;

    if !db_path.exists() {
        std::fs::File::create(db_path).with_context(|| {
            format!(
                "failed to create sqlite database file: {}",
                db_path.display()
            )
        })?;
    }

    let orm_db_path = db_path.to_path_buf();
    let mut sea_options = ConnectOptions::new(build_sqlite_url(db_path)?);
    sea_options
        .max_connections(1)
        .min_connections(1)
        .connect_timeout(Duration::from_secs(8))
        .acquire_timeout(Duration::from_secs(8))
        .idle_timeout(Duration::from_secs(30))
        .sqlx_logging(false)
        .sqlx_logging_level(LevelFilter::Warn)
        .map_sqlx_sqlite_opts(move |_| {
            build_sqlite_connect_options(&orm_db_path).expect("sqlite path already validated")
        });

    let orm = Database::connect(sea_options).await.with_context(|| {
        format!(
            "failed to connect sqlite database with sea-orm: {}",
            db_path.display()
        )
    })?;

    Ok(orm)
}

pub(crate) fn build_sqlite_url(db_path: &Path) -> Result<String> {
    let normalized = db_path
        .to_str()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "sqlite database path contains invalid UTF-8",
            )
        })?
        .replace('\\', "/");
    Ok(format!("sqlite://{}?mode={}", normalized, SQLITE_MODE))
}

fn build_sqlite_connect_options(db_path: &Path) -> Result<SqliteConnectOptions> {
    SqliteConnectOptions::from_str(&build_sqlite_url(db_path)?)
        .map(|options| {
            options
                .filename(PathBuf::from(db_path))
                .create_if_missing(true)
                .foreign_keys(false)
                .journal_mode(SqliteJournalMode::Wal)
                .synchronous(SqliteSynchronous::Normal)
                .busy_timeout(Duration::from_secs(8))
        })
        .with_context(|| {
            format!(
                "failed to parse sqlite connection options: {}",
                db_path.display()
            )
        })
}
