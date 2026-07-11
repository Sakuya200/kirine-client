# 前后端音频流式输出链路 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 打通一条新的前后端音频流式输出链路--后端占位 hook 经 Tauri 2 Channel 流式下发合成正弦波 WAV，前端可复用组件支持缓冲流式播放与本地路径播放双模式。

**Architecture:** Tauri 2 `ipc::Channel<AudioStreamEvent>` 传输 + 前端组件/hook 拆分。后端 `stream_audio_placeholder` 命令不接入 Service trait/适配器层，调用纯函数 `build_sine_wave_stream_events`（内部 `generate_sine_wave_wav`）生成 `[Started, Chunk..., Finished]` 事件序列，逐个 emit（chunk 间 sleep 50ms）。前端 `useStreamableAudioPlayer` 拥有 Channel 订阅、chunk 累计、Blob 构建、HTMLAudioElement 生命周期；`StreamableAudioPlayer.vue` 为纯 UI（一个播放按钮）。

**Tech Stack:** Rust + Tauri 2（`tauri::ipc::Channel`）、Vue 3 + TypeScript（`@tauri-apps/api/core` 的 `Channel`/`invoke`/`convertFileSrc`）、现有 `BaseButton` + heroicons + Tailwind brand 配色。

## Global Constraints

- **不触碰** `Service` trait、`LocalService`/`RemoteService`、适配器层、DB、迁移、现有 `AudioResultPlayer.vue` / `useTaskAudioPlayer.ts`。
- `context_id` 类型为 `String`，纯占位，仅 `tracing::info!` 记录，不参与生成逻辑。
- 后端 hook 不接收 `State<'_, ServiceState>`--不依赖服务层。
- Rust 纯函数与命令位于 `src-tauri/src/hooks/streaming.rs`；测试位于 `src-tauri/tests/streaming_hooks.rs`（复用现有集成测试基建）。
- 前端无测试基建，本计划不为前端写自动化测试；前端验证 = `npm run build`（`vue-tsc --noEmit && vite build`）通过。
- 常量（后端）：`SINE_WAVE_DURATION_SECS=2.0`、`SINE_WAVE_FREQ=440.0`、`SINE_WAVE_SAMPLE_RATE=44100`、`CHUNK_SAMPLE_COUNT=4096`、`INTER_CHUNK_DELAY_MS=50`。
- WAV 格式：44100Hz / 16-bit / mono PCM，幅度 0.3 防削波。
- 提交信息用中文，末尾附 `Co-Authored-By: Claude <noreply@anthropic.com>`。
- 工作目录为 `d:\Project\llm\kirine-client`，cargo 命令用 `--manifest-path src-tauri/Cargo.toml`。

## File Structure

| 文件 | 操作 | 职责 |
|------|------|------|
| `src-tauri/src/hooks/streaming.rs` | 新建 | `AudioStreamEvent` 枚举 + `generate_sine_wave_wav` + `build_sine_wave_stream_events` 纯函数 + `stream_audio_placeholder` 命令 |
| `src-tauri/src/hooks/mod.rs` | 修改 | `pub(crate) mod streaming;` + `generate_handler!` 注册命令 |
| `src-tauri/src/test_support.rs` | 修改 | 重导出 `AudioStreamEvent` / `generate_sine_wave_wav` / `build_sine_wave_stream_events` |
| `src-tauri/tests/streaming_hooks.rs` | 新建 | Rust 单测：WAV 格式 / 事件序列 / 协议序列化 |
| `src/hooks/useStreamableAudioPlayer.ts` | 新建 | 流式 + 路径双模式播放 hook |
| `src/components/common/StreamableAudioPlayer.vue` | 新建 | 播放按钮组件（纯 UI） |

---

## Task 1: Backend — 事件协议 + 正弦波生成器 + 分块器（纯函数）+ 单测

**Files:**

- Create: `src-tauri/src/hooks/streaming.rs`
- Modify: `src-tauri/src/hooks/mod.rs`（新增 `pub(crate) mod streaming;`）
- Modify: `src-tauri/src/test_support.rs`（重导出纯函数与枚举）
- Test: `src-tauri/tests/streaming_hooks.rs`

**Interfaces:**

- Produces: `AudioStreamEvent`（枚举，`pub`，serde `tag="type"` + `rename_all="camelCase"`，变体 `Started` / `Chunk { bytes: Vec<u8> }` / `Finished` / `Error { message: String }`）、`generate_sine_wave_wav(duration_secs: f32, freq: f32, sample_rate: u32) -> Vec<u8>`、`build_sine_wave_stream_events(duration_secs: f32, freq: f32, sample_rate: u32, chunk_sample_count: usize) -> Vec<AudioStreamEvent>`。Task 2 的命令依赖这三个符号。

- [ ] **Step 1: 创建 streaming.rs 含枚举与纯函数桩（返回 `todo!()`）**

创建 `src-tauri/src/hooks/streaming.rs`（仅枚举与纯函数桩；常量与 `Channel` 留到 Task 2 与命令一同引入，避免本任务产生未使用项告警）：

```rust
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
    todo!()
}

/// 构建流式事件序列：`[Started, Chunk(header+pcm1), Chunk(pcm2), ..., Finished]`。
pub fn build_sine_wave_stream_events(
    duration_secs: f32,
    freq: f32,
    sample_rate: u32,
    chunk_sample_count: usize,
) -> Vec<AudioStreamEvent> {
    todo!()
}
```

- [ ] **Step 2: 在 hooks/mod.rs 注册 streaming 子模块**

修改 `src-tauri/src/hooks/mod.rs`，在现有 `mod task_history;` 下方加一行 `pub(crate) mod streaming;`（用 `pub(crate)` 以便 `test_support` 重导出，对齐 `service/mod.rs` 的 `pub(crate) mod pipeline;` 模式）：

```rust
mod model_info;
mod settings;
mod speaker_info;
mod task_history;
pub(crate) mod streaming;
```

（仅在 `mod task_history;` 之后新增最后一行，其余保持不变。）

- [ ] **Step 3: 在 test_support.rs 重导出纯函数与枚举**

修改 `src-tauri/src/test_support.rs`，在现有 `pub use crate::service::pipeline::build_llm_task_script_args;` 附近新增：

```rust
pub use crate::hooks::streaming::{
    build_sine_wave_stream_events, generate_sine_wave_wav, AudioStreamEvent,
};
```

- [ ] **Step 4: 编写失败测试 `tests/streaming_hooks.rs`**

创建 `src-tauri/tests/streaming_hooks.rs`：

```rust
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
```

- [ ] **Step 5: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test streaming_hooks`
Expected: FAIL — 测试因 `todo!()` panic（"not yet implemented"）。编译应通过（桩签名已存在）。

- [ ] **Step 6: 实现 `generate_sine_wave_wav`**

替换 `src-tauri/src/hooks/streaming.rs` 中 `generate_sine_wave_wav` 的 `todo!()` 为：

```rust
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
```

- [ ] **Step 7: 实现 `build_sine_wave_stream_events`**

替换 `src-tauri/src/hooks/streaming.rs` 中 `build_sine_wave_stream_events` 的 `todo!()` 为：

```rust
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
```

- [ ] **Step 8: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test streaming_hooks`
Expected: PASS — 全部 9 个测试通过。

- [ ] **Step 9: 运行全量测试，确认无回归**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — 既有测试不受影响（streaming 模块独立，未改 Service/DB）。

- [ ] **Step 10: 提交**

```bash
git add src-tauri/src/hooks/streaming.rs src-tauri/src/hooks/mod.rs src-tauri/src/test_support.rs src-tauri/tests/streaming_hooks.rs
git commit -m "$(cat <<'EOF'
新增流式音频占位纯函数与单测（事件协议/正弦波WAV/分块器）

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Backend — stream_audio_placeholder 命令 + 注册

**Files:**

- Modify: `src-tauri/src/hooks/streaming.rs`（追加命令）
- Modify: `src-tauri/src/hooks/mod.rs`（`generate_handler!` 注册）

**Interfaces:**

- Consumes: Task 1 的 `build_sine_wave_stream_events`。本任务定义常量 `SINE_WAVE_DURATION_SECS` / `SINE_WAVE_FREQ` / `SINE_WAVE_SAMPLE_RATE` / `CHUNK_SAMPLE_COUNT` / `INTER_CHUNK_DELAY_MS` 供命令使用。
- Produces: Tauri 命令 `stream_audio_placeholder(task_id: i64, context_id: String, on_event: tauri::ipc::Channel<AudioStreamEvent>) -> Result<(), String>`，前端经 `invoke('stream_audio_placeholder', { taskId, contextId, onEvent: channel })` 调用。Task 3/4 的前端依赖此命令名。

- [ ] **Step 1: 在 streaming.rs 追加常量与命令**

在 `src-tauri/src/hooks/streaming.rs` 文件末尾追加（用全限定 `tauri::ipc::Channel` 避免 `use` 导入；常量与其唯一消费者命令一同引入）：

```rust
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
        on_event.emit(event).map_err(|e| e.to_string())?;
        tokio::time::sleep(std::time::Duration::from_millis(INTER_CHUNK_DELAY_MS)).await;
    }
    Ok(())
}
```

- [ ] **Step 2: 在 generate_handler! 注册命令**

修改 `src-tauri/src/hooks/mod.rs` 的 `load_hooks` 函数，在 `generate_handler!` 宏内追加 `streaming::stream_audio_placeholder,`（建议放在 `settings::save_settings_config` 之前一行）：

```rust
pub fn load_hooks(builder: tauri::Builder<Wry>) -> tauri::Builder<Wry> {
    builder.invoke_handler(tauri::generate_handler![
        speaker_info::create_speaker_info,
        import_model_as_speaker,
        speaker_info::list_speaker_infos,
        speaker_info::update_speaker_info,
        speaker_info::delete_speaker_info,
        model_info::list_model_infos,
        model_info::get_device_type,
        model_info::install_model,
        model_info::uninstall_model,
        task_history::list_history_records,
        task_history::get_history_record,
        task_history::get_text_to_speech_audio,
        task_history::get_voice_clone_audio,
        task_history::get_voice_design_audio,
        task_history::save_text_to_speech_audio_as,
        task_history::save_voice_clone_audio_as,
        task_history::save_voice_design_audio_as,
        task_history::save_model_training_template_as,
        task_history::delete_history_record,
        task_history::create_text_to_speech_task,
        task_history::create_model_training_task,
        task_history::cancel_history_task,
        task_history::create_voice_clone_task,
        task_history::create_voice_design_task,
        streaming::stream_audio_placeholder,
        settings::get_settings_config,
        settings::get_ui_config,
        settings::save_settings_config
    ])
}
```

- [ ] **Step 3: 构建并测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS — 编译通过（命令注册成功），全部测试通过。命令本身无单测（Channel 需 IPC 上下文），正确性由 Task 1 纯函数保证。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/hooks/streaming.rs src-tauri/src/hooks/mod.rs
git commit -m "$(cat <<'EOF'
新增stream_audio_placeholder流式hook命令并注册

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Frontend — useStreamableAudioPlayer hook

**Files:**

- Create: `src/hooks/useStreamableAudioPlayer.ts`

**Interfaces:**

- Consumes: Task 2 的 `stream_audio_placeholder` 命令（经 `invoke`）、`@tauri-apps/api/core` 的 `Channel` / `convertFileSrc` / `invoke`。
- Produces: `useStreamableAudioPlayer(options?)` hook，返回 `{ isPlaying, hasData, isStreaming, streamComplete, playbackProgress, currentPlaybackSeconds, startStreaming, setAudioPath, togglePlayback, stopPlayback, reset }`。Task 4 的组件依赖 `isPlaying` / `hasData` / `isStreaming` / `togglePlayback` / `startStreaming` / `setAudioPath`。

- [ ] **Step 1: 创建 hook 文件**

创建 `src/hooks/useStreamableAudioPlayer.ts`：

```ts
import { computed, onBeforeUnmount, ref } from 'vue';
import { Channel, convertFileSrc, invoke } from '@tauri-apps/api/core';

type AudioStreamEvent =
  | { type: 'started' }
  | { type: 'chunk'; bytes: number[] }
  | { type: 'finished' }
  | { type: 'error'; message: string };

interface UseStreamableAudioPlayerOptions {
  onPlaybackEnded?: () => void;
  onPlaybackError?: () => void;
  onStreamError?: (message: string) => void;
}

export const useStreamableAudioPlayer = (options: UseStreamableAudioPlayerOptions = {}) => {
  const isPlaying = ref(false);
  const hasData = ref(false);
  const isStreaming = ref(false);
  const streamComplete = ref(false);
  const playbackProgress = ref(0);
  const audioCurrentTime = ref(0);

  const currentPlaybackSeconds = computed(() => Math.round(audioCurrentTime.value));

  let audioElement: HTMLAudioElement | null = null;
  let audioObjectUrl: string | null = null;
  let accumulated: number[] = [];
  let pathUrl: string | null = null;
  let removeAudioListeners: (() => void) | null = null;

  const releaseObjectUrl = () => {
    if (audioObjectUrl) {
      URL.revokeObjectURL(audioObjectUrl);
      audioObjectUrl = null;
    }
  };

  const destroyAudioElement = () => {
    if (audioElement) {
      removeAudioListeners?.();
      removeAudioListeners = null;
      audioElement.pause();
      audioElement.removeAttribute('src');
      audioElement = null;
    }
    isPlaying.value = false;
    playbackProgress.value = 0;
    audioCurrentTime.value = 0;
  };

  const stopPlayback = () => {
    audioElement?.pause();
    isPlaying.value = false;
  };

  const reset = ({ releaseSource = false } = {}) => {
    destroyAudioElement();
    if (releaseSource) {
      releaseObjectUrl();
      accumulated = [];
      pathUrl = null;
      hasData.value = false;
      isStreaming.value = false;
      streamComplete.value = false;
    }
  };

  const ensureAudioElement = (src: string) => {
    if (!audioElement || audioElement.src !== src) {
      destroyAudioElement();
      const next = new Audio(src);

      const handleTimeUpdate = () => {
        const duration =
          Number.isFinite(next.duration) && next.duration > 0 ? next.duration : 0;
        audioCurrentTime.value = next.currentTime;
        playbackProgress.value =
          duration > 0 ? Math.min(100, (next.currentTime / duration) * 100) : 0;
      };
      const handlePause = () => {
        isPlaying.value = false;
      };
      const handlePlay = () => {
        isPlaying.value = true;
      };
      const handleEnded = () => {
        isPlaying.value = false;
        playbackProgress.value = 100;
        options.onPlaybackEnded?.();
      };
      const handleError = () => {
        stopPlayback();
        options.onPlaybackError?.();
      };

      next.addEventListener('timeupdate', handleTimeUpdate);
      next.addEventListener('pause', handlePause);
      next.addEventListener('play', handlePlay);
      next.addEventListener('ended', handleEnded);
      next.addEventListener('error', handleError);

      removeAudioListeners = () => {
        next.removeEventListener('timeupdate', handleTimeUpdate);
        next.removeEventListener('pause', handlePause);
        next.removeEventListener('play', handlePlay);
        next.removeEventListener('ended', handleEnded);
        next.removeEventListener('error', handleError);
      };

      audioElement = next;
    }
    return audioElement;
  };

  const resolveSourceUrl = (): string | null => {
    if (pathUrl) {
      return pathUrl;
    }
    if (accumulated.length > 0) {
      const blob = new Blob([Uint8Array.from(accumulated)], { type: 'audio/wav' });
      releaseObjectUrl();
      audioObjectUrl = URL.createObjectURL(blob);
      return audioObjectUrl;
    }
    return null;
  };

  const togglePlayback = () => {
    if (!hasData.value) {
      return;
    }
    if (isPlaying.value) {
      stopPlayback();
      return;
    }
    const src = resolveSourceUrl();
    if (!src) {
      return;
    }
    const element = ensureAudioElement(src);
    element.play().catch(() => options.onPlaybackError?.());
  };

  const startStreaming = async (taskId: number, contextId: string) => {
    accumulated = [];
    hasData.value = false;
    streamComplete.value = false;
    isStreaming.value = true;
    destroyAudioElement();
    releaseObjectUrl();
    pathUrl = null;

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      switch (message.type) {
        case 'started':
          break;
        case 'chunk':
          accumulated.push(...message.bytes);
          if (!hasData.value) {
            hasData.value = true;
          }
          break;
        case 'finished':
          streamComplete.value = true;
          isStreaming.value = false;
          break;
        case 'error':
          isStreaming.value = false;
          options.onStreamError?.(message.message);
          break;
      }
    };

    try {
      await invoke('stream_audio_placeholder', { taskId, contextId, onEvent: channel });
    } catch (error) {
      isStreaming.value = false;
      options.onStreamError?.(error instanceof Error ? error.message : String(error));
    }
  };

  const setAudioPath = (filePath: string) => {
    destroyAudioElement();
    releaseObjectUrl();
    accumulated = [];
    pathUrl = filePath ? convertFileSrc(filePath) : null;
    hasData.value = !!pathUrl;
  };

  onBeforeUnmount(() => {
    reset({ releaseSource: true });
  });

  return {
    isPlaying,
    hasData,
    isStreaming,
    streamComplete,
    playbackProgress,
    currentPlaybackSeconds,
    startStreaming,
    setAudioPath,
    togglePlayback,
    stopPlayback,
    reset,
  };
};
```

- [ ] **Step 2: 类型检查 + 构建，确认通过**

Run: `npm run build`
Expected: PASS — `vue-tsc --noEmit` 无类型错误，`vite build` 成功。

- [ ] **Step 3: 提交**

```bash
git add src/hooks/useStreamableAudioPlayer.ts
git commit -m "$(cat <<'EOF'
新增useStreamableAudioPlayer前端流式/路径播放hook

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Frontend — StreamableAudioPlayer 组件

**Files:**

- Create: `src/components/common/StreamableAudioPlayer.vue`

**Interfaces:**

- Consumes: Task 3 的 `useStreamableAudioPlayer`、`BaseButton`、`@heroicons/vue/24/outline` 的 `PlayIcon` / `PauseIcon`、`useUiStore`。
- Produces: `<StreamableAudioPlayer :mode="..." :task-id="..." :context-id="..." />`（stream 模式）或 `<StreamableAudioPlayer mode="path" :audio-path="..." />`（path 模式）。

- [ ] **Step 1: 创建组件文件**

创建 `src/components/common/StreamableAudioPlayer.vue`：

```vue
<script setup lang="ts">
import { PauseIcon, PlayIcon } from '@heroicons/vue/24/outline';
import { computed, watch } from 'vue';

import BaseButton from '@/components/common/BaseButton.vue';
import { useStreamableAudioPlayer } from '@/hooks/useStreamableAudioPlayer';
import { useUiStore } from '@/stores/ui';

interface Props {
  mode: 'stream' | 'path';
  taskId?: number;
  contextId?: string;
  audioPath?: string;
}

const props = withDefaults(defineProps<Props>(), {
  taskId: undefined,
  contextId: '',
  audioPath: undefined,
});

const uiStore = useUiStore();

const { isPlaying, hasData, isStreaming, togglePlayback, startStreaming, setAudioPath } =
  useStreamableAudioPlayer({
    onPlaybackEnded: () => {
      uiStore.notifyInfo('音频播放结束。', 2200);
    },
    onPlaybackError: () => {
      uiStore.notifyError('音频播放失败，请检查音频数据是否可解码。');
    },
    onStreamError: message => {
      uiStore.notifyError(`音频流式接收失败：${message}`);
    },
  });

const actionLabel = computed(() => {
  if (isPlaying.value) return '暂停播放';
  if (!hasData.value) return '等待数据';
  return '播放音频';
});

if (props.mode === 'stream') {
  watch(
    () => [props.taskId, props.contextId] as const,
    ([taskId, contextId]) => {
      if (taskId != null) {
        void startStreaming(taskId, contextId);
      }
    },
    { immediate: true },
  );
} else {
  watch(
    () => props.audioPath,
    path => {
      if (path !== undefined) {
        setAudioPath(path);
      }
    },
    { immediate: true },
  );
}
</script>

<template>
  <div class="inline-flex items-center gap-2 rounded-2xl border border-brand-200 bg-white/85 p-3">
    <BaseButton tone="ghost" :disabled="!hasData" @click="togglePlayback">
      <component :is="isPlaying ? PauseIcon : PlayIcon" class="h-4 w-4" aria-hidden="true" />
      <span>{{ actionLabel }}</span>
    </BaseButton>
    <span v-if="mode === 'stream' && !hasData" class="text-xs text-stone-500">等待音频数据…</span>
    <span v-else-if="mode === 'stream' && isStreaming" class="text-xs text-stone-500">流式接收中…</span>
  </div>
</template>
```

- [ ] **Step 2: 类型检查 + 构建，确认通过**

Run: `npm run build`
Expected: PASS — `vue-tsc --noEmit` 无类型错误，`vite build` 成功。

- [ ] **Step 3: 提交**

```bash
git add src/components/common/StreamableAudioPlayer.vue
git commit -m "$(cat <<'EOF'
新增StreamableAudioPlayer流式播放组件

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Known Follow-ups（非本期范围）

- **路径模式运行时依赖 asset 协议配置**：`convertFileSrc` 走 Tauri 2 asset 协议，实际挂载使用路径模式前需在 `src-tauri/capabilities/*.json` 配置 `assetProtocol.scope` 允许目标路径。本期组件不挂载、不测试路径模式，故不阻塞交付。
- **组件挂载**：本期组件未挂载到任何页面。后续接入任务结果卡或独立播放面板时再挂载并手动验证流式/路径双模式。
- **协议优化**：`bytes` 当前为 JSON number 数组，真实接入适配器后可改为 base64 或 Tauri 原生 binary 通道以降开销。
- **后端对接适配器**：`stream_audio_placeholder` 后续替换为真实流式推理，`context_id` 届时承载会话上下文。
