//! 流式控制帧解析与事件映射的纯函数单测。
//!
//! 原位于 `src/service/pipeline/streaming.rs` 内联 `#[cfg(test)]` 模块；被测项
//! （`parse_streaming_frame` / `frame_to_event` / `serialize_input_entry` 及若干
//! `StreamingContext*` 结构）为 `pub(crate)`，经 `test_support` 桥接重导出后迁入本文件。
//! chunk 不走 JSON 控制帧（二进制帧编解码见 `streaming_transport.rs` 测试）。

use kirine_client_lib::test_support::{
    frame_to_event, parse_streaming_frame, serialize_input_entry, StreamingContextBasic,
    StreamingContextJson, StreamingFramePayload, StreamingMessageEntry, StreamingSpeaker,
};
use tauri::ipc::InvokeResponseBody;

#[test]
fn parses_started_frame() {
    let frame = parse_streaming_frame(r#"{"type":"started","contextId":"msg-1"}"#)
        .expect("parse")
        .expect("some");
    assert_eq!(frame.context_id, "msg-1");
    assert_eq!(frame.payload, StreamingFramePayload::Started);
}

#[test]
fn parses_finished_and_error_frames() {
    let fin = parse_streaming_frame(r#"{"type":"finished","contextId":"msg-1"}"#)
        .expect("parse")
        .expect("some");
    assert_eq!(fin.payload, StreamingFramePayload::Finished);

    let err = parse_streaming_frame(r#"{"type":"error","contextId":"msg-1","message":"boom"}"#)
        .expect("parse")
        .expect("some");
    assert_eq!(
        err.payload,
        StreamingFramePayload::Error {
            message: "boom".to_string()
        }
    );
}

#[test]
fn parses_session_ready_frame() {
    let frame = parse_streaming_frame(r#"{"type":"session_ready","contextId":"__session__"}"#)
        .expect("parse")
        .expect("some");
    assert_eq!(frame.context_id, "__session__");
    assert_eq!(frame.payload, StreamingFramePayload::SessionReady);
}

#[test]
fn session_ready_frame_is_not_forwarded_as_audio_event() {
    let frame = parse_streaming_frame(r#"{"type":"session_ready","contextId":"__session__"}"#)
        .expect("parse")
        .expect("some");
    assert!(frame_to_event(&frame).is_none());
}

#[test]
fn blank_line_yields_none() {
    assert!(parse_streaming_frame("   ").expect("parse").is_none());
}

#[test]
fn malformed_line_is_err() {
    assert!(parse_streaming_frame("not json").is_err());
}

#[test]
fn context_json_round_trips() {
    let ctx = StreamingContextJson {
        basic: StreamingContextBasic {
            task_id: 7,
            base_model: "gpt_sovits_cpufast".to_string(),
            model_version: "v1".to_string(),
            device: "cpu".to_string(),
            language: "chinese".to_string(),
            speakers: vec![StreamingSpeaker {
                id: "spk-1".to_string(),
                name: "A".to_string(),
                base_model: "gpt_sovits_cpufast".to_string(),
                model_version: None,
                ref_audio_path: "/ref.wav".to_string(),
                ref_audio_name: "ref.wav".to_string(),
                ref_text: "参考".to_string(),
                description: None,
                speaker_dir_name: None,
                category: String::new(),
                side: "left".to_string(),
                avatar_path: Some(
                    "%DATA_DIR_PATH%/samples/streaming-speech_7/avatar_1_A.png".to_string(),
                ),
                avatar_name: Some("A.png".to_string()),
            }],
        },
        messages: vec![StreamingMessageEntry {
            context_id: "msg-2".to_string(),
            speaker_name: "A".to_string(),
            text: "你好".to_string(),
            audio_path: "%DATA_DIR_PATH%/streaming_7/audio/msg-2.wav".to_string(),
        }],
    };
    let json = serde_json::to_string(&ctx).expect("serialize");
    let back: StreamingContextJson = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.basic.task_id, 7);
    assert_eq!(back.basic.speakers[0].ref_audio_path, "/ref.wav");
    assert_eq!(back.messages[0].context_id, "msg-2");
    // camelCase 字段名
    assert!(json.contains("\"taskId\""));
    assert!(json.contains("\"contextId\""));
    assert!(json.contains("\"audioPath\""));
}

#[test]
fn input_entry_serializes_single_line() {
    let line = serialize_input_entry("msg-2", "A", "你好", "/p/a.wav");
    assert!(!line.contains('\n'));
    let v: serde_json::Value = serde_json::from_str(&line).expect("parse");
    assert_eq!(v["contextId"], "msg-2");
    assert_eq!(v["text"], "你好");
}

#[test]
fn input_entry_serializes_camel_case_fields_for_python_reader() {
    let line = serialize_input_entry("msg-2", "A", "你好", "/p/a.wav");
    let v: serde_json::Value = serde_json::from_str(&line).expect("parse");
    assert_eq!(v["contextId"], "msg-2");
    assert_eq!(v["speakerName"], "A");
    assert_eq!(v["text"], "你好");
    assert_eq!(v["audioPath"], "/p/a.wav");
    assert!(v.get("context_id").is_none());
    assert!(v.get("speaker_name").is_none());
}

#[test]
fn unknown_frame_type_is_err() {
    let result = parse_streaming_frame(r#"{"type":"bogus","contextId":"msg-1"}"#);
    assert!(result.is_err());
}

#[test]
fn frame_to_event_maps_control_payloads_to_json() {
    let started = parse_streaming_frame(r#"{"type":"started","contextId":"msg-1"}"#)
        .expect("parse")
        .expect("some");
    match frame_to_event(&started) {
        Some(InvokeResponseBody::Json(json)) => {
            assert_eq!(json, r#"{"type":"started"}"#);
        }
        other => panic!("expected Json, got {other:?}"),
    }

    let finished = parse_streaming_frame(r#"{"type":"finished","contextId":"msg-1"}"#)
        .expect("parse")
        .expect("some");
    match frame_to_event(&finished) {
        Some(InvokeResponseBody::Json(json)) => {
            assert_eq!(json, r#"{"type":"finished"}"#);
        }
        other => panic!("expected Json, got {other:?}"),
    }

    let error = parse_streaming_frame(r#"{"type":"error","contextId":"msg-1","message":"boom"}"#)
        .expect("parse")
        .expect("some");
    match frame_to_event(&error) {
        Some(InvokeResponseBody::Json(json)) => {
            assert!(json.contains(r#""type":"error""#));
            assert!(json.contains("boom"));
        }
        other => panic!("expected Json, got {other:?}"),
    }
}

#[test]
fn frame_to_event_maps_chunk_payload_to_raw_bytes() {
    let frame = kirine_client_lib::test_support::StreamingFrame {
        context_id: "msg-1".to_string(),
        payload: StreamingFramePayload::Chunk {
            bytes: vec![b'h', b'i'],
        },
    };
    match frame_to_event(&frame) {
        Some(InvokeResponseBody::Raw(bytes)) => assert_eq!(bytes, vec![b'h', b'i']),
        other => panic!("expected Raw, got {other:?}"),
    }
}
