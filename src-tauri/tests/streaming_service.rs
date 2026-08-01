//! 流式语音会话「启动清扫」逻辑：上次应用退出后残留的 Running 流式会话，
//! 在 `init_db` 时被标记为 Cancelled。非 streaming 的 Running 任务不应被误清扫。

use kirine_client_lib::test_support::models::{HistoryTaskType, TaskStatus};
use kirine_client_lib::test_support::LocalServiceHarness;

#[tokio::test]
async fn sweep_marks_stale_running_streaming_sessions_cancelled() {
    let harness = LocalServiceHarness::new("streaming_sweep")
        .await
        .expect("harness");
    harness
        .seed_history(
            1,
            HistoryTaskType::StreamingSpeech,
            "流式语音 1",
            TaskStatus::Running,
        )
        .await
        .expect("seed streaming running");
    harness
        .seed_history(
            2,
            HistoryTaskType::TextToSpeech,
            "TTS 2",
            TaskStatus::Running,
        )
        .await
        .expect("seed tts running");

    harness
        .sweep_stale_streaming_sessions()
        .await
        .expect("sweep");

    // 残留的流式 Running 会话被清扫为 Cancelled
    assert_eq!(
        harness.history_status(1).await.expect("streaming status"),
        TaskStatus::Cancelled
    );
    // 非 streaming 的 Running 不应被清扫
    assert_eq!(
        harness.history_status(2).await.expect("tts status"),
        TaskStatus::Running
    );

    harness.shutdown().await.expect("shutdown");
}
