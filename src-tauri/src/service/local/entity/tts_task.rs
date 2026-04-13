use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tts_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub history_id: i64,
    pub speaker_id: i64,
    pub model_path: Option<String>,
    pub base_model: String,
    pub hardware_type: String,
    pub language: String,
    pub format: String,
    pub text: String,
    pub voice_prompt: String,
    pub char_count: i64,
    pub file_name: String,
    pub output_file_path: Option<String>,
    pub create_time: String,
    pub modify_time: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
