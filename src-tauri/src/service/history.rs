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
    /// 任务详情（HistoryTaskDetailDialog 打开时按需拉取，替代全量 record 轮询）
    async fn get_history_record_detail(&self, history_id: i64) -> Result<serde_json::Value>;
    /// 任务日志 tail（maxBytes 缺省 64KB，超限对齐首个换行截断）
    async fn get_history_task_log(&self, history_id: i64, max_bytes: Option<u64>)
        -> Result<Option<String>>;
    async fn delete_history_record(
        &self,
        history_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool>;
    async fn update_task_status(&self, payload: UpdateTaskStatusPayload) -> Result<()>;
    async fn cancel_history_task(&self, history_id: i64) -> Result<bool>;
}
