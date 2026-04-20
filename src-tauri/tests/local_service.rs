use kirine_client_lib::{test_support::LocalServiceHarness, Result};

#[tokio::test]
async fn local_service_bootstraps_missing_database_file() -> Result<()> {
    let harness = LocalServiceHarness::new("bootstrap").await?;

    assert!(harness.database_file_exists());
    assert!(harness.speakers_query_succeeds().await?);

    harness.shutdown().await
}

#[tokio::test]
async fn speaker_crud_round_trip_uses_local_database() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-crud").await?;

    let created = harness.create_test_speaker().await?;
    assert!(created.id > 0);

    let listed = harness.list_speakers().await?;
    assert!(listed
        .iter()
        .any(|speaker| speaker.name == "SeaOrm Speaker"));

    let updated = harness.update_test_speaker(created.id).await?;
    assert_eq!(updated.name, "Updated Speaker");
    assert_eq!(updated.description, "updated by test");

    let deleted = harness.delete_speaker(created.id).await?;
    assert!(deleted);
    assert!(harness
        .list_speakers()
        .await?
        .iter()
        .all(|speaker| speaker.id != created.id));

    harness.shutdown().await
}

#[tokio::test]
async fn local_service_migrates_legacy_schema_without_compat_layer() -> Result<()> {
    let harness = LocalServiceHarness::new_with_legacy_schema("legacy-schema").await?;

    let speakers = harness.list_speakers().await?;
    let legacy = speakers
        .iter()
        .find(|speaker| speaker.name == "Legacy Speaker")
        .expect("legacy speaker should exist after migration");
    assert_eq!(legacy.description, "");
    assert_eq!(legacy.samples, 2);

    harness.shutdown().await
}

#[tokio::test]
async fn local_service_migrates_legacy_task_tables_to_surrogate_ids() -> Result<()> {
    let harness =
        LocalServiceHarness::new_with_legacy_task_detail_schema("legacy-task-detail-ids").await?;

    assert!(harness.table_has_column("tts_tasks", "id").await?);
    assert!(
        harness
            .table_has_column("model_training_tasks", "id")
            .await?
    );
    assert!(harness.table_has_column("voice_clone_tasks", "id").await?);

    assert_eq!(
        harness.task_detail_id_for_history("tts_tasks", 101).await?,
        Some(1)
    );
    assert_eq!(
        harness
            .task_detail_id_for_history("model_training_tasks", 102)
            .await?,
        Some(1)
    );
    assert_eq!(
        harness
            .task_detail_id_for_history("voice_clone_tasks", 103)
            .await?,
        Some(1)
    );

    harness.shutdown().await
}
