use std::{fs, path::PathBuf};

use rand::random;
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, EntityTrait, Schema};
use sqlx::{sqlite::SqlitePoolOptions, Row};

use crate::{
    service::entity::{speaker, task_history, training_task, tts_task, voice_clone_task},
    service::{
        models::{
            AppLanguage, CreateSpeakerPayload, HistoryRecord, ModelInfo, SpeakerInfo,
            SpeakerSource, SpeakerStatus, UpdateSpeakerPayload,
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
                base_model: "qwen3_tts".to_string(),
                description: "created by test".to_string(),
                status: SpeakerStatus::Ready,
                source: SpeakerSource::Local,
            })
            .await
    }

    pub async fn list_speakers(&self) -> Result<Vec<SpeakerInfo>> {
        self.service.list_speaker_infos().await
    }

    pub async fn list_model_infos(&self) -> Result<Vec<ModelInfo>> {
        self.service.list_model_infos().await
    }

    pub async fn list_history_records(&self) -> Result<Vec<HistoryRecord>> {
        self.service.list_history_records().await
    }

    pub async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        self.service.get_history_record(history_id).await
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

    pub async fn new_with_pre_refactor_schema(label: &str) -> Result<Self> {
        let root_dir = test_root(label);
        let data_dir = root_dir.join("data");
        let model_dir = root_dir.join("models");

        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&model_dir)?;
        seed_pre_refactor_schema(&data_dir.join("app.db")).await?;

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

async fn create_legacy_task_detail_tables_from_entities(db_path: &PathBuf) -> Result<()> {
    create_current_task_detail_tables(db_path).await
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
            history_id, speaker_id, model_path, base_model, model_scale, language, format,
            export_audio_name, text, model_params_json, char_count, file_name, output_file_path,
            create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(101_i64)
    .bind(1_i64)
    .bind(Option::<String>::None)
    .bind("qwen3_tts")
    .bind("1.7B")
    .bind("chinese")
    .bind("wav")
    .bind("legacy-tts")
    .bind("hello")
    .bind("{}")
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
            history_id, language, base_model, model_scale, model_name, model_params_json,
            sample_count, samples_json, notes_json, output_speaker_id, create_time,
            modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(102_i64)
    .bind("chinese")
    .bind("qwen3_tts")
    .bind("1.7B")
    .bind("legacy-model")
    .bind("{}")
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
            history_id, base_model, model_scale, language, format, export_audio_name,
            ref_audio_name, ref_audio_path, ref_text, text, model_params_json, char_count,
            file_name, output_file_path, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(103_i64)
    .bind("qwen3_tts")
    .bind("1.7B")
    .bind("chinese")
    .bind("wav")
    .bind("legacy-voice-clone")
    .bind("ref.wav")
    .bind("/tmp/ref.wav")
    .bind("参考")
    .bind("生成")
    .bind("{}")
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

async fn seed_pre_refactor_schema(db_path: &PathBuf) -> Result<()> {
    if !db_path.exists() {
        fs::File::create(db_path)?;
    }

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
        CREATE TABLE speakers (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            languages_json TEXT NOT NULL,
            samples INTEGER NOT NULL DEFAULT 0,
            base_model TEXT NOT NULL DEFAULT 'qwen3_tts',
            description TEXT NOT NULL DEFAULT '',
            model_path TEXT,
            status TEXT NOT NULL,
            source TEXT NOT NULL,
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE task_history (
            id INTEGER PRIMARY KEY,
            task_type TEXT NOT NULL,
            title TEXT NOT NULL,
            speaker_id INTEGER,
            speaker_name_snapshot TEXT NOT NULL,
            status TEXT NOT NULL,
            duration_seconds INTEGER NOT NULL DEFAULT 0,
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            finished_time TEXT,
            error_message TEXT,
            deleted INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE model_info (
            id INTEGER PRIMARY KEY,
            base_model TEXT NOT NULL,
            model_name TEXT NOT NULL,
            model_scale_list_json TEXT NOT NULL,
            required_model_name_list_json TEXT NOT NULL,
            required_model_repo_id_list_json TEXT NOT NULL,
            supported_feature_list_json TEXT NOT NULL,
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE UNIQUE INDEX idx_model_info_base_model ON model_info (base_model)
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE tts_tasks (
            id INTEGER PRIMARY KEY,
            history_id INTEGER NOT NULL,
            speaker_id INTEGER NOT NULL,
            model_path TEXT,
            base_model TEXT NOT NULL DEFAULT 'qwen3_tts',
            language TEXT NOT NULL,
            format TEXT NOT NULL,
            text TEXT NOT NULL,
            voice_prompt TEXT NOT NULL DEFAULT '',
            char_count INTEGER NOT NULL,
            file_name TEXT NOT NULL,
            output_file_path TEXT,
            hardware_type TEXT NOT NULL DEFAULT 'cuda',
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE model_training_tasks (
            id INTEGER PRIMARY KEY,
            history_id INTEGER NOT NULL,
            language TEXT NOT NULL,
            base_model TEXT NOT NULL DEFAULT 'qwen3_tts',
            model_name TEXT NOT NULL,
            sample_count INTEGER NOT NULL,
            samples_json TEXT NOT NULL DEFAULT '[]',
            notes_json TEXT NOT NULL DEFAULT '[]',
            output_speaker_id INTEGER,
            epoch_count INTEGER NOT NULL,
            batch_size INTEGER NOT NULL,
            gradient_accumulation_steps INTEGER NOT NULL DEFAULT 4,
            enable_gradient_checkpointing INTEGER NOT NULL DEFAULT 0,
            hardware_type TEXT NOT NULL DEFAULT 'cuda',
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE voice_clone_tasks (
            id INTEGER PRIMARY KEY,
            history_id INTEGER NOT NULL,
            base_model TEXT NOT NULL DEFAULT 'qwen3_tts',
            language TEXT NOT NULL,
            format TEXT NOT NULL DEFAULT 'wav',
            ref_audio_name TEXT NOT NULL,
            ref_audio_path TEXT NOT NULL,
            ref_text TEXT NOT NULL,
            text TEXT NOT NULL,
            char_count INTEGER NOT NULL,
            file_name TEXT NOT NULL,
            output_file_path TEXT,
            hardware_type TEXT NOT NULL DEFAULT 'cuda',
            create_time TEXT NOT NULL,
            modify_time TEXT NOT NULL,
            deleted INTEGER NOT NULL DEFAULT 0
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

    sqlx::query(
        r#"
        INSERT INTO model_info (
            id, base_model, model_name, model_scale_list_json,
            required_model_name_list_json, required_model_repo_id_list_json,
            supported_feature_list_json, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(1_i64)
    .bind("qwen3_tts")
    .bind("Qwen3-TTS")
    .bind(r#"["1.7B","0.6B"]"#)
    .bind(r#"["Qwen3-TTS-12Hz-1.7B-Base","Qwen3-TTS-Tokenizer-12Hz","Qwen3-TTS-12Hz-1.7B-CustomVoice"]"#)
    .bind(r#"["Qwen/Qwen3-TTS-12Hz-1.7B-Base","Qwen/Qwen3-TTS-Tokenizer-12Hz","Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice"]"#)
    .bind(r#"["text_to_speech","voice_clone","model_training"]"#)
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
            id, history_id, speaker_id, model_path, base_model, language, format,
            text, voice_prompt, char_count, file_name, output_file_path,
            hardware_type, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(1_i64)
    .bind(101_i64)
    .bind(1_i64)
    .bind(Option::<String>::None)
    .bind("qwen3_tts")
    .bind("chinese")
    .bind("wav")
    .bind("hello")
    .bind("warm and natural")
    .bind(5_i64)
    .bind("legacy-tts.wav")
    .bind(Option::<String>::None)
    .bind("cuda")
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO model_training_tasks (
            id, history_id, language, base_model, model_name, sample_count, samples_json,
            notes_json, output_speaker_id, epoch_count, batch_size,
            gradient_accumulation_steps, enable_gradient_checkpointing,
            hardware_type, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(2_i64)
    .bind(102_i64)
    .bind("chinese")
    .bind("qwen3_tts")
    .bind("legacy-model")
    .bind(1_i64)
    .bind("[]")
    .bind("[]")
    .bind(Some(1_i64))
    .bind(12_i64)
    .bind(3_i64)
    .bind(6_i64)
    .bind(1_i64)
    .bind("cuda")
    .bind("2026-04-01 10:00:00")
    .bind("2026-04-01 10:00:00")
    .bind(0_i64)
    .execute(&pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO voice_clone_tasks (
            id, history_id, base_model, language, format, ref_audio_name, ref_audio_path,
            ref_text, text, char_count, file_name, output_file_path,
            hardware_type, create_time, modify_time, deleted
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(3_i64)
    .bind(103_i64)
    .bind("qwen3_tts")
    .bind("chinese")
    .bind("wav")
    .bind("ref.wav")
    .bind("/tmp/ref.wav")
    .bind("参考")
    .bind("生成")
    .bind(2_i64)
    .bind("legacy-voice-clone.wav")
    .bind(Option::<String>::None)
    .bind("cuda")
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
