//! 流式音频占位 hook 的纯函数测试：
//! - WAV 格式正确性（generate_sine_wave_wav）
//! - 事件序列正确性（build_sine_wave_stream_events）
//! - 协议序列化（AudioStreamEvent serde tag/camelCase）
//!
//! 命令本身（stream_audio_placeholder 的 Channel emit 链路）需 Tauri IPC 上下文，
//! 不在单测范围；命令是 iterate+emit+sleep 的薄封装，正确性由纯函数保证。

use kirine_client_lib::test_support::{
    build_sine_wave_stream_events, generate_sine_wave_wav, AudioStreamEvent,
};

const DURATION: f32 = 2.0;
const FREQ: f32 = 440.0;
const SAMPLE_RATE: u32 = 44100;
const CHUNK_SAMPLE_COUNT: usize = 4096;

// ---- 测试组 A：WAV 格式正确性 ----

#[test]
fn sine_wave_wav_has_valid_header_magic() {
    let wav = generate_sine_wave_wav(DURATION, FREQ, SAMPLE_RATE);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(&wav[36..40], b"data");
}

#[test]
fn sine_wave_wav_header_fields_are_correct() {
    let wav = generate_sine_wave_wav(DURATION, FREQ, SAMPLE_RATE);
    let num_samples = (DURATION * SAMPLE_RATE as f32) as usize;
    let data_size = num_samples * 2;

    assert_eq!(
        u32::from_le_bytes(wav[4..8].try_into().unwrap()),
        36 + data_size as u32
    );
    assert_eq!(
        u32::from_le_bytes(wav[16..20].try_into().unwrap()),
        16
    );
    assert_eq!(
        u16::from_le_bytes(wav[20..22].try_into().unwrap()),
        1
    );
    assert_eq!(
        u16::from_le_bytes(wav[22..24].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(wav[24..28].try_into().unwrap()),
        SAMPLE_RATE
    );
    assert_eq!(
        u32::from_le_bytes(wav[28..32].try_into().unwrap()),
        SAMPLE_RATE * 2
    );
    assert_eq!(
        u16::from_le_bytes(wav[32..34].try_into().unwrap()),
        2
    );
    assert_eq!(
        u16::from_le_bytes(wav[34..36].try_into().unwrap()),
        16
    );
    assert_eq!(
        u32::from_le_bytes(wav[40..44].try_into().unwrap()),
        data_size as u32
    );
}

#[test]
fn sine_wave_wav_total_length_matches() {
    let wav = generate_sine_wave_wav(DURATION, FREQ, SAMPLE_RATE);
    let num_samples = (DURATION * SAMPLE_RATE as f32) as usize;
    assert_eq!(wav.len(), 44 + num_samples * 2);
}

// ---- 测试组 B：事件序列正确性 ----

#[test]
fn stream_events_start_and_finish_with_correct_variants() {
    let events = build_sine_wave_stream_events(DURATION, FREQ, SAMPLE_RATE, CHUNK_SAMPLE_COUNT);
    assert!(matches!(events.first(), Some(AudioStreamEvent::Started)));
    assert!(matches!(events.last(), Some(AudioStreamEvent::Finished)));
}

#[test]
fn stream_events_middle_are_all_chunks() {
    let events = build_sine_wave_stream_events(DURATION, FREQ, SAMPLE_RATE, CHUNK_SAMPLE_COUNT);
    let middle = &events[1..events.len() - 1];
    assert!(middle
        .iter()
        .all(|e| matches!(e, AudioStreamEvent::Chunk { .. })));
}

#[test]
fn stream_events_chunk_count_exceeds_one() {
    let events = build_sine_wave_stream_events(DURATION, FREQ, SAMPLE_RATE, CHUNK_SAMPLE_COUNT);
    let chunk_count = events
        .iter()
        .filter(|e| matches!(e, AudioStreamEvent::Chunk { .. }))
        .count();
    assert!(chunk_count > 1, "expected chunking, got {chunk_count} chunks");
}

#[test]
fn stream_events_first_chunk_contains_riff_header() {
    let events = build_sine_wave_stream_events(DURATION, FREQ, SAMPLE_RATE, CHUNK_SAMPLE_COUNT);
    let first_chunk = events
        .iter()
        .find_map(|e| match e {
            AudioStreamEvent::Chunk { bytes } => Some(bytes),
            _ => None,
        })
        .expect("at least one chunk");
    assert_eq!(&first_chunk[..4], b"RIFF");
}

#[test]
fn stream_events_concatenated_chunks_equal_full_wav() {
    let events = build_sine_wave_stream_events(DURATION, FREQ, SAMPLE_RATE, CHUNK_SAMPLE_COUNT);
    let full_wav = generate_sine_wave_wav(DURATION, FREQ, SAMPLE_RATE);

    let mut concatenated = Vec::new();
    for event in &events {
        if let AudioStreamEvent::Chunk { bytes } = event {
            concatenated.extend_from_slice(bytes);
        }
    }
    assert_eq!(concatenated, full_wav);
}

// ---- 测试组 C：协议序列化 ----

#[test]
fn audio_stream_event_serializes_with_type_tag_and_camel_case() {
    let started = serde_json::to_value(&AudioStreamEvent::Started).unwrap();
    assert_eq!(started, serde_json::json!({"type":"started"}));

    let chunk = serde_json::to_value(&AudioStreamEvent::Chunk {
        bytes: vec![0, 1],
    })
    .unwrap();
    assert_eq!(chunk, serde_json::json!({"type":"chunk","bytes":[0,1]}));

    let finished = serde_json::to_value(&AudioStreamEvent::Finished).unwrap();
    assert_eq!(finished, serde_json::json!({"type":"finished"}));

    let error = serde_json::to_value(&AudioStreamEvent::Error {
        message: "x".into(),
    })
    .unwrap();
    assert_eq!(error, serde_json::json!({"type":"error","message":"x"}));
}
