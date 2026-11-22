use async_trait::async_trait;

use crate::{
    service::models::{CreateModelTrainingTaskPayload, ModelTrainingTaskResult},
    Result,
};

#[async_trait]
pub trait ModelTrainingService: Send + Sync {
    async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult>;
}
