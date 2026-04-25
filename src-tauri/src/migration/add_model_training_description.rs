use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260425_000001_add_model_training_description"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let connection = manager.get_connection();
        connection
            .execute_unprepared(
                "ALTER TABLE model_training_tasks ADD COLUMN description TEXT NOT NULL DEFAULT ''",
            )
            .await
            .map_err(|err| DbErr::Migration(format!("failed to add description column to model_training_tasks: {err}")))?;

        connection
            .execute_unprepared(
                r#"
                UPDATE model_training_tasks
                SET description = (
                    SELECT COALESCE(speakers.description, '')
                    FROM task_history
                    LEFT JOIN speakers ON speakers.id = task_history.speaker_id AND speakers.deleted = 0
                    WHERE task_history.id = model_training_tasks.history_id AND task_history.deleted = 0
                )
                WHERE deleted = 0
                "#,
            )
            .await
            .map_err(|err| DbErr::Migration(format!("failed to backfill model training descriptions: {err}")))?;

        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}