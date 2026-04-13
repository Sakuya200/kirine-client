use std::{fs, path::PathBuf};

use rand::random;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema};
use sqlx::{sqlite::SqlitePoolOptions, Row};

use crate::{
    config::BaseModel,
    service::entity::{speaker, task_history, training_task, tts_task, voice_clone_task},
    service::{
        models::{
            AppLanguage, CreateSpeakerPayload, SpeakerInfo, SpeakerSource, SpeakerStatus,
            UpdateSpeakerPayload,
        },
        LocalService, Service,
    },
    Result,
};

pub struct LocalServiceHarness {
    root_dir: PathBuf,
    data_dir: PathBuf,
    service: LocalService,
}

impl LocalServiceHarness {
    pub async fn new(label: &str) -> Result<Self> {
        let root_dir = test_root(label);
        let data_dir = root_dir.join("data");
        let model_dir = root_dir.join("models");
        let service =
            LocalService::from_paths(root_dir.clone(), data_dir.clone(), model_dir).await?;

        Ok(Self {
            root_dir,
            data_dir,
            service,
        })
    }

    pub fn database_file_exists(&self) -> bool {
        self.data_dir.join("app.db").exists()
    }

    pub async fn speakers_query_succeeds(&self) -> Result<bool> {
        self.service.list_speaker_infos().await?;
        Ok(true)
    }

    pub async fn create_test_speaker(&self) -> Result<SpeakerInfo> {
        self.service
            .create_speaker_info(CreateSpeakerPayload {
                name: "SeaOrm Speaker".to_string(),
                languages: vec![AppLanguage::Chinese, AppLanguage::English],
                samples: 3,
                base_model: BaseModel::Qwen3Tts,
                description: "created by test".to_string(),
                status: SpeakerStatus::Ready,
                source: SpeakerSource::Local,
            })
            .await
    }

    pub async fn list_speakers(&self) -> Result<Vec<SpeakerInfo>> {
        self.service.list_speaker_infos().await
    }

    pub async fn speaker_model_path_by_name(&self, name: &str) -> Result<Option<String>> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let row = sqlx::query(
            "SELECT model_path FROM speakers WHERE name = ? AND deleted = 0 ORDER BY id ASC LIMIT 1",
        )
        .bind(name)
        .fetch_optional(&pool)
        .await?;
        pool.close().await;

        Ok(row.and_then(|row| row.get::<Option<String>, _>("model_path")))
    }

    pub async fn update_test_speaker(&self, id: i64) -> Result<SpeakerInfo> {
        self.service
            .update_speaker_info(UpdateSpeakerPayload {
                id,
                name: "Updated Speaker".to_string(),
                description: "updated by test".to_string(),
            })
            .await
    }

    pub async fn delete_speaker(&self, id: i64) -> Result<bool> {
        self.service.delete_speaker_info(id).await
    }

    pub async fn shutdown(self) -> Result<()> {
        self.service.close().await?;
        fs::remove_dir_all(self.root_dir)?;
        Ok(())
    }

    pub async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let row = sqlx::query(
            "SELECT COUNT(1) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table_name)
        .fetch_one(&pool)
        .await?;
        pool.close().await;

        Ok(row.get::<i64, _>("count") > 0)
    }

    pub async fn table_has_column(&self, table_name: &str, column_name: &str) -> Result<bool> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let pragma = format!("PRAGMA table_info({table_name})");
        let rows = sqlx::query(&pragma).fetch_all(&pool).await?;
        pool.close().await;

        Ok(rows
            .iter()
            .any(|row| row.get::<String, _>("name") == column_name))
    }

    pub async fn task_detail_id_for_history(
        &self,
        table_name: &str,
        history_id: i64,
    ) -> Result<Option<i64>> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let sql = format!("SELECT id FROM {table_name} WHERE history_id = ?");
        let row = sqlx::query(&sql)
            .bind(history_id)
            .fetch_optional(&pool)
            .await?;
        pool.close().await;

        Ok(row.map(|row| row.get::<i64, _>("id")))
    }

    pub async fn new_with_legacy_schema(label: &str) -> Result<Self> {
        let root_dir = test_root(label);
        let data_dir = root_dir.join("data");
        let model_dir = root_dir.join("models");

        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&model_dir)?;
        seed_legacy_schema(&data_dir.join("app.db")).await?;

        let service =
            LocalService::from_paths(root_dir.clone(), data_dir.clone(), model_dir).await?;

        Ok(Self {
            root_dir,
            data_dir,
            service,
        })
    }

    pub async fn new_with_legacy_task_detail_schema(label: &str) -> Result<Self> {
        let root_dir = test_root(label);
        let data_dir = root_dir.join("data");
        let model_dir = root_dir.join("models");

        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&model_dir)?;
        seed_legacy_task_detail_schema(&data_dir.join("app.db")).await?;

        let service =
            LocalService::from_paths(root_dir.clone(), data_dir.clone(), model_dir).await?;

        Ok(Self {
            root_dir,
            data_dir,
            service,
        })
    }
}

fn test_root(label: &str) -> PathBuf {
    let unique = random::<u64>();
    std::env::temp_dir().join(format!("kirine-client-{label}-{unique}"))
}

struct SqliteTableColumn {
    name: String,
    data_type: String,
    not_null: bool,
    default_value: Option<String>,
}

fn sqlite_database_url(db_path: &PathBuf) -> String {
    let normalized = db_path.to_string_lossy().replace('\\', "/");
    format!("sqlite://{}?mode=rwc", normalized)
}

async fn open_database_connection(db_path: &PathBuf) -> Result<DatabaseConnection> {
    Ok(Database::connect(&sqlite_database_url(db_path)).await?)
}

async fn create_table_from_entity<E>(db: &DatabaseConnection, entity: E) -> Result<()>
where
    E: EntityTrait,
{
    let backend = DbBackend::Sqlite;
    let schema = Schema::new(backend);
    let statement = backend.build(&schema.create_table_from_entity(entity));
    db.execute(statement).await?;
    Ok(())
}

async fn create_current_speakers_table(db_path: &PathBuf) -> Result<()> {
    let db = open_database_connection(db_path).await?;
    create_table_from_entity(&db, speaker::Entity).await
}

async fn create_current_task_history_table(db_path: &PathBuf) -> Result<()> {
    let db = open_database_connection(db_path).await?;
    create_table_from_entity(&db, task_history::Entity).await
}

#[allow(dead_code)]
async fn create_current_task_detail_tables(db_path: &PathBuf) -> Result<()> {
    let db = open_database_connection(db_path).await?;
    create_table_from_entity(&db, tts_task::Entity).await?;
    create_table_from_entity(&db, training_task::Entity).await?;
    create_table_from_entity(&db, voice_clone_task::Entity).await
}

async fn load_sqlite_table_columns(
    pool: &sqlx::SqlitePool,
    table_name: &str,
) -> Result<Vec<SqliteTableColumn>> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| SqliteTableColumn {
            name: row.get::<String, _>("name"),
            data_type: row.get::<String, _>("type"),
            not_null: row.get::<i64, _>("notnull") != 0,
            default_value: row.get::<Option<String>, _>("dflt_value"),
        })
        .collect())
}

fn build_legacy_task_table_sql(table_name: &str, columns: &[SqliteTableColumn]) -> String {
    let column_defs = columns
        .iter()
        .filter(|column| column.name != "id")
        .map(|column| {
            if column.name == "history_id" {
                return format!("{} {} NOT NULL PRIMARY KEY", column.name, column.data_type);
            }

            let mut column_def = format!("{} {}", column.name, column.data_type);
            if column.not_null {
                column_def.push_str(" NOT NULL");
            }
            if let Some(default_value) = column.default_value.as_deref() {
                column_def.push_str(" DEFAULT ");
                column_def.push_str(default_value);
            }
            column_def
        })
        .collect::<Vec<_>>();

    format!(
        "CREATE TABLE {} (\n            {}\n        )",
        table_name,
        column_defs.join(",\n            ")
    )
}

async fn create_legacy_task_detail_tables_from_entities(db_path: &PathBuf) -> Result<()> {
    create_current_task_detail_tables(db_path).await?;

    let pool = open_sqlite_pool(db_path).await?;
    for table_name in ["tts_tasks", "model_training_tasks", "voice_clone_tasks"] {
        let columns = load_sqlite_table_columns(&pool, table_name).await?;
        let legacy_sql = build_legacy_task_table_sql(table_name, &columns);

        sqlx::query(&format!("DROP TABLE {table_name}"))
            .execute(&pool)
            .await?;
        sqlx::query(&legacy_sql).execute(&pool).await?;
    }
    pool.close().await;

    Ok(())
}

async fn seed_legacy_schema(db_path: &PathBuf) -> Result<()> {
    if !db_path.exists() {
        fs::File::create(db_path)?;
    }

    create_current_speakers_table(db_path).await?;

    let pool = open_sqlite_pool(db_path).await?;

    sqlx::query(
        r#"
        INSERT INTO speakers (
            id, name, languages_json, samples, base_model, description, model_path,
            status, source, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(1_i64)
    .bind("Legacy Speaker")
    .bind(r#"["chinese"]"#)
    .bind(2_i64)
    .bind("qwen3_tts")
    .bind("")
    .bind(Option::<String>::None)
    .bind("ready")
    .bind("local")
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE model_training_samples (
            id INTEGER PRIMARY KEY,
            file_path TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}

async fn seed_legacy_task_detail_schema(db_path: &PathBuf) -> Result<()> {
    if !db_path.exists() {
        fs::File::create(db_path)?;
    }

    create_current_speakers_table(db_path).await?;
    create_current_task_history_table(db_path).await?;
    create_legacy_task_detail_tables_from_entities(db_path).await?;

    let pool = open_sqlite_pool(db_path).await?;

    sqlx::query(
        r#"
        CREATE TABLE app_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO speakers (
            id, name, languages_json, samples, base_model, description, model_path,
            status, source, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(1_i64)
    .bind("Legacy Speaker")
    .bind(r#"["chinese"]"#)
    .bind(2_i64)
    .bind("qwen3_tts")
    .bind("")
    .bind(Option::<String>::None)
    .bind("ready")
    .bind("local")
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    for (history_id, task_type, title) in [
        (101_i64, "text_to_speech", "legacy-tts"),
        (102_i64, "model_training", "legacy-training"),
        (103_i64, "voice_clone", "legacy-voice-clone"),
    ] {
        sqlx::query(
            r#"
            INSERT INTO task_history (
                id, task_type, title, speaker_id, speaker_name_snapshot, status,
                duration_seconds, create_time, modify_time, finished_time, error_message, deleted
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(history_id)
        .bind(task_type)
        .bind(title)
        .bind(Some(1_i64))
        .bind("Legacy Speaker")
        .bind("pending")
        .bind(0_i64)
        .bind("2026-04-01 10:00:00")
        .bind("2026-04-01 10:00:00")
        .bind(Option::<String>::None)
        .bind(Option::<String>::None)
        .bind(0_i64)
        .execute(&pool)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO tts_tasks (
            history_id, speaker_id, model_path, base_model, hardware_type, language, format,
            text, voice_prompt, char_count, file_name, output_file_path, create_time,
            modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(101_i64)
    .bind(1_i64)
    .bind(Option::<String>::None)
    .bind("qwen3_tts")
    .bind("cpu")
    .bind("chinese")
    .bind("wav")
    .bind("hello")
    .bind("")
    .bind(5_i64)
    .bind("legacy-tts.wav")
    .bind(Option::<String>::None)
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO model_training_tasks (
            history_id, language, base_model, hardware_type, model_name, epoch_count,
            batch_size, sample_count, samples_json, notes_json, output_speaker_id,
            create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(102_i64)
    .bind("chinese")
    .bind("qwen3_tts")
    .bind("cpu")
    .bind("legacy-model")
    .bind(1_i64)
    .bind(1_i64)
    .bind(1_i64)
    .bind("[]")
    .bind("[]")
    .bind(Some(1_i64))
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO voice_clone_tasks (
            history_id, base_model, hardware_type, language, format, ref_audio_name,
            ref_audio_path, ref_text, text, char_count, file_name, output_file_path,
            create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(103_i64)
    .bind("qwen3_tts")
    .bind("cpu")
    .bind("chinese")
    .bind("wav")
    .bind("ref.wav")
    .bind("/tmp/ref.wav")
    .bind("参考")
    .bind("生成")
    .bind(2_i64)
    .bind("legacy-voice-clone.wav")
    .bind(Option::<String>::None)
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    pool.close().await;
    Ok(())
}

async fn open_sqlite_pool(db_path: &PathBuf) -> Result<sqlx::SqlitePool> {
    let database_url = sqlite_database_url(db_path);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?)
}
