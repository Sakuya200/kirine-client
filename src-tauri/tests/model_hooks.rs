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

#[tokio::test]
async fn sync_backfills_current_device_for_single_device_models() -> Result<()> {
    // sync_supported_models 的回填契约：
    // - 单设备模型：current_device 自动回填为唯一设备（如 gpt_sovits_cpufast -> cpu）
    // - 多设备模型：current_device 留空（要求用户先选）
    // 覆盖 upsert insert 分支的 resolve_current_device_value(None, ...)。
    let harness = LocalServiceHarness::new("model-current-device-backfill").await?;
    let models = harness.list_model_infos().await?;

    let mut seen_single = false;
    for m in &models {
        if m.supported_devices.len() == 1 {
            seen_single = true;
            assert_eq!(
                m.current_device,
                Some(m.supported_devices[0]),
                "单设备模型 {} 的 current_device 应被回填",
                m.base_model
            );
        } else {
            assert!(
                m.current_device.is_none(),
                "多设备模型 {} 初始 current_device 应为 None",
                m.base_model
            );
        }
    }
    assert!(
        seen_single,
        "测试需要至少一个单设备模型以验证回填（检查 src-model 配置）"
    );

    harness.shutdown().await
}

#[tokio::test]
async fn set_model_current_device_valid_updates_and_persists() -> Result<()> {
    // 合法设备写入：返回值含新 currentDevice，且重新列表读取仍保留（跨会话持久化）。
    let harness = LocalServiceHarness::new("model-set-device-valid").await?;
    let models = harness.list_model_infos().await?;

    let target = models
        .iter()
        .find(|m| m.supported_devices.len() >= 2)
        .expect("需要至少一个多设备模型");
    assert!(
        target.current_device.is_none(),
        "多设备模型初始 current_device 应为 None"
    );
    let device = target.supported_devices[0];

    let updated = harness
        .service()
        .set_model_current_device(target.id, device)
        .await?;
    assert_eq!(updated.id, target.id);
    assert_eq!(updated.current_device, Some(device));

    let reloaded = harness.list_model_infos().await?;
    let reloaded_target = reloaded
        .iter()
        .find(|m| m.id == target.id)
        .expect("模型应存在");
    assert_eq!(reloaded_target.current_device, Some(device));

    harness.shutdown().await
}

#[tokio::test]
async fn set_model_current_device_rejects_unsupported_device() -> Result<()> {
    // 非法设备：单设备模型写入其不支持的另一设备 -> Err（不触达脚本，仅 DB 校验）。
    let harness = LocalServiceHarness::new("model-set-device-invalid").await?;
    let models = harness.list_model_infos().await?;

    let target = models
        .iter()
        .find(|m| m.supported_devices.len() == 1)
        .expect("需要至少一个单设备模型");
    let only = target.supported_devices[0];
    let other = if only == HardwareType::Cpu {
        HardwareType::Cuda
    } else {
        HardwareType::Cpu
    };

    let err = harness
        .service()
        .set_model_current_device(target.id, other)
        .await;
    assert!(err.is_err(), "不支持设备应报错");

    // 原值不被破坏
    let reloaded = harness.list_model_infos().await?;
    let reloaded_target = reloaded
        .iter()
        .find(|m| m.id == target.id)
        .expect("模型应存在");
    assert_eq!(reloaded_target.current_device, Some(only));

    harness.shutdown().await
}

#[tokio::test]
async fn set_model_current_device_errors_for_unknown_id() -> Result<()> {
    // 负路径：不存在的 model_id 在行查找阶段即 Err。
    let harness = LocalServiceHarness::new("model-set-device-unknown").await?;

    let err = harness
        .service()
        .set_model_current_device(999_999, HardwareType::Cpu)
        .await;
    assert!(err.is_err(), "未知 model_id 应报错");

    harness.shutdown().await
}
