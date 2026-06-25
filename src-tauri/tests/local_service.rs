use kirine_client_lib::{
    test_support::{AppLanguage, LocalServiceHarness, PageRequest, SpeakerFilter, SpeakerStatus},
    Result,
};

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

    // 新增：status=Ready、samples=3
    let created = harness.create_test_speaker().await?;
    assert!(created.id > 0);

    // 分页 + 统计：列表应包含新建项，统计卡字段应反映本次新增
    let paged = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 10,
            filter: None,
        })
        .await?;
    assert_eq!(paged.page, 1);
    assert_eq!(paged.page_size, 10);
    assert!(paged.total >= 1);
    assert!(paged.items.iter().any(|speaker| speaker.id == created.id));
    assert!(paged.ready_count >= 1);
    assert!(paged.total_samples >= 3);
    assert_eq!(paged.total_pages, ((paged.total as u32) + 10 - 1) / 10);

    // 翻页：page_size=1 时第 1 页只返回 1 条，total_pages 应等于全量总数
    let first_page = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 1,
            filter: None,
        })
        .await?;
    assert_eq!(first_page.items.len(), 1);
    assert_eq!(first_page.total, paged.total);
    assert_eq!(first_page.total_pages, paged.total as u32);

    // 筛选：按关键字 + 状态过滤，结果应只含匹配项且仍能命中新建说话人
    let filtered = harness
        .list_speakers_paged(PageRequest {
            page: 1,
            page_size: 10,
            filter: Some(SpeakerFilter {
                keyword: Some("SeaOrm".to_string()),
                status: Some(SpeakerStatus::Ready),
                language: Some(AppLanguage::Chinese),
            }),
        })
        .await?;
    assert!(filtered
        .items
        .iter()
        .all(|speaker| speaker.name.contains("SeaOrm")));
    assert!(filtered
        .items
        .iter()
        .any(|speaker| speaker.id == created.id));

    // 更新
    let updated = harness.update_test_speaker(created.id).await?;
    assert_eq!(updated.name, "Updated Speaker");
    assert_eq!(updated.description, "updated by test");

    // 删除
    let deleted = harness.delete_speaker(created.id).await?;
    assert!(deleted);

    // 删除后列表不再包含该项
    let after_delete = harness.list_speakers_paged(PageRequest::default()).await?;
    assert!(after_delete
        .items
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
