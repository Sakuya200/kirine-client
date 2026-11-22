//! 任务列表失效事件发射通道。
//!
//! service 层深处（update_task_status/create_*/delete 等）无法直接拿到
//! AppHandle，这里用模块级 `OnceLock` 在应用启动时注册一次；未注册时
//! （如集成测试、非 Tauri 环境）发射静默跳过，业务路径不受影响。

use std::sync::OnceLock;

use tauri::Emitter;
use tracing::warn;

use super::models::{
    HistoryTaskEvent, HistoryTaskEventAction, HistoryTaskType, HISTORY_TASK_UPDATED_EVENT,
};
use crate::service::models::TaskStatus;

static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

pub(crate) fn register_app_handle(app: tauri::AppHandle) {
    let _ = APP_HANDLE.set(app);
}

/// 发射任务列表失效信号事件。只做通知，不承载 detail/taskLog。
pub(crate) fn emit_history_task_event(
    history_id: i64,
    task_type: HistoryTaskType,
    status: TaskStatus,
    modify_time: &str,
    action: HistoryTaskEventAction,
) {
    let Some(app) = APP_HANDLE.get() else {
        return;
    };
    let event = HistoryTaskEvent {
        history_id,
        task_type,
        status,
        modify_time: modify_time.to_string(),
        action,
    };
    if let Err(err) = app.emit(HISTORY_TASK_UPDATED_EVENT, event) {
        warn!(
            error = %err,
            history_id,
            event = HISTORY_TASK_UPDATED_EVENT,
            "failed to emit history task event"
        );
    }
}
