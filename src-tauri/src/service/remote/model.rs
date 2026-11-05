use async_trait::async_trait;

use super::RemoteService;
use crate::{
    config::HardwareType,
    service::{
        model::ModelService,
        models::{ModelFilter, ModelInfo, ModelMutationResult, Page, PageRequest},
    },
    Result,
};

#[async_trait]
impl ModelService for RemoteService {
    async fn list_model_infos(&self, request: PageRequest<ModelFilter>) -> Result<Page<ModelInfo>> {
        self.client
            .list_model_infos(request.into())
            .await
            .map(Into::into)
    }

    async fn get_device_type(&self, base_model: &str, model_version: &str) -> Result<HardwareType> {
        self.client.get_device_type(base_model, model_version).await
    }

    async fn install_model(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult> {
        self.client.install_model(model_id, device).await
    }

    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult> {
        self.client.uninstall_model(model_id).await
    }

    async fn set_model_current_device(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelInfo> {
        self.client.set_model_current_device(model_id, device).await
    }
}
