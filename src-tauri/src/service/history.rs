use async_trait::async_trait;

use crate::{
    service::models::{
        HistoryFilter, HistoryRecord, HistoryRecordSummary, HistoryTaskType, Page, PageRequest,
        UpdateTaskStatusPayload,
    },
    Result,
};

#[async_trait]
pub trait HistoryService: Send + Sync {
    async fn list_history_records(
        &self,
        request: PageRequest<HistoryFilter>,
    ) -> Result<Page<HistoryRecordSummary>>;
    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord>;
    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool>;
    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<()>;
    async fn cancel_history_task(&self, history_id: i64) -> Result<bool>;
}
