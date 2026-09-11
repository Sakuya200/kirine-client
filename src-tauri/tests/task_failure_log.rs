use anyhow::anyhow;
use kirine_client_lib::test_support::{
    models::HistoryTaskType, record_task_failure_log, LocalServiceHarness,
};

/// 早期校验失败（如模型未安装）发生在任何脚本执行之前，任务日志文件尚不存在。
/// 失败原因必须被落进按 `task_log_file_path` 约定定位的日志文件，
/// 供 `get_history_record` 的 taskLog（历史详情「任务日志」面板）读取展示。
#[tokio::test]
async fn record_task_failure_log_creates_missing_task_log_file() {
    let root = std::env::temp_dir().join(format!("task-failure-log-create-{}", std::process::id()));
    let log_dir = root.join("logs");
    let harness = LocalServiceHarness::new_with_log_dir("task-failure-log-create", &log_dir)
        .await
        .unwrap();

    let task_id = 4210;
    let error = anyhow!("模型 fake_model:v9 未安装，请先在模型管理页安装后再执行任务");

    record_task_failure_log(
        harness.service(),
        HistoryTaskType::VoiceClone,
        task_id,
        "fake_model",
        &error,
    );

    let task_log_path = log_dir.join("task").join("voice-clone-4210.log");
    let content = std::fs::read_to_string(&task_log_path).unwrap();
    assert!(content.contains("[pipeline] 任务执行失败 (base_model=fake_model)"));
    assert!(content.contains("模型 fake_model:v9 未安装"));

    harness.shutdown().await.unwrap();
    let _ = std::fs::remove_dir_all(&root);
}

/// 后置阶段失败时任务日志已存在（脚本输出已在其中），失败原因应以追加方式
/// 写入而非截断既有日志。
#[tokio::test]
async fn record_task_failure_log_appends_without_truncating_existing_log() {
    let root = std::env::temp_dir().join(format!("task-failure-log-append-{}", std::process::id()));
    let log_dir = root.join("logs");
    let harness = LocalServiceHarness::new_with_log_dir("task-failure-log-append", &log_dir)
        .await
        .unwrap();

    let task_id = 4211;
    let task_log_path = log_dir.join("task").join(format!("tts-{}.log", task_id));
    std::fs::create_dir_all(task_log_path.parent().unwrap()).unwrap();
    std::fs::write(&task_log_path, "[stdout]\nprevious script output\n").unwrap();

    let error = anyhow!("chain root").context("wrapped failure context");

    record_task_failure_log(
        harness.service(),
        HistoryTaskType::TextToSpeech,
        task_id,
        "fake_model",
        &error,
    );

    let content = std::fs::read_to_string(&task_log_path).unwrap();
    assert!(content.starts_with("[stdout]\nprevious script output\n"));
    assert!(content.contains("[pipeline] 任务执行失败 (base_model=fake_model)"));
    // {:#} 展开完整错误链
    assert!(content.contains("wrapped failure context: chain root"));

    harness.shutdown().await.unwrap();
    let _ = std::fs::remove_dir_all(&root);
}
