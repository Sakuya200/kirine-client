//! 覆盖 5 个 speaker hook 对应的 Service 层方法（hooks 为透传，真实逻辑在 Service 层）：
//! `create_speaker_info` / `list_speaker_infos` / `import_model_as_speaker`
//! / `update_speaker_info` / `delete_speaker_info`。
//!
//! 每个 `#[tokio::test]` 经 `LocalServiceHarness::new` 起独立临时库（规则1&2）。

use std::fs;

use kirine_client_lib::test_support::{
    models::{CreateSpeakerPayload, ImportModelAsSpeakerPayload, SpeakerSource, SpeakerStatus},
    LocalServiceHarness, PageRequest, Service, SpeakerFilter,
};
use kirine_client_lib::Result;

#[tokio::test]
async fn create_speaker_info_persists_row_and_returns_id() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-create").await?;

    let created = harness
        .service()
        .create_speaker_info(CreateSpeakerPayload {
            name: "Alice".to_string(),
            samples: 5,
            base_model: "qwen3_tts".to_string(),
            description: "test speaker".to_string(),
            status: SpeakerStatus::Ready,
            source: SpeakerSource::Local,
        })
        .await?;

    assert!(created.id > 0);
    assert_eq!(created.name, "Alice");
    assert_eq!(created.samples, 5);
    assert_eq!(created.status, SpeakerStatus::Ready);
    assert_eq!(created.source, SpeakerSource::Local);
    assert_eq!(created.base_model, "qwen3_tts");

    // 落库：list 应能查到
    let all = harness.list_speakers().await?;
    assert!(all.iter().any(|s| s.id == created.id));

    harness.shutdown().await
}

#[tokio::test]
async fn list_speaker_infos_pagination_filter_and_stats() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-list").await?;

    // 造 3 条 ready + 1 条 training
    for i in 0..3 {
        harness
            .service()
            .create_speaker_info(CreateSpeakerPayload {
                name: format!("Ready-{i}"),
                samples: 2,
                base_model: "qwen3_tts".to_string(),
                description: "".to_string(),
                status: SpeakerStatus::Ready,
                source: SpeakerSource::Local,
            })
            .await?;
    }
    harness
        .service()
        .create_speaker_info(CreateSpeakerPayload {
            name: "Training-One".to_string(),
            samples: 0,
            base_model: "qwen3_tts".to_string(),
            description: "".to_string(),
            status: SpeakerStatus::Training,
            source: SpeakerSource::Local,
        })
        .await?;

    // 分页：page_size=2，第 1 页返回 2 条，total>=4，total_pages 向上取整
    let paged = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 2,
            filter: None,
        })
        .await?;
    assert_eq!(paged.page, 1);
    assert_eq!(paged.page_size, 2);
    assert!(paged.total >= 4);
    assert_eq!(paged.items.len(), 2);
    assert_eq!(
        paged.total_pages,
        ((paged.total as u32) + 2 - 1) / 2
    );
    // 统计字段
    assert!(paged.ready_count >= 3);
    assert!(paged.training_count >= 1);
    assert!(paged.total_samples >= 6); // 3 * 2

    // 筛选：keyword + status
    let filtered = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(SpeakerFilter {
                keyword: Some("Training".to_string()),
                status: Some(SpeakerStatus::Training),
            }),
        })
        .await?;
    assert!(filtered.items.iter().all(|s| s.name.contains("Training")));
    assert_eq!(filtered.items.len(), 1);

    harness.shutdown().await
}

#[tokio::test]
async fn import_model_as_speaker_copies_into_managed_dir() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-import").await?;

    // 准备一个临时「外部模型目录」作为导入源
    let source_dir = harness.app_dir().join("external-model");
    fs::create_dir_all(source_dir.join("sub"))?;
    fs::write(source_dir.join("weights.bin"), b"fake-weights")?;
    fs::write(source_dir.join("sub").join("config.json"), b"{}")?;

    let imported = harness
        .service()
        .import_model_as_speaker(ImportModelAsSpeakerPayload {
            base_model: "qwen3_tts".to_string(),
            model_version: "1.7B".to_string(),
            source_model_dir_path: source_dir.to_string_lossy().to_string(),
            name: "Imported".to_string(),
            description: "from external dir".to_string(),
        })
        .await?;

    assert!(imported.id > 0);
    assert_eq!(imported.source, SpeakerSource::Local);
    assert_eq!(imported.status, SpeakerStatus::Ready);
    assert_eq!(imported.base_model, "qwen3_tts");

    // 产物落到受管理目录 model_dir/<id>，且递归复制了源内容
    let managed = harness.model_dir().join(imported.id.to_string());
    assert!(managed.is_dir());
    assert!(managed.join("weights.bin").exists());
    assert!(managed.join("sub").join("config.json").exists());

    harness.shutdown().await
}

#[tokio::test]
async fn update_speaker_info_changes_name_and_description() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-update").await?;
    let created = harness.create_test_speaker().await?;

    let updated = harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: created.id,
            name: "Renamed".to_string(),
            description: "new desc".to_string(),
        })
        .await?;

    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.description, "new desc");
    // status / source / samples / base_model 保留
    assert_eq!(updated.status, SpeakerStatus::Ready);
    assert_eq!(updated.samples, 3);

    harness.shutdown().await
}

#[tokio::test]
async fn delete_speaker_info_soft_deletes_and_hides_from_list() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-delete").await?;
    let created = harness.create_test_speaker().await?;

    let deleted = harness.delete_speaker(created.id).await?;
    assert!(deleted);

    let after = harness.list_speakers().await?;
    assert!(after.iter().all(|s| s.id != created.id));

    // 重复删除已软删的行 → 返回 false
    let again = harness.delete_speaker(created.id).await?;
    assert!(!again);

    harness.shutdown().await
}
