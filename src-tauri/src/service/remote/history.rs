use async_trait::async_trait;

use super::RemoteService;
use crate::{
    service::{
        history::HistoryService,
        models::{
            HistoryFilter, HistoryRecord, HistoryRecordSummary, HistoryTaskType, Page,
            PageRequest, UpdateTaskStatusPayload,
        },
    },
    Result,
};

#[async_trait]
impl HistoryService for RemoteService {
    async fn list_history_records(
        &self,
        request: PageRequest<HistoryFilter>,
    ) -> Result<Page<HistoryRecordSummary>> {
        self.client
            .list_history_records(request.into())
            .await
            .map(Into::into)
    }

    async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        self.client.get_history_record(history_id).await
    }

    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        self.client
            .delete_history_record(history_id, task_type)
            .await
    }

    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<HistoryRecord> {
        self.client.update_task_status(payload).await
    }

    async fn cancel_history_task(&self, history_id: i64) -> Result<bool> {
        self.client.cancel_history_task(history_id).await
    }
}
