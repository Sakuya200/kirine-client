//! 流式音频事件协议序列化测试：
//! - 协议序列化（AudioStreamEvent serde tag/camelCase）
//!
//! 三个 `#[tauri::command]` 的 Channel emit 链路需 Tauri IPC 上下文，不在单测范围；
//! 正弦波占位工具（generate_sine_wave_wav / build_sine_wave_stream_events）已随
//! `stream_audio_placeholder` 命令一并移除，相关 WAV/事件序列测试同步删除。

use kirine_client_lib::test_support::AudioStreamEvent;

// ---- 协议序列化 ----

#[test]
fn audio_stream_event_serializes_with_type_tag_and_camel_case() {
    let started = serde_json::to_value(&AudioStreamEvent::Started).unwrap();
    assert_eq!(started, serde_json::json!({"type":"started"}));

    let chunk = serde_json::to_value(&AudioStreamEvent::Chunk { bytes: vec![0, 1] }).unwrap();
    assert_eq!(chunk, serde_json::json!({"type":"chunk","bytes":[0,1]}));

    let finished = serde_json::to_value(&AudioStreamEvent::Finished).unwrap();
    assert_eq!(finished, serde_json::json!({"type":"finished"}));

    let error = serde_json::to_value(&AudioStreamEvent::Error {
        message: "x".into(),
    })
    .unwrap();
    assert_eq!(error, serde_json::json!({"type":"error","message":"x"}));
}
