use async_trait::async_trait;

use crate::{
    config::HardwareType,
    service::models::{ModelFilter, ModelInfo, ModelMutationResult, Page, PageRequest},
    Result,
};

#[async_trait]
pub trait ModelService: Send + Sync {
    async fn list_model_infos(&self, request: PageRequest<ModelFilter>) -> Result<Page<ModelInfo>>;
    async fn get_device_type(&self, base_model: &str, model_version: &str) -> Result<HardwareType>;
    async fn install_model(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelMutationResult>;
    async fn uninstall_model(&self, model_id: i64) -> Result<ModelMutationResult>;
    async fn set_model_current_device(
        &self,
        model_id: i64,
        device: HardwareType,
    ) -> Result<ModelInfo>;
}
