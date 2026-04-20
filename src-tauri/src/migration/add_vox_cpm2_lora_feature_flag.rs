use sea_orm::Statement;
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260419_000002_add_vox_cpm2_lora_feature_flag"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"
                UPDATE model_info
                SET supported_feature_list_json = '["text-to-speech","voice-clone","model-training","lora"]',
                    modify_time = CURRENT_TIMESTAMP
                WHERE base_model = 'vox_cpm2' AND deleted = 0
                "#
                .to_string(),
            ))
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute(Statement::from_string(
                manager.get_database_backend(),
                r#"
                UPDATE model_info
                SET supported_feature_list_json = '["text-to-speech","voice-clone","model-training"]',
                    modify_time = CURRENT_TIMESTAMP
                WHERE base_model = 'vox_cpm2' AND deleted = 0
                "#
                .to_string(),
            ))
            .await?;

        Ok(())
    }
}