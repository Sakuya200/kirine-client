use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "model_training_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub history_id: i64,
    pub language: String,
    pub base_model: String,
    pub model_scale: String,
    pub model_name: String,
    pub description: String,
    pub model_params_json: String,
    pub sample_count: i64,
    pub samples_json: String,
    pub notes_json: String,
    pub output_speaker_id: Option<i64>,
    pub create_time: String,
    pub modify_time: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
