use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::hooks::streaming::AudioStreamEvent;

/// 脚本 stdout 单帧（JSON 行）。`bytes` 为 base64。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamingFrame {
    pub context_id: String,
    pub payload: StreamingFramePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamingFramePayload {
    Started,
    Chunk { bytes: Vec<u8> },
    Finished,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StreamingContextJson {
    pub basic: StreamingContextBasic,
    pub messages: Vec<StreamingMessageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingContextBasic {
    pub task_id: i64,
    pub base_model: String,
    pub model_version: String,
    pub device: String,
    pub language: String,
    pub speakers: Vec<StreamingSpeaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingSpeaker {
    pub id: String,
    pub name: String,
    pub base_model: String,
    #[serde(default)]
    pub model_version: Option<String>,
    pub ref_audio_path: String,
    pub ref_audio_name: String,
    pub ref_text: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingMessageEntry {
    pub context_id: String,
    pub speaker_name: String,
    pub text: String,
    pub audio_path: String,
}

pub(crate) fn serialize_input_entry(
    context_id: &str,
    speaker_name: &str,
    text: &str,
    audio_path: &str,
) -> String {
    serde_json::to_string(&StreamingMessageEntry {
        context_id: context_id.to_string(),
        speaker_name: speaker_name.to_string(),
        text: text.to_string(),
        audio_path: audio_path.to_string(),
    })
    .unwrap_or_default()
}

#[derive(Deserialize)]
struct RawFrame {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "contextId")]
    context_id: String,
    #[serde(default)]
    bytes: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// 解析脚本 stdout 一行。空行返回 `Ok(None)`；坏行返回 `Err`。
pub(crate) fn parse_streaming_frame(line: &str) -> Result<Option<StreamingFrame>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let raw: RawFrame = serde_json::from_str(trimmed).context("failed to parse streaming frame")?;
    let payload = match raw.kind.as_str() {
        "started" => StreamingFramePayload::Started,
        "chunk" => {
            let bytes = base64_decode(raw.bytes.as_deref().unwrap_or(""))?;
            StreamingFramePayload::Chunk { bytes }
        }
        "finished" => StreamingFramePayload::Finished,
        "error" => StreamingFramePayload::Error {
            message: raw.message.unwrap_or_default(),
        },
        other => anyhow::bail!("unknown streaming frame type: {other}"),
    };
    Ok(Some(StreamingFrame { context_id: raw.context_id, payload }))
}

/// 将帧映射为下发前端的 `AudioStreamEvent`。
pub(crate) fn frame_to_event(frame: &StreamingFrame) -> Option<AudioStreamEvent> {
    match &frame.payload {
        StreamingFramePayload::Started => Some(AudioStreamEvent::Started),
        StreamingFramePayload::Chunk { bytes } => Some(AudioStreamEvent::Chunk { bytes: bytes.clone() }),
        StreamingFramePayload::Finished => Some(AudioStreamEvent::Finished),
        StreamingFramePayload::Error { message } => Some(AudioStreamEvent::Error { message: message.clone() }),
    }
}

fn base64_decode(value: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(value).context("failed to base64-decode chunk bytes")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_started_frame() {
        let frame = parse_streaming_frame(r#"{"type":"started","contextId":"msg-1"}"#)
            .expect("parse")
            .expect("some");
        assert_eq!(frame.context_id, "msg-1");
        assert_eq!(frame.payload, StreamingFramePayload::Started);
    }

    #[test]
    fn parses_chunk_frame_with_base64_bytes() {
        // "hi" -> base64 "aGk="
        let frame = parse_streaming_frame(
            r#"{"type":"chunk","contextId":"msg-1","bytes":"aGk="}"#,
        )
        .expect("parse")
        .expect("some");
        assert_eq!(frame.payload, StreamingFramePayload::Chunk { bytes: vec![b'h', b'i'] });
    }

    #[test]
    fn parses_finished_and_error_frames() {
        let fin = parse_streaming_frame(r#"{"type":"finished","contextId":"msg-1"}"#)
            .expect("parse")
            .expect("some");
        assert_eq!(fin.payload, StreamingFramePayload::Finished);

        let err = parse_streaming_frame(
            r#"{"type":"error","contextId":"msg-1","message":"boom"}"#,
        )
        .expect("parse")
        .expect("some");
        assert_eq!(
            err.payload,
            StreamingFramePayload::Error { message: "boom".to_string() }
        );
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
}
