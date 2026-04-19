use kirine_client_lib::{test_support::LocalServiceHarness, Result};

#[tokio::test]
async fn voxcpm2_model_info_exposes_lora_feature_flag() -> Result<()> {
    let harness = LocalServiceHarness::new("vox-training-feature-flag").await?;

    let vox = harness
        .list_model_infos()
        .await?
        .into_iter()
        .find(|item| item.base_model == "vox_cpm2" && item.model_scale == "2B")
        .expect("expected VoxCPM2 model info to exist");

    assert!(vox.supported_feature_list.iter().any(|feature| feature == "lora"));

    harness.shutdown().await
}

#[tokio::test]
async fn voxcpm2_training_task_persists_lora_params_into_model_params_json() -> Result<()> {
    let harness = LocalServiceHarness::new("vox-training-model-params").await?;

    let task = harness.create_vox_training_task().await?;
    let stored_model_params = harness
        .training_task_model_params_json(task.task_id)
        .await?
        .expect("expected persisted training model params");
    let parsed: serde_json::Value = serde_json::from_str(&stored_model_params)?;

    assert_eq!(parsed.get("useLora").and_then(serde_json::Value::as_bool), Some(true));
    assert_eq!(parsed.get("trainingMode").and_then(serde_json::Value::as_str), Some("lora"));
    assert_eq!(parsed.get("loraRank").and_then(serde_json::Value::as_i64), Some(24));
    assert_eq!(parsed.get("loraAlpha").and_then(serde_json::Value::as_i64), Some(48));
    assert_eq!(parsed.get("loraDropout").and_then(serde_json::Value::as_str), Some("0.15"));

    harness.shutdown().await
}

#[tokio::test]
async fn voxcpm2_training_task_accepts_legacy_training_mode_payload() -> Result<()> {
    let harness = LocalServiceHarness::new("vox-training-legacy-mode").await?;

    let task = harness
        .create_vox_training_task_with_params(serde_json::json!({
            "trainingMode": "full",
            "epochCount": 2,
            "batchSize": 4,
            "gradientAccumulationSteps": 1,
            "enableGradientCheckpointing": false
        }))
        .await?;
    let stored_model_params = harness
        .training_task_model_params_json(task.task_id)
        .await?
        .expect("expected persisted legacy-compatible model params");
    let parsed: serde_json::Value = serde_json::from_str(&stored_model_params)?;

    assert_eq!(parsed.get("useLora").and_then(serde_json::Value::as_bool), Some(false));
    assert_eq!(parsed.get("trainingMode").and_then(serde_json::Value::as_str), Some("full"));
    assert_eq!(parsed.get("loraRank").and_then(serde_json::Value::as_i64), Some(32));
    assert_eq!(parsed.get("loraAlpha").and_then(serde_json::Value::as_i64), Some(32));
    assert_eq!(parsed.get("loraDropout").and_then(serde_json::Value::as_str), Some("0.0"));

    harness.shutdown().await
}