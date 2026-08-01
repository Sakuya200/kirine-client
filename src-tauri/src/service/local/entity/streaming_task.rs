use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "streaming_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub history_id: i64,
    pub base_model: String,
    pub model_version: String,
    pub language: String,
    pub device: String,
    pub model_params_json: String,
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub message_count: i64,
    pub create_time: String,
    pub modify_time: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
