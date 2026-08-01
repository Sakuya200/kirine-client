//! 覆盖 history hook 对应的 Service 层方法（除 4 个 `create_*_task` 外）：
//! `list_history_records` / `get_history_record` / `read_text_to_speech_audio`
//! / `read_voice_clone_audio` / `read_voice_design_audio` / `delete_history_record`
//! / `update_task_status` / `cancel_history_task`。
//!
//! 4 个 `create_*_task` 涉及任务执行，按规则4 不真正执行任务——其脚本调用参数契约在
//! `pipeline_params.rs` 覆盖。本文件的 history 行通过 `LocalServiceHarness` 的 SQL 种子
//! helper 直接插入，避免触发 `create_*_task` 的后台 pipeline（测试期 `src_model_root`
//! 解析到真实仓库 src-model，调用 create_*_task 会真实执行脚本）。

use std::fs;

use kirine_client_lib::test_support::{
    models::{HistoryFilter, HistoryTaskType, TaskStatus, UpdateTaskStatusPayload},
    LocalServiceHarness, PageRequest, Service,
};
use kirine_client_lib::Result;

const BASE: &str = "qwen3_tts";
const VERSION: &str = "1.7B";

fn write_temp_audio(harness: &LocalServiceHarness, name: &str) -> std::path::PathBuf {
    let path = harness.app_dir().join(name);
    fs::write(&path, b"RIFF....fake-audio-bytes").expect("write temp audio");
    path
}

#[tokio::test]
async fn list_history_records_pagination_and_filter() -> Result<()> {
    let harness = LocalServiceHarness::new("history-list").await?;

    harness
        .seed_history(
            1,
            HistoryTaskType::TextToSpeech,
            "tts-one",
            TaskStatus::Completed,
        )
        .await?;
    harness
        .seed_history(
            2,
            HistoryTaskType::TextToSpeech,
            "tts-two",
            TaskStatus::Pending,
        )
        .await?;
    harness
        .seed_history(3, HistoryTaskType::VoiceClone, "vc-one", TaskStatus::Failed)
        .await?;

    // 分页
    let paged = harness
        .service()
        .list_history_records(PageRequest {
            page: 1,
            page_size: 2,
            filter: None,
        })
        .await?;
    assert_eq!(paged.page_size, 2);
    assert_eq!(paged.items.len(), 2);
    assert!(paged.total >= 3);

    // summary 不含 detail / task_log
    for item in &paged.items {
        // HistoryRecordSummary 字段：无 detail/task_log，编译期保证；这里校验基础字段
        assert!(!item.title.is_empty());
    }

    // 筛选：task_type
    let tts_only = harness
        .service()
        .list_history_records(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(HistoryFilter {
                keyword: None,
                task_type: Some(HistoryTaskType::TextToSpeech),
                status: None,
            }),
        })
        .await?;
    assert!(tts_only
        .items
        .iter()
        .all(|h| h.task_type == HistoryTaskType::TextToSpeech));
    assert!(tts_only.total >= 2);

    // 筛选：status
    let completed = harness
        .service()
        .list_history_records(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(HistoryFilter {
                keyword: None,
                task_type: None,
                status: Some(TaskStatus::Completed),
            }),
        })
        .await?;
    assert!(completed
        .items
        .iter()
        .all(|h| h.status == TaskStatus::Completed));

    // 筛选：keyword
    let kw = harness
        .service()
        .list_history_records(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(HistoryFilter {
                keyword: Some("vc".to_string()),
                task_type: None,
                status: None,
            }),
        })
        .await?;
    assert!(kw.items.iter().all(|h| h.title.contains("vc")));

    harness.shutdown().await
}

#[tokio::test]
async fn get_history_record_returns_full_detail() -> Result<()> {
    let harness = LocalServiceHarness::new("history-get").await?;

    harness
        .seed_history(
            10,
            HistoryTaskType::TextToSpeech,
            "detail-tts",
            TaskStatus::Completed,
        )
        .await?;
    harness.seed_tts_detail(10, BASE, VERSION, None).await?;

    let record = harness.get_history_record(10).await?;
    assert_eq!(record.id, 10);
    assert_eq!(record.task_type, HistoryTaskType::TextToSpeech);
    assert_eq!(record.status, TaskStatus::Completed);
    // detail 为非空 JSON 对象
    let detail = record
        .detail
        .as_object()
        .expect("detail should be an object");
    assert!(!detail.is_empty());
    assert_eq!(detail.get("baseModel").and_then(|v| v.as_str()), Some(BASE));

    harness.shutdown().await
}

#[tokio::test]
async fn read_text_to_speech_audio_returns_bytes() -> Result<()> {
    let harness = LocalServiceHarness::new("history-read-tts").await?;
    let audio = write_temp_audio(&harness, "tts-out.wav");

    harness
        .seed_history(
            20,
            HistoryTaskType::TextToSpeech,
            "read-tts",
            TaskStatus::Completed,
        )
        .await?;
    harness
        .seed_tts_detail(20, BASE, VERSION, Some(&audio.to_string_lossy()))
        .await?;

    let asset = harness.service().read_text_to_speech_audio(20).await?;
    assert_eq!(asset.task_id, 20);
    assert!(!asset.bytes.is_empty());
    assert_eq!(asset.bytes, b"RIFF....fake-audio-bytes");

    harness.shutdown().await
}

#[tokio::test]
async fn read_voice_clone_audio_returns_bytes() -> Result<()> {
    let harness = LocalServiceHarness::new("history-read-vc").await?;
    let audio = write_temp_audio(&harness, "vc-out.wav");

    harness
        .seed_history(
            21,
            HistoryTaskType::VoiceClone,
            "read-vc",
            TaskStatus::Completed,
        )
        .await?;
    harness
        .seed_voice_clone_detail(21, BASE, VERSION, Some(&audio.to_string_lossy()))
        .await?;

    let asset = harness.service().read_voice_clone_audio(21).await?;
    assert_eq!(asset.task_id, 21);
    assert!(!asset.bytes.is_empty());

    harness.shutdown().await
}

#[tokio::test]
async fn read_voice_design_audio_returns_bytes() -> Result<()> {
    let harness = LocalServiceHarness::new("history-read-vd").await?;
    let audio = write_temp_audio(&harness, "vd-out.wav");

    harness
        .seed_history(
            22,
            HistoryTaskType::VoiceDesign,
            "read-vd",
            TaskStatus::Completed,
        )
        .await?;
    harness
        .seed_voice_design_detail(22, BASE, VERSION, Some(&audio.to_string_lossy()))
        .await?;

    let asset = harness.service().read_voice_design_audio(22).await?;
    assert_eq!(asset.task_id, 22);
    assert!(!asset.bytes.is_empty());

    harness.shutdown().await
}

#[tokio::test]
async fn read_audio_errors_when_task_not_completed() -> Result<()> {
    let harness = LocalServiceHarness::new("history-read-pending").await?;
    let audio = write_temp_audio(&harness, "pending-out.wav");

    harness
        .seed_history(
            23,
            HistoryTaskType::TextToSpeech,
            "pending-tts",
            TaskStatus::Pending,
        )
        .await?;
    harness
        .seed_tts_detail(23, BASE, VERSION, Some(&audio.to_string_lossy()))
        .await?;

    let err = harness.service().read_text_to_speech_audio(23).await;
    assert!(
        err.is_err(),
        "reading audio of a non-completed task should error"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn delete_history_record_soft_deletes_row_and_detail() -> Result<()> {
    let harness = LocalServiceHarness::new("history-delete").await?;

    harness
        .seed_history(
            30,
            HistoryTaskType::TextToSpeech,
            "del-tts",
            TaskStatus::Completed,
        )
        .await?;
    harness.seed_tts_detail(30, BASE, VERSION, None).await?;
    assert!(harness
        .task_detail_id_for_history("tts_tasks", 30)
        .await?
        .is_some());

    let deleted = harness
        .service()
        .delete_history_record(30, HistoryTaskType::TextToSpeech)
        .await?;
    assert!(deleted);

    // history 与 detail 均软删
    let list = harness.list_history_records().await?;
    assert!(list.iter().all(|h| h.id != 30));
    assert!(harness
        .task_detail_id_for_history("tts_tasks", 30)
        .await?
        .is_none());

    // 重复删除已软删行 → false
    let again = harness
        .service()
        .delete_history_record(30, HistoryTaskType::TextToSpeech)
        .await?;
    assert!(!again);

    harness.shutdown().await
}

#[tokio::test]
async fn update_task_status_persists_transition() -> Result<()> {
    let harness = LocalServiceHarness::new("history-update-status").await?;

    harness
        .seed_history(
            40,
            HistoryTaskType::TextToSpeech,
            "status-tts",
            TaskStatus::Pending,
        )
        .await?;
    harness.seed_tts_detail(40, BASE, VERSION, None).await?;

    let updated = harness
        .service()
        .update_task_status(UpdateTaskStatusPayload {
            task_id: 40,
            status: TaskStatus::Completed,
            duration_seconds: Some(12),
        })
        .await?;
    assert_eq!(updated.id, 40);
    assert_eq!(updated.status, TaskStatus::Completed);

    // 落库校验
    let record = harness.get_history_record(40).await?;
    assert_eq!(record.status, TaskStatus::Completed);

    harness.shutdown().await
}

#[tokio::test]
async fn cancel_history_task_rejects_finished_task() -> Result<()> {
    let harness = LocalServiceHarness::new("history-cancel-finished").await?;
    harness
        .seed_history(
            50,
            HistoryTaskType::TextToSpeech,
            "finished",
            TaskStatus::Completed,
        )
        .await?;

    let err = harness.service().cancel_history_task(50).await;
    assert!(err.is_err(), "cancelling a finished task should error");

    harness.shutdown().await
}

#[tokio::test]
async fn cancel_history_task_errors_without_active_control() -> Result<()> {
    // Pending 任务但无后台 pipeline 注册的 ActiveTaskControl（未走 create_*_task），
    // cancel 应返回 Err（无可用终止句柄），不真正执行任务。
    let harness = LocalServiceHarness::new("history-cancel-no-control").await?;
    harness
        .seed_history(
            51,
            HistoryTaskType::TextToSpeech,
            "pending-no-ctrl",
            TaskStatus::Pending,
        )
        .await?;

    let err = harness.service().cancel_history_task(51).await;
    assert!(
        err.is_err(),
        "cancelling a task without active control should error"
    );

    harness.shutdown().await
}
