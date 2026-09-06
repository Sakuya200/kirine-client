//! 流式说话人热更新：
//! - `validate_streaming_speakers` 各拒绝路径（非空、重名、trained 约束、参考音频约束）
//! - `validate_streaming_speakers_hot_update` 运行中 trained 约束（加/换拒绝、删放行）
//! - `update_streaming_speakers_impl` 持久化集成测试（context.json RMW、头像 ingest、
//!   进程不在时 `applied_to_session=false`、运行中通道失败不回滚落盘）

use kirine_client_lib::test_support::models::{HistoryTaskType, StreamingSpeakerInput, TaskStatus};
use kirine_client_lib::test_support::{
    ingest_streaming_speaker_avatars, validate_streaming_speakers,
    validate_streaming_speakers_hot_update, LocalServiceHarness, StreamingSpeaker,
};
use serde_json::json;

// ---- 测试数据构造 ----

fn voice_clone(name: &str, ref_audio: &str) -> StreamingSpeakerInput {
    StreamingSpeakerInput {
        name: name.to_string(),
        base_model: "moss_tts_realtime".to_string(),
        model_version: None,
        ref_audio_path: ref_audio.to_string(),
        ref_audio_name: ref_audio.to_string(),
        ref_text: String::new(),
        description: None,
        speaker_dir_name: None,
        category: "voice-clone".to_string(),
        side: "right".to_string(),
        avatar_path: None,
        avatar_name: None,
    }
}

fn trained(name: &str, dir: &str) -> StreamingSpeakerInput {
    StreamingSpeakerInput {
        name: name.to_string(),
        base_model: "moss_tts_realtime".to_string(),
        model_version: None,
        ref_audio_path: String::new(),
        ref_audio_name: String::new(),
        ref_text: String::new(),
        description: None,
        speaker_dir_name: Some(dir.to_string()),
        category: "trained".to_string(),
        side: "right".to_string(),
        avatar_path: None,
        avatar_name: None,
    }
}

fn old_speaker(name: &str, category: &str, dir: Option<&str>) -> StreamingSpeaker {
    serde_json::from_value(json!({
        "id": name,
        "name": name,
        "baseModel": "moss_tts_realtime",
        "refAudioPath": "",
        "refAudioName": "",
        "refText": "",
        "category": category,
        "speakerDirName": dir,
    }))
    .expect("streaming speaker")
}

// ---- validate_streaming_speakers ----

#[test]
fn validate_rejects_empty_speakers() {
    let err = validate_streaming_speakers(&[]).expect_err("empty must fail");
    assert!(err.to_string().contains("至少需要一个说话人"));
}

#[test]
fn validate_rejects_blank_name() {
    let err = validate_streaming_speakers(&[voice_clone("  ", "a.wav")]).expect_err("blank name");
    assert!(err.to_string().contains("名称不能为空"));
}

#[test]
fn validate_rejects_duplicate_names() {
    let err = validate_streaming_speakers(&[voice_clone("A", "a.wav"), voice_clone("A", "b.wav")])
        .expect_err("duplicate name");
    assert!(err.to_string().contains("重复"));
}

#[test]
fn validate_rejects_trained_without_dir() {
    let err = validate_streaming_speakers(&[trained("T", "")]).expect_err("no dir");
    assert!(err.to_string().contains("speakerDirName"));
}

#[test]
fn validate_rejects_multiple_trained() {
    let err = validate_streaming_speakers(&[trained("T1", "spk-1"), trained("T2", "spk-2")])
        .expect_err("two trained");
    assert!(err.to_string().contains("至多支持一个已训练说话人"));
}

#[test]
fn validate_rejects_voice_clone_without_ref_audio() {
    let err = validate_streaming_speakers(&[voice_clone("A", "  ")]).expect_err("no ref audio");
    assert!(err.to_string().contains("参考音频"));
}

#[test]
fn validate_accepts_single_voice_clone() {
    validate_streaming_speakers(&[voice_clone("A", "a.wav")]).expect("valid voice-clone");
}

#[test]
fn validate_accepts_one_trained_with_voice_clones() {
    validate_streaming_speakers(&[voice_clone("A", "a.wav"), trained("T", "spk-1")])
        .expect("valid mix");
}

// ---- validate_streaming_speakers_hot_update ----

#[test]
fn hot_update_keeps_existing_trained() {
    let old = [old_speaker("T", "trained", Some("spk-1"))];
    let new = [trained("T", "spk-1")];
    validate_streaming_speakers_hot_update(&old, &new).expect("kept trained passes");
}

#[test]
fn hot_update_rejects_new_trained() {
    let old = [];
    let new = [trained("T", "spk-9")];
    let err = validate_streaming_speakers_hot_update(&old, &new).expect_err("new trained");
    assert!(err.to_string().contains("不支持新增或更换已训练说话人"));
}

#[test]
fn hot_update_rejects_swapped_trained_dir() {
    let old = [old_speaker("T", "trained", Some("spk-1"))];
    let new = [trained("T", "spk-2")];
    let err = validate_streaming_speakers_hot_update(&old, &new).expect_err("swapped dir");
    assert!(err.to_string().contains("不支持新增或更换已训练说话人"));
}

#[test]
fn hot_update_allows_removing_trained() {
    let old = [old_speaker("T", "trained", Some("spk-1"))];
    let new = [voice_clone("A", "a.wav")];
    validate_streaming_speakers_hot_update(&old, &new).expect("removing trained passes");
}

#[test]
fn hot_update_rejects_trained_without_dir() {
    let old = [];
    let new = [trained("T", "")];
    let err = validate_streaming_speakers_hot_update(&old, &new).expect_err("no dir");
    assert!(err.to_string().contains("speakerDirName"));
}

#[test]
fn hot_update_ignores_voice_clone_changes() {
    let old = [old_speaker("A", "voice-clone", None), old_speaker("T", "trained", Some("spk-1"))];
    let new = [voice_clone("B", "b.wav"), trained("T", "spk-1")];
    validate_streaming_speakers_hot_update(&old, &new).expect("voice-clone churn passes");
}

// ---- update_streaming_speakers_impl 持久化 ----

/// 组装一个带 context.json 与 streaming_tasks 详情行的任务（status 用非 Running，
/// 避免与启动清扫语义混淆；impl 对 status 不敏感）。
async fn setup_task(harness: &LocalServiceHarness, history_id: i64) -> std::path::PathBuf {
    harness
        .seed_history(
            history_id,
            HistoryTaskType::StreamingSpeech,
            "流式语音（测试）",
            TaskStatus::Cancelled,
        )
        .await
        .expect("seed history");

    let sample_dir = harness.data_dir().join("tasks").join("streaming").join(format!("{history_id}"));
    std::fs::create_dir_all(&sample_dir).expect("create sample dir");
    let context_path = sample_dir.join("context.json");
    std::fs::write(
        &context_path,
        serde_json::to_vec_pretty(&json!({
            "basic": {
                "taskId": history_id,
                "baseModel": "moss_tts_realtime",
                "modelVersion": "v1",
                "device": "cpu",
                "language": "chinese",
                "speakers": [
                    {
                        "id": "A",
                        "name": "A",
                        "baseModel": "moss_tts_realtime",
                        "category": "voice-clone",
                        "refAudioPath": "a.wav",
                        "refAudioName": "a.wav",
                        "refText": "",
                        "side": "right"
                    },
                    {
                        "id": "C",
                        "name": "C",
                        "baseModel": "moss_tts_realtime",
                        "category": "voice-clone",
                        "refAudioPath": "c.wav",
                        "refAudioName": "c.wav",
                        "refText": "",
                        "side": "right",
                        "avatarPath": "%DATA_DIR_PATH%/tasks/streaming/1/avatar_3_C.png",
                        "avatarName": "c.png"
                    }
                ]
            },
            "messages": [
                {
                    "contextId": "msg-1",
                    "speakerName": "A",
                    "text": "你好",
                    "audioPath": "%DATA_DIR_PATH%/tasks/streaming/audio/msg-1.wav"
                }
            ]
        }))
        .expect("serialize context"),
    )
    .expect("write context.json");
    // C 的旧头像文件
    let avatar_file = harness.data_dir().join("tasks").join("streaming").join(format!("{history_id}")).join("avatar_3_C.png");
    std::fs::write(&avatar_file, b"old-avatar").expect("write avatar");

    harness
        .seed_streaming_detail(history_id, &format!("%DATA_DIR_PATH%/tasks/streaming/{history_id}/context.json"))
        .await
        .expect("seed streaming detail");
    context_path
}

#[tokio::test]
async fn update_streaming_speakers_persists_context_without_process() {
    let harness = LocalServiceHarness::new("streaming_speakers_persist")
        .await
        .expect("harness");
    let context_path = setup_task(&harness, 1).await;

    let result = harness
        .update_streaming_speakers(
            1,
            &[
                voice_clone("A", "a.wav"),
                voice_clone("B", "b.wav"), // 新增
            ], // C 被移除
        )
        .await
        .expect("update speakers");

    // 进程未注册：仅落盘
    assert!(!result.applied_to_session);

    let context: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&context_path).expect("read context"))
            .expect("parse context");
    let speakers = context["basic"]["speakers"].as_array().expect("speakers");
    assert_eq!(speakers.len(), 2);
    assert_eq!(speakers[0]["name"], json!("A"));
    assert_eq!(speakers[1]["name"], json!("B"));
    assert_eq!(speakers[1]["id"], json!("B"));

    // messages 未被触碰
    let messages = context["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["contextId"], json!("msg-1"));

    // 被移除说话人 C 的旧头像被清理
    let c_avatar = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("1")
        .join("avatar_3_C.png");
    assert!(!c_avatar.exists(), "removed speaker avatar should be deleted");

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn update_streaming_speakers_reports_session_send_failure_without_rollback() {
    let harness = LocalServiceHarness::new("streaming_speakers_send_fail")
        .await
        .expect("harness");
    let context_path = setup_task(&harness, 1).await;

    // 注册会话句柄但无真实进程：0x11 帧无接收端，发送失败 → applied=false，
    // 但 context.json 仍应更新（不回滚）。
    harness.register_streaming_session(1);

    let result = harness
        .update_streaming_speakers(1, &[voice_clone("A", "a.wav")])
        .await
        .expect("update speakers");

    assert!(!result.applied_to_session, "no receiver -> applied=false");

    let context: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&context_path).expect("read context"))
            .expect("parse context");
    let speakers = context["basic"]["speakers"].as_array().expect("speakers");
    assert_eq!(speakers.len(), 1);
    assert_eq!(speakers[0]["name"], json!("A"));

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn update_streaming_speakers_rejects_hot_trained_addition() {
    let harness = LocalServiceHarness::new("streaming_speakers_hot_trained")
        .await
        .expect("harness");
    let context_path = setup_task(&harness, 1).await;
    harness.register_streaming_session(1);

    let err = harness
        .update_streaming_speakers(1, &[voice_clone("A", "a.wav"), trained("T", "spk-9")])
        .await
        .expect_err("new trained must be rejected");
    assert!(err.to_string().contains("不支持新增或更换已训练说话人"));

    // 拒绝时 context.json 不应被改写
    let context: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&context_path).expect("read context"))
            .expect("parse context");
    let speakers = context["basic"]["speakers"].as_array().expect("speakers");
    assert_eq!(speakers.len(), 2, "context.json must stay untouched on rejection");

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn update_streaming_speakers_rejects_unknown_task_type() {
    let harness = LocalServiceHarness::new("streaming_speakers_wrong_type")
        .await
        .expect("harness");
    harness
        .seed_history(1, HistoryTaskType::TextToSpeech, "TTS", TaskStatus::Completed)
        .await
        .expect("seed tts");
    let err = harness
        .update_streaming_speakers(1, &[voice_clone("A", "a.wav")])
        .await
        .expect_err("non-streaming task");
    assert!(err.to_string().contains("不是流式语音会话"));

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn update_streaming_speakers_validates_input() {
    let harness = LocalServiceHarness::new("streaming_speakers_validate")
        .await
        .expect("harness");
    let _ = setup_task(&harness, 1).await;

    // 空列表
    let err = harness
        .update_streaming_speakers(1, &[])
        .await
        .expect_err("empty speakers");
    assert!(err.to_string().contains("至少需要一个说话人"));

    // 重名
    let err = harness
        .update_streaming_speakers(1, &[voice_clone("A", "a.wav"), voice_clone("A", "b.wav")])
        .await
        .expect_err("duplicate names");
    assert!(err.to_string().contains("重复"));

    harness.shutdown().await.expect("shutdown");
}

// ---- ingest_streaming_speaker_avatars（创建会话时的头像复制） ----

/// 带序列化头像路径的说话人（模拟新会话复用历史会话说话人：avatarPath 是
/// `%DATA_DIR_PATH%` 占位路径）。
fn with_serialized_avatar(mut speaker: StreamingSpeakerInput, serialized: &str) -> StreamingSpeakerInput {
    speaker.avatar_path = Some(serialized.to_string());
    speaker.avatar_name = Some("花花.png".to_string());
    speaker
}

#[tokio::test]
async fn ingest_avatars_resolves_serialized_path_and_copies_into_sample_dir() {
    let harness = LocalServiceHarness::new("streaming_avatars_ingest_copy").await.expect("harness");

    // 旧任务 sample 目录中的头像文件（跨任务复用场景）
    let old_sample = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("77");
    std::fs::create_dir_all(&old_sample).expect("create old sample dir");
    let old_avatar = old_sample.join("avatar_1_花花.png");
    std::fs::write(&old_avatar, b"old-avatar").expect("write old avatar");

    // 新任务 sample 目录
    let new_sample = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("78");
    std::fs::create_dir_all(&new_sample).expect("create new sample dir");

    let paths = ingest_streaming_speaker_avatars(
        harness.data_dir(),
        &new_sample,
        &[with_serialized_avatar(
            voice_clone("花花", "a.wav"),
            "%DATA_DIR_PATH%/tasks/streaming/77/avatar_1_花花.png",
        )],
    )
    .expect("ingest avatars");

    assert_eq!(
        paths,
        vec![Some("%DATA_DIR_PATH%/tasks/streaming/78/avatar_1_花花.png".to_string())]
    );
    let copied = new_sample.join("avatar_1_花花.png");
    assert_eq!(
        std::fs::read(&copied).expect("read copied avatar"),
        b"old-avatar",
        "avatar must be copied into the new task's sample dir"
    );

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn ingest_avatars_skips_missing_serialized_source() {
    let harness = LocalServiceHarness::new("streaming_avatars_ingest_missing").await.expect("harness");

    let new_sample = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("78");
    std::fs::create_dir_all(&new_sample).expect("create sample dir");

    // 序列化路径源文件不存在（如原任务被删除）：跳过头像，不阻断会话创建
    let paths = ingest_streaming_speaker_avatars(
        harness.data_dir(),
        &new_sample,
        &[with_serialized_avatar(
            voice_clone("花花", "a.wav"),
            "%DATA_DIR_PATH%/tasks/streaming/99/avatar_1_花花.png",
        )],
    )
    .expect("missing serialized source must not fail");

    assert_eq!(paths, vec![None]);
    assert!(
        !new_sample.join("avatar_1_花花.png").exists(),
        "no target file should be written when the source is missing"
    );

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn ingest_avatars_rejects_non_image_extension() {
    let harness = LocalServiceHarness::new("streaming_avatars_ingest_ext").await.expect("harness");

    let sample_dir = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("78");
    std::fs::create_dir_all(&sample_dir).expect("create sample dir");

    let err = ingest_streaming_speaker_avatars(
        harness.data_dir(),
        &sample_dir,
        &[with_serialized_avatar(
            voice_clone("花花", "a.wav"),
            "%DATA_DIR_PATH%/tasks/streaming/77/avatar_1_花花.txt",
        )],
    )
    .expect_err("non-image extension");
    assert!(err.to_string().contains("仅支持"));

    harness.shutdown().await.expect("shutdown");
}

#[tokio::test]
async fn ingest_avatars_returns_none_when_unset() {
    let harness = LocalServiceHarness::new("streaming_avatars_ingest_unset").await.expect("harness");

    let sample_dir = harness
        .data_dir()
        .join("tasks")
        .join("streaming")
        .join("78");
    std::fs::create_dir_all(&sample_dir).expect("create sample dir");

    let paths = ingest_streaming_speaker_avatars(
        harness.data_dir(),
        &sample_dir,
        &[voice_clone("A", "a.wav")],
    )
    .expect("ingest avatars without avatar config");

    assert_eq!(paths, vec![None]);

    harness.shutdown().await.expect("shutdown");
}
