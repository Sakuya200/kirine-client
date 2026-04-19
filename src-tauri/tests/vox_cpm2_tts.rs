use kirine_client_lib::{test_support::LocalServiceHarness, Result};

#[tokio::test]
async fn voxcpm2_tts_task_creation_defers_inference_model_resolution_until_pipeline() -> Result<()> {
    let harness = LocalServiceHarness::new("vox-tts-missing-base-model").await?;

    let src_model_root = harness.ensure_src_model_root()?;
    let base_model_dir = src_model_root.join("base-models").join("VoxCPM2");
    assert!(!base_model_dir.exists());

    let result = harness.create_vox_preset_tts_task().await?;
    let stored_model_path = harness
        .tts_task_model_path(result.task_id)
        .await?
        .expect("expected stored voxcpm2 model path");

    assert_eq!(stored_model_path, "%SRC_MODEL_ROOT_PATH%/base-models/VoxCPM2");
    assert!(base_model_dir.exists());

    harness.shutdown().await
}