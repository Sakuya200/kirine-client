use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260508_000002_make_tts_speaker_nullable"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let rows = db
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA table_info('tts_tasks')".to_string(),
            ))
            .await?;

        let mut speaker_id_is_not_null = false;
        for row in rows {
            let name: String = row.try_get("", "name")?;
            if name != "speaker_id" {
                continue;
            }

            let not_null: i64 = row.try_get("", "notnull")?;
            speaker_id_is_not_null = not_null != 0;
            break;
        }

        if !speaker_id_is_not_null {
            return Ok(());
        }

        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA foreign_keys = OFF".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "BEGIN IMMEDIATE TRANSACTION".to_string(),
        ))
        .await?;

        let migration_result = async {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "ALTER TABLE tts_tasks RENAME TO tts_tasks__legacy_required_speaker".to_string(),
            ))
            .await?;
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "CREATE TABLE tts_tasks (id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, history_id INTEGER NOT NULL, speaker_id INTEGER, model_path TEXT, base_model TEXT NOT NULL, model_scale TEXT NOT NULL, language TEXT NOT NULL, format TEXT NOT NULL, export_audio_name TEXT NOT NULL, text TEXT NOT NULL, model_params_json TEXT NOT NULL DEFAULT '{}', char_count INTEGER NOT NULL, file_name TEXT NOT NULL, output_file_path TEXT, create_time TEXT NOT NULL, modify_time TEXT NOT NULL, deleted INTEGER NOT NULL DEFAULT 0, CONSTRAINT fk_tts_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE, CONSTRAINT fk_tts_tasks_speaker FOREIGN KEY (speaker_id) REFERENCES speakers (id))".to_string(),
            ))
            .await?;
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "INSERT INTO tts_tasks (id, history_id, speaker_id, model_path, base_model, model_scale, language, format, export_audio_name, text, model_params_json, char_count, file_name, output_file_path, create_time, modify_time, deleted) SELECT id, history_id, speaker_id, model_path, base_model, model_scale, language, format, export_audio_name, text, model_params_json, char_count, file_name, output_file_path, create_time, modify_time, deleted FROM tts_tasks__legacy_required_speaker".to_string(),
            ))
            .await?;
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "DROP TABLE tts_tasks__legacy_required_speaker".to_string(),
            ))
            .await?;
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "CREATE UNIQUE INDEX IF NOT EXISTS idx_tts_tasks_history_id ON tts_tasks (history_id)".to_string(),
            ))
            .await?;

            Result::<(), DbErr>::Ok(())
        }
        .await;

        match migration_result {
            Ok(()) => {
                db.execute(Statement::from_string(DbBackend::Sqlite, "COMMIT".to_string()))
                    .await?;
                db.execute(Statement::from_string(
                    DbBackend::Sqlite,
                    "PRAGMA foreign_keys = ON".to_string(),
                ))
                .await?;
                Ok(())
            }
            Err(err) => {
                let _ = db
                    .execute(Statement::from_string(DbBackend::Sqlite, "ROLLBACK".to_string()))
                    .await;
                let _ = db
                    .execute(Statement::from_string(
                        DbBackend::Sqlite,
                        "PRAGMA foreign_keys = ON".to_string(),
                    ))
                    .await;
                Err(err)
            }
        }
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}