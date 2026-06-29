//! 覆盖 4 个 model hook 对应的 Service 层方法：
//! `list_model_infos` / `get_device_type` / `install_model` / `uninstall_model`。
//!
//! 注意：`install_model` / `uninstall_model` / `get_device_type` 在「已找到模型行」后会
//! 触发真实 shell 脚本或删除真实 `src-model` 产物（测试期 `src_model_root` 解析到真实
//! 仓库 src-model）。因此：
//! - 对这三个 hook 仅测**负路径**（不存在的 model_id / base_model → 在行查找阶段即 Err，
//!   不触达脚本与文件系统），保证隔离与安全；
//! - `install_model` 的脚本调用参数契约由 `pipeline_params.rs`（规则4）覆盖；
//! - 模型下载标志的 DB 翻转（install/uninstall 的 DB 侧）通过 `model_downloaded` /
//!   `set_model_downloaded` 直接校验。
//!
//! `list_model_infos` 为纯读（sync 已在 harness 建库时完成），可完整测分页/筛选/字段。

use kirine_client_lib::test_support::{
    models::{HistoryTaskType, ModelFilter},
    HardwareType, LocalServiceHarness, PageRequest, Service,
};
use kirine_client_lib::Result;

const QWEN3_BASE: &str = "qwen3_tts";
const QWEN3_VERSION: &str = "1.7B";

#[tokio::test]
async fn list_model_infos_syncs_six_models_with_full_fields() -> Result<()> {
    let harness = LocalServiceHarness::new("model-list").await?;

    let models = harness.list_model_infos().await?;
    assert!(models.len() >= 6, "expected >=6 synced models, got {}", models.len());

    // 每个模型应携带 supported_languages / supported_devices / 特性矩阵
    for m in &models {
        assert!(!m.base_model.is_empty());
        assert!(!m.supported_languages.is_empty(), "{} missing languages", m.base_model);
        assert!(!m.supported_devices.is_empty(), "{} missing devices", m.base_model);
        assert!(!m.supported_feature_list.is_empty(), "{} missing features", m.base_model);
    }

    // 至少包含 qwen3_tts
    assert!(models.iter().any(|m| m.base_model == QWEN3_BASE));

    harness.shutdown().await
}

#[tokio::test]
async fn list_model_infos_keyword_filter_matches() -> Result<()> {
    let harness = LocalServiceHarness::new("model-filter-keyword").await?;

    let filtered = harness
        .service()
        .list_model_infos(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(ModelFilter {
                keyword: Some("qwen".to_string()),
                downloaded: None,
                feature: None,
            }),
        })
        .await?;

    assert!(filtered.total >= 1);
    assert!(filtered
        .items
        .iter()
        .all(|m| m.base_model.contains("qwen") || m.model_name.contains("qwen")));

    harness.shutdown().await
}

#[tokio::test]
async fn list_model_infos_feature_filter_matches_supported() -> Result<()> {
    let harness = LocalServiceHarness::new("model-filter-feature").await?;

    let filtered = harness
        .service()
        .list_model_infos(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(ModelFilter {
                keyword: None,
                downloaded: None,
                feature: Some(HistoryTaskType::TextToSpeech),
            }),
        })
        .await?;

    // qwen3_tts 支持 text-to-speech，应被命中
    assert!(filtered.items.iter().any(|m| m.base_model == QWEN3_BASE));
    assert!(filtered.items.iter().all(|m| m
        .supported_feature_list
        .iter()
        .any(|f| f == "text-to-speech")));

    harness.shutdown().await
}

#[tokio::test]
async fn get_device_type_errors_for_unknown_model() -> Result<()> {
    let harness = LocalServiceHarness::new("model-device-unknown").await?;

    let err = harness
        .service()
        .get_device_type("does_not_exist", "0.0")
        .await;
    assert!(err.is_err(), "expected error for unknown model");

    harness.shutdown().await
}

#[tokio::test]
async fn install_model_errors_for_unknown_id() -> Result<()> {
    // 负路径：不存在的 model_id 在行查找阶段即 Err，不触达真实脚本（规则4 + 隔离）。
    let harness = LocalServiceHarness::new("model-install-unknown").await?;

    let err = harness
        .service()
        .install_model(999_999, HardwareType::Cpu)
        .await;
    assert!(err.is_err(), "expected error for unknown model id");

    harness.shutdown().await
}

#[tokio::test]
async fn uninstall_model_errors_for_unknown_id() -> Result<()> {
    // 负路径：不存在的 model_id 不触达真实文件系统删除（隔离）。
    let harness = LocalServiceHarness::new("model-uninstall-unknown").await?;

    let err = harness.service().uninstall_model(999_999).await;
    assert!(err.is_err(), "expected error for unknown model id");

    harness.shutdown().await
}

#[tokio::test]
async fn model_downloaded_flag_round_trips_in_db() -> Result<()> {
    // 覆盖 install/uninstall 的 DB 侧：下载标志可读写翻转，不执行真实脚本。
    let harness = LocalServiceHarness::new("model-downloaded-flag").await?;

    // 初始 synced 模型默认未下载
    let initial = harness.model_downloaded(QWEN3_BASE, QWEN3_VERSION).await?;
    assert!(!initial);

    harness
        .set_model_downloaded(QWEN3_BASE, QWEN3_VERSION, true)
        .await?;
    assert!(harness.model_downloaded(QWEN3_BASE, QWEN3_VERSION).await?);

    harness
        .set_model_downloaded(QWEN3_BASE, QWEN3_VERSION, false)
        .await?;
    assert!(!harness.model_downloaded(QWEN3_BASE, QWEN3_VERSION).await?);

    harness.shutdown().await
}
