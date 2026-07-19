use tauri::State;

use crate::service::{
    models::{
        CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload, StreamingSpeechTaskResult,
    },
    ServiceState,
};

/// 流式音频事件协议。前端通过 `Channel<AudioStreamEvent>` 订阅。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AudioStreamEvent {
    Started,
    Chunk { bytes: Vec<u8> },
    Finished,
    Error { message: String },
}

/// 生成正弦波 PCM WAV 字节（44100Hz / 16-bit / mono）。
pub fn generate_sine_wave_wav(duration_secs: f32, freq: f32, sample_rate: u32) -> Vec<u8> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let data_size = num_samples * 2;
    let byte_rate = sample_rate * 2;
    let mut buf = Vec::with_capacity(44 + data_size);

    // RIFF header
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");

    // fmt chunk
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // audio_format = PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&byte_rate.to_le_bytes());
    buf.extend_from_slice(&2u16.to_le_bytes()); // block_align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits_per_sample

    // data chunk
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_size as u32).to_le_bytes());

    // PCM samples (sine wave, amplitude 0.3 防削波)
    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * freq * 2.0 * std::f32::consts::PI).sin() * 0.3;
        let value = (sample * i16::MAX as f32) as i16;
        buf.extend_from_slice(&value.to_le_bytes());
    }

    buf
}

/// 构建流式事件序列：`[Started, Chunk(header+pcm1), Chunk(pcm2), ..., Finished]`。
pub fn build_sine_wave_stream_events(
    duration_secs: f32,
    freq: f32,
    sample_rate: u32,
    chunk_sample_count: usize,
) -> Vec<AudioStreamEvent> {
    let wav = generate_sine_wave_wav(duration_secs, freq, sample_rate);
    let header_len = 44usize;
    let pcm = &wav[header_len..];
    let bytes_per_sample = 2usize;
    let chunk_byte_len = chunk_sample_count * bytes_per_sample;

    let mut events = Vec::new();
    events.push(AudioStreamEvent::Started);

    let mut cursor = 0usize;
    let mut first = true;
    while cursor < pcm.len() {
        let end = (cursor + chunk_byte_len).min(pcm.len());
        if first {
            // 首 chunk：WAV 头 + 首段 PCM
            let mut chunk_buf = Vec::with_capacity(header_len + (end - cursor));
            chunk_buf.extend_from_slice(&wav[..header_len]);
            chunk_buf.extend_from_slice(&pcm[cursor..end]);
            events.push(AudioStreamEvent::Chunk { bytes: chunk_buf });
            first = false;
        } else {
            events.push(AudioStreamEvent::Chunk {
                bytes: pcm[cursor..end].to_vec(),
            });
        }
        cursor = end;
    }

    events.push(AudioStreamEvent::Finished);
    events
}

const SINE_WAVE_DURATION_SECS: f32 = 2.0;
const SINE_WAVE_FREQ: f32 = 440.0;
const SINE_WAVE_SAMPLE_RATE: u32 = 44100;
const CHUNK_SAMPLE_COUNT: usize = 4096;
const INTER_CHUNK_DELAY_MS: u64 = 50;

/// 流式音频占位 hook。生成合成正弦波 WAV，分块经 Channel 下发。
/// 不对接适配器层；`task_id` / `context_id` 仅日志记录。
#[tauri::command]
pub async fn stream_audio_placeholder(
    task_id: i64,
    context_id: String,
    on_event: tauri::ipc::Channel<AudioStreamEvent>,
) -> std::result::Result<(), String> {
    tracing::info!(
        task_id,
        %context_id,
        "stream_audio_placeholder invoked (placeholder)"
    );
    let events = build_sine_wave_stream_events(
        SINE_WAVE_DURATION_SECS,
        SINE_WAVE_FREQ,
        SINE_WAVE_SAMPLE_RATE,
        CHUNK_SAMPLE_COUNT,
    );
    for event in events {
        on_event.send(event).map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_millis(INTER_CHUNK_DELAY_MS)).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn create_streaming_speech_task(
    payload: CreateStreamingSpeechTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<StreamingSpeechTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_streaming_speech_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn send_streaming_message(
    payload: SendStreamingMessagePayload,
    on_event: tauri::ipc::Channel<AudioStreamEvent>,
    state: State<'_, ServiceState>,
) -> std::result::Result<(), String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .send_streaming_message(payload, on_event)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cancel_streaming_task(
    task_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .cancel_streaming_task(task_id)
        .await
        .map_err(|err| err.to_string())
}
