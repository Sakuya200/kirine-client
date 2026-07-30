use kirine_client_lib::test_support::models::HistoryTaskType;
use kirine_client_lib::test_support::LocalServiceHarness;

#[tokio::test]
async fn streaming_tasks_table_exists_with_expected_columns() {
    let harness = LocalServiceHarness::new("streaming_schema")
        .await
        .expect("harness");
    assert!(harness
        .table_exists("streaming_tasks")
        .await
        .expect("table_exists"));
    for col in [
        "id",
        "history_id",
        "base_model",
        "model_version",
        "language",
        "device",
        "model_params_json",
        "context_file_path",
        "input_cache_file_path",
        "output_audio_dir",
        "message_count",
        "create_time",
        "modify_time",
        "deleted",
    ] {
        assert!(
            harness
                .table_has_column("streaming_tasks", col)
                .await
                .expect("table_has_column"),
            "streaming_tasks missing column {col}"
        );
    }
    assert_eq!(
        HistoryTaskType::StreamingSpeech.as_str(),
        "streaming-speech"
    );
    assert_eq!(HistoryTaskType::StreamingSpeech.storage_dir(), "streaming");
    harness.shutdown().await.expect("shutdown");
}
