use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "task_history")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub task_type: String,
    pub title: String,
    pub speaker_id: Option<i64>,
    pub speaker_name_snapshot: String,
    pub status: String,
    pub duration_seconds: i64,
    pub create_time: String,
    pub modify_time: String,
    pub finished_time: Option<String>,
    pub error_message: Option<String>,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
