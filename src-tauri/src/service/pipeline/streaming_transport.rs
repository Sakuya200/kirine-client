//! 流式会话环回 Socket 帧协议编解码（纯函数，供 runner 与集成测试共用）。
//!
//! 帧格式：`[u32 LE payload_len][u8 kind][payload]`（`payload_len` 含 kind 字节）。
//! - `0x01` auth：Python->Rust，JSON `{"token": "..."}`，连接建立后的首帧鉴权。
//! - `0x02` control：Python->Rust，JSON 帧（started/session_ready/finished/error），
//!   低频小体积，保留 JSON 可读性。
//! - `0x03` chunk：Python->Rust，`[u16 LE contextIdLen][contextId UTF-8][音频裸字节]`
//!   （首 chunk 含 44 字节 WAV 哨兵头），无 base64、无 JSON。
//! - `0x10` input：Rust->Python，JSON 消息条目（contextId/speakerName/text/audioPath）。
//! - `0x11` speakers_update：Rust->Python，JSON `{"speakers": [...]}` 全量说话人快照
//!   （字段同 `StreamingSpeakerInput`，camelCase）。运行中说话人热更新：Python 收到后
//!   重建说话人表并增量编码 voice-clone prompt。

use anyhow::{bail, Context};

use crate::Result;

pub const FRAME_KIND_AUTH: u8 = 0x01;
pub const FRAME_KIND_CONTROL: u8 = 0x02;
pub const FRAME_KIND_CHUNK: u8 = 0x03;
pub const FRAME_KIND_INPUT: u8 = 0x10;
pub const FRAME_KIND_SPEAKERS_UPDATE: u8 = 0x11;

/// 单帧 payload 上限：防止长度前缀损坏时巨量分配。音频 chunk 为百 KB 级，16MB 已远超所需。
pub const MAX_FRAME_PAYLOAD: usize = 16 * 1024 * 1024;

/// 帧头（长度前缀 + kind）的固定字节数。
pub const FRAME_HEADER_LEN: usize = 5;

/// 编码一帧：`[u32 LE len(kind+payload)][kind][payload]`。
pub fn encode_frame(kind: u8, payload: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
    buf.extend_from_slice(&((payload.len() + 1) as u32).to_le_bytes());
    buf.push(kind);
    buf.extend_from_slice(payload);
    buf
}

/// 编码控制帧（payload 为 JSON 文本）。
pub fn encode_control_frame(json: &str) -> Vec<u8> {
    encode_frame(FRAME_KIND_CONTROL, json.as_bytes())
}

/// 编码 input 帧（Rust->Python 消息条目，payload 为 JSON 文本）。
pub fn encode_input_frame(json: &str) -> Vec<u8> {
    encode_frame(FRAME_KIND_INPUT, json.as_bytes())
}

/// 编码 speakers_update 帧（Rust->Python 全量说话人快照，payload 为 JSON 文本）。
pub fn encode_speakers_update_frame(json: &str) -> Vec<u8> {
    encode_frame(FRAME_KIND_SPEAKERS_UPDATE, json.as_bytes())
}

/// 编码 chunk 帧：contextId 前置长度前缀，后接音频裸字节。
pub fn encode_chunk_frame(context_id: &str, bytes: &[u8]) -> Result<Vec<u8>> {
    let id = context_id.as_bytes();
    if id.len() > u16::MAX as usize {
        bail!("contextId 超长（{} 字节，上限 65535）", id.len());
    }
    let mut payload = Vec::with_capacity(2 + id.len() + bytes.len());
    payload.extend_from_slice(&(id.len() as u16).to_le_bytes());
    payload.extend_from_slice(id);
    payload.extend_from_slice(bytes);
    Ok(encode_frame(FRAME_KIND_CHUNK, &payload))
}

/// 校验帧头长度前缀：返回 `(kind, payload_len)`；超限或为零时报错。
pub fn decode_frame_header(header: &[u8]) -> Result<(u8, usize)> {
    if header.len() != FRAME_HEADER_LEN {
        bail!("帧头长度错误：期望 5 字节，实际 {}", header.len());
    }
    let total = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
    if total == 0 {
        bail!("帧长度前缀为零");
    }
    if total > MAX_FRAME_PAYLOAD {
        bail!("帧长度超限：{total} > {MAX_FRAME_PAYLOAD}");
    }
    Ok((header[4], total - 1))
}

/// 解析 chunk 帧 payload（不含 kind 字节）：`[u16 LE contextIdLen][contextId][音频字节]`。
pub fn parse_chunk_payload(payload: &[u8]) -> Result<(&str, &[u8])> {
    if payload.len() < 2 {
        bail!("chunk payload 过短：{} 字节", payload.len());
    }
    let id_len = u16::from_le_bytes([payload[0], payload[1]]) as usize;
    let rest = &payload[2..];
    if rest.len() < id_len {
        bail!(
            "chunk contextId 长度不匹配：声明 {id_len} 字节，实际仅 {} 字节",
            rest.len()
        );
    }
    let context_id = std::str::from_utf8(&rest[..id_len])
        .context("chunk contextId 不是合法 UTF-8")?;
    Ok((context_id, &rest[id_len..]))
}

/// 解析 auth 帧 payload（不含 kind 字节）：JSON `{"token": "..."}`。
pub fn parse_auth_payload(payload: &[u8]) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct AuthFrame {
        #[serde(default)]
        token: String,
    }
    let text = std::str::from_utf8(payload).context("auth payload 不是合法 UTF-8")?;
    let frame: AuthFrame =
        serde_json::from_str(text).context("failed to parse auth frame")?;
    Ok(frame.token)
}

/// 生成随机会话令牌（32 字节随机数的 hex），供 Python 连接后回传鉴权。
pub fn generate_session_token() -> Result<String> {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}
