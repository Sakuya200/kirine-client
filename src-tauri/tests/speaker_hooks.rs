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
            speaker_name: "Alice".to_string(),
            samples: 5,
            base_model: "qwen3_tts".to_string(),
            description: "test speaker".to_string(),
            status: SpeakerStatus::Ready,
            source: SpeakerSource::Local,
        })
        .await?;

    assert!(created.id > 0);
    assert_eq!(created.speaker_name, "Alice");
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
                speaker_name: format!("Ready-{i}"),
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
            speaker_name: "Training-One".to_string(),
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
    assert_eq!(paged.total_pages, ((paged.total as u32) + 2 - 1) / 2);
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
                base_model: None,
            }),
        })
        .await?;
    assert!(filtered
        .items
        .iter()
        .all(|s| s.speaker_name.contains("Training")));
    assert_eq!(filtered.items.len(), 1);

    harness.shutdown().await
}

#[tokio::test]
async fn list_speaker_infos_filters_by_base_model() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-base-model-filter").await?;

    // 两个 base_model 各造说话人：qwen3_tts 2 条 ready，firered_tts 1 条 ready
    for i in 0..2 {
        harness
            .service()
            .create_speaker_info(CreateSpeakerPayload {
                speaker_name: format!("Qwen-{i}"),
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
            speaker_name: "Fire-0".to_string(),
            samples: 1,
            base_model: "firered_tts".to_string(),
            description: "".to_string(),
            status: SpeakerStatus::Ready,
            source: SpeakerSource::Local,
        })
        .await?;

    // baseModel 过滤只返回对应条目，统计跟随筛选
    let filtered = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 100,
            filter: Some(SpeakerFilter {
                keyword: None,
                status: Some(SpeakerStatus::Ready),
                base_model: Some("firered_tts".to_string()),
            }),
        })
        .await?;
    assert_eq!(filtered.items.len(), 1);
    assert_eq!(filtered.items[0].speaker_name, "Fire-0");
    assert_eq!(filtered.items[0].base_model, "firered_tts");
    assert_eq!(filtered.ready_count, 1);
    assert_eq!(filtered.training_count, 0);
    assert_eq!(filtered.total_samples, 1);

    harness.shutdown().await
}

#[tokio::test]
async fn sync_speaker_definition_preserves_modify_time_when_unchanged() -> Result<()> {
    // 预置说话人 upsert 契约：内容无变化时不得刷新 modify_time
    // （否则每次应用启动都会把预置说话人顶到按 modify_time 排序的列表头部）
    let harness = LocalServiceHarness::new("speaker-preset-sync").await?;

    let preset = harness
        .list_speakers()
        .await?
        .into_iter()
        .find(|s| s.source == SpeakerSource::Preset)
        .expect("同步后应存在预置说话人");
    let original_description = preset.description.clone();

    // 内容无变化：再次 sync 不应改动行（modify_time 保持）
    harness.resync_supported_models().await?;
    let after_resync = harness
        .list_speakers()
        .await?
        .into_iter()
        .find(|s| s.id == preset.id)
        .expect("预置说话人应存在");
    assert_eq!(after_resync.modify_time, preset.modify_time);

    // 用户改动描述后再 sync：内容有差异，应被重写回预置定义
    harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: preset.id,
            speaker_name: preset.speaker_name.clone(),
            description: "user-edited".to_string(),
            avatar_source_path: None,
            remove_avatar: false,
        })
        .await?;
    harness.resync_supported_models().await?;
    let reverted = harness
        .list_speakers()
        .await?
        .into_iter()
        .find(|s| s.id == preset.id)
        .expect("预置说话人应存在");
    assert_ne!(reverted.description, "user-edited");
    assert_eq!(reverted.description, original_description);

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
            speaker_name: "Imported".to_string(),
            description: "from external dir".to_string(),
            avatar_source_path: None,
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
        .update_speaker_info(
            kirine_client_lib::test_support::models::UpdateSpeakerPayload {
                id: created.id,
                speaker_name: "Renamed".to_string(),
                description: "new desc".to_string(),
                avatar_source_path: None,
                remove_avatar: false,
            },
        )
        .await?;

    assert_eq!(updated.speaker_name, "Renamed");
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

    // 重复删除已软删的行 -> 返回 false
    let again = harness.delete_speaker(created.id).await?;
    assert!(!again);

    harness.shutdown().await
}

fn write_avatar_fixture(harness: &LocalServiceHarness, file_name: &str, bytes: &[u8]) -> std::path::PathBuf {
    let avatar_path = harness.app_dir().join(file_name);
    fs::write(&avatar_path, bytes).expect("写入头像测试文件失败");
    avatar_path
}

#[tokio::test]
async fn speaker_avatar_set_read_remove_lifecycle() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-avatar").await?;
    let created = harness.create_test_speaker().await?;

    // 未设置头像：列表 content type 为 None，读取报错（前端据此回退占位）
    let listed = harness.list_speakers().await?;
    let target = listed.iter().find(|s| s.id == created.id).unwrap();
    assert!(target.avatar_content_type.is_none());
    assert!(harness.service().get_speaker_avatar(created.id).await.is_err());

    // 设置头像：content type 按扩展名推断，字节可读回
    let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 1, 2, 3];
    let avatar_path = write_avatar_fixture(&harness, "avatar.png", &png_bytes);
    let updated = harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: created.id,
            speaker_name: created.speaker_name.clone(),
            description: created.description.clone(),
            avatar_source_path: Some(avatar_path.to_string_lossy().to_string()),
            remove_avatar: false,
        })
        .await?;
    assert_eq!(updated.avatar_content_type.as_deref(), Some("image/png"));

    let asset = harness.service().get_speaker_avatar(created.id).await?;
    assert_eq!(asset.speaker_id, created.id);
    assert_eq!(asset.content_type, "image/png");
    assert_eq!(asset.bytes, png_bytes);

    // 列表只携带 content type，不携带字节
    let listed = harness.list_speakers().await?;
    let target = listed.iter().find(|s| s.id == created.id).unwrap();
    assert_eq!(target.avatar_content_type.as_deref(), Some("image/png"));

    // remove_avatar 优先：清除后读取报错
    harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: created.id,
            speaker_name: created.speaker_name.clone(),
            description: created.description.clone(),
            avatar_source_path: None,
            remove_avatar: true,
        })
        .await?;
    assert!(harness.service().get_speaker_avatar(created.id).await.is_err());
    let listed = harness.list_speakers().await?;
    let target = listed.iter().find(|s| s.id == created.id).unwrap();
    assert!(target.avatar_content_type.is_none());

    harness.shutdown().await
}

#[tokio::test]
async fn speaker_avatar_oversize_file_rejected() -> Result<()> {
    let harness = LocalServiceHarness::new("speaker-avatar-oversize").await?;
    let created = harness.create_test_speaker().await?;

    // 超过 2MB 上限：拒绝写入，头像保持未设置
    let avatar_path = write_avatar_fixture(&harness, "avatar.png", &vec![0u8; 2 * 1024 * 1024 + 1]);
    let result = harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: created.id,
            speaker_name: created.speaker_name.clone(),
            description: created.description.clone(),
            avatar_source_path: Some(avatar_path.to_string_lossy().to_string()),
            remove_avatar: false,
        })
        .await;
    assert!(result.is_err());
    assert!(harness.service().get_speaker_avatar(created.id).await.is_err());

    harness.shutdown().await
}

#[tokio::test]
async fn sync_speaker_definition_preserves_avatar() -> Result<()> {
    // 预置说话人 upsert 更新分支只 Set 显式字段，用户设置的头像不应被 resync 清掉
    let harness = LocalServiceHarness::new("speaker-preset-avatar").await?;

    let preset = harness
        .list_speakers()
        .await?
        .into_iter()
        .find(|s| s.source == SpeakerSource::Preset)
        .expect("同步后应存在预置说话人");

    let png_bytes: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 9, 8, 7];
    let avatar_path = write_avatar_fixture(&harness, "preset-avatar.png", &png_bytes);
    harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: preset.id,
            speaker_name: preset.speaker_name.clone(),
            description: preset.description.clone(),
            avatar_source_path: Some(avatar_path.to_string_lossy().to_string()),
            remove_avatar: false,
        })
        .await?;

    // 用户改描述触发 upsert 更新分支（重写预置定义），头像应保留
    harness
        .service()
        .update_speaker_info(kirine_client_lib::test_support::models::UpdateSpeakerPayload {
            id: preset.id,
            speaker_name: preset.speaker_name.clone(),
            description: "user-edited".to_string(),
            avatar_source_path: None,
            remove_avatar: false,
        })
        .await?;
    harness.resync_supported_models().await?;

    let asset = harness.service().get_speaker_avatar(preset.id).await?;
    assert_eq!(asset.content_type, "image/png");
    assert_eq!(asset.bytes, png_bytes);

    harness.shutdown().await
}
