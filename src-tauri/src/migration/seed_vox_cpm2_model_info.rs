use sea_orm::Statement;
use sea_orm_migration::prelude::*;

const DEFAULT_TIMESTAMP: &str = "2026-04-17 00:00:00";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                format!(
                    r#"INSERT OR REPLACE INTO model_info (id, base_model, model_name, model_scale, required_model_name_list_json, required_model_repo_id_list_json, supported_feature_list_json, create_time, modify_time, deleted) VALUES (3, 'vox_cpm2', 'VoxCPM2', '2B', '["VoxCPM2"]', '["openbmb/VoxCPM2"]', '["text-to-speech","voice-clone","model-training"]', '{DEFAULT_TIMESTAMP}', '{DEFAULT_TIMESTAMP}', 0)"#
                ),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                "DELETE FROM model_info WHERE id = 3 OR base_model = 'vox_cpm2'".to_string(),
            ))
            .await?;
        Ok(())
    }
}
