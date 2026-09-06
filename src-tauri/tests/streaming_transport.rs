//! 流式会话环回 Socket 帧协议编解码单测（纯函数，对齐 `streaming_transport.rs`）：
//! 长度前缀分帧、chunk 二进制 payload（contextId 长度前缀 + 裸字节）、auth/控制帧 JSON、
//! 令牌生成。

use kirine_client_lib::test_support::{
    decode_frame_header, encode_chunk_frame, encode_control_frame, encode_frame,
    encode_input_frame, encode_speakers_update_frame, generate_session_token, parse_auth_payload,
    parse_chunk_payload, FRAME_HEADER_LEN, FRAME_KIND_AUTH, FRAME_KIND_CONTROL, FRAME_KIND_CHUNK,
    FRAME_KIND_INPUT, FRAME_KIND_SPEAKERS_UPDATE,
};

#[test]
fn encode_frame_writes_length_prefix_and_kind() {
    let frame = encode_frame(FRAME_KIND_CONTROL, b"abc");
    assert_eq!(frame.len(), FRAME_HEADER_LEN + 3);
    // u32 LE 长度（含 kind 字节）
    assert_eq!(&frame[..4], &[4, 0, 0, 0]);
    assert_eq!(frame[4], FRAME_KIND_CONTROL);
    assert_eq!(&frame[5..], b"abc");
}

#[test]
fn frame_header_round_trips() {
    let frame = encode_input_frame(r#"{"contextId":"m1"}"#);
    let (kind, payload_len) = decode_frame_header(&frame[..FRAME_HEADER_LEN]).expect("header");
    assert_eq!(kind, FRAME_KIND_INPUT);
    assert_eq!(payload_len, frame.len() - FRAME_HEADER_LEN);
    assert_eq!(&frame[FRAME_HEADER_LEN..], br#"{"contextId":"m1"}"#);
}

#[test]
fn zero_length_header_is_rejected() {
    assert!(decode_frame_header(&[0, 0, 0, 0, FRAME_KIND_CONTROL]).is_err());
}

#[test]
fn oversize_length_is_rejected() {
    let header = (u32::MAX).to_le_bytes();
    let mut header = header.to_vec();
    header.push(FRAME_KIND_CHUNK);
    assert!(decode_frame_header(&header).is_err());
}

#[test]
fn wrong_header_length_is_rejected() {
    assert!(decode_frame_header(&[1, 2, 3]).is_err());
}

#[test]
fn chunk_frame_round_trips_context_id_and_bytes() {
    let bytes = vec![0x52, 0x49, 0x46, 0x46]; // 模拟 PCM 片段
    let frame = encode_chunk_frame("msg-7", &bytes).expect("encode");
    let (kind, payload_len) = decode_frame_header(&frame[..FRAME_HEADER_LEN]).expect("header");
    assert_eq!(kind, FRAME_KIND_CHUNK);
    let payload = &frame[FRAME_HEADER_LEN..];
    assert_eq!(payload.len(), payload_len);
    let (context_id, audio) = parse_chunk_payload(payload).expect("parse");
    assert_eq!(context_id, "msg-7");
    assert_eq!(audio, &bytes[..]);
}

#[test]
fn chunk_payload_with_multibyte_context_id_round_trips() {
    let frame = encode_chunk_frame("消息-1", &[1, 2, 3]).expect("encode");
    let (context_id, audio) = parse_chunk_payload(&frame[FRAME_HEADER_LEN..]).expect("parse");
    assert_eq!(context_id, "消息-1");
    assert_eq!(audio, &[1, 2, 3]);
}

#[test]
fn truncated_chunk_payload_is_rejected() {
    // 声明 8 字节 contextId 但实际只有 3 字节
    let payload = [8u8, 0, b'a', b'b', b'c'];
    assert!(parse_chunk_payload(&payload).is_err());
    // 少于长度前缀本身
    assert!(parse_chunk_payload(&[0]).is_err());
}

#[test]
fn auth_payload_parses_token() {
    let frame = encode_frame(FRAME_KIND_AUTH, br#"{"token":"tok-123"}"#);
    let token = parse_auth_payload(&frame[FRAME_HEADER_LEN..]).expect("parse");
    assert_eq!(token, "tok-123");
}

#[test]
fn auth_payload_missing_token_is_empty() {
    let frame = encode_frame(FRAME_KIND_AUTH, b"{}");
    let token = parse_auth_payload(&frame[FRAME_HEADER_LEN..]).expect("parse");
    assert_eq!(token, "");
}

#[test]
fn malformed_auth_payload_is_err() {
    let frame = encode_frame(FRAME_KIND_AUTH, b"not json");
    assert!(parse_auth_payload(&frame[FRAME_HEADER_LEN..]).is_err());
}

#[test]
fn control_frame_carries_json_payload() {
    let json = r#"{"type":"started","contextId":"msg-1"}"#;
    let frame = encode_control_frame(json);
    let (kind, _) = decode_frame_header(&frame[..FRAME_HEADER_LEN]).expect("header");
    assert_eq!(kind, FRAME_KIND_CONTROL);
    assert_eq!(&frame[FRAME_HEADER_LEN..], json.as_bytes());
}

#[test]
fn speakers_update_frame_layout_matches_protocol() {
    // 0x11 帧：与 0x10 input 帧共用同一种帧头（u32 LE 长度 + kind 字节），
    // payload 为全量 speakers JSON 快照（Rust 不接收该帧，仅发送）。
    let json = r#"{"speakers":[{"name":"A","category":"voice-clone"}]}"#;
    let frame = encode_speakers_update_frame(json);
    let (kind, payload_len) = decode_frame_header(&frame[..FRAME_HEADER_LEN]).expect("header");
    assert_eq!(kind, FRAME_KIND_SPEAKERS_UPDATE);
    assert_eq!(payload_len, json.len());
    assert_eq!(&frame[FRAME_HEADER_LEN..], json.as_bytes());
    // 长度前缀含 kind 字节，与 Python 侧 encode_frame 对齐
    assert_eq!(&frame[..4], &((json.len() + 1) as u32).to_le_bytes());
}

#[test]
fn speakers_update_frame_round_trips_through_decode_frame_header() {
    let frame = encode_speakers_update_frame("{}");
    assert_ne!(FRAME_KIND_SPEAKERS_UPDATE, FRAME_KIND_INPUT);
    let (kind, payload_len) = decode_frame_header(&frame[..FRAME_HEADER_LEN]).expect("header");
    assert_eq!(kind, FRAME_KIND_SPEAKERS_UPDATE);
    assert_eq!(payload_len, 2);
}

#[test]
fn session_tokens_are_unique_and_hex() {
    let a = generate_session_token().expect("token");
    let b = generate_session_token().expect("token");
    assert_ne!(a, b);
    assert_eq!(a.len(), 64); // 32 字节 -> 64 个 hex 字符
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}
