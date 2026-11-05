use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        models::{CreateModelTrainingTaskPayload, ModelTrainingTaskResult},
        training::ModelTrainingService,
    },
    Result,
};

#[async_trait]
impl ModelTrainingService for RemoteService {
    async fn create_model_training_task(
        &self,
        payload: CreateModelTrainingTaskPayload,
    ) -> Result<ModelTrainingTaskResult> {
        self.client.create_model_training_task(payload).await
    }
}
