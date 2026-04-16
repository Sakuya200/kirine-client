use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "model_info")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub base_model: String,
    pub model_name: String,
    pub model_scale_list_json: String,
    pub required_model_name_list_json: String,
    pub required_model_repo_id_list_json: String,
    pub supported_feature_list_json: String,
    pub create_time: String,
    pub modify_time: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}