# 前后端音频流式输出链路 设计文档

- 日期：2026-07-11
- 状态：已获批，待编写实现计划
- 分支：v.0.12.0

## 1. 背景与目标

当前所有模型功能的音频输出均为**非流式**：任务完成后，后端一次性返回完整音频字节（`get_*_audio` 命令返回 `*AudioAsset { bytes: Vec<u8> }`），前端 `AudioResultPlayer.vue` + `useTaskAudioPlayer.ts` 用 `HTMLAudioElement` + Blob URL 播放。

本设计打通一条**新的前后端音频流式输出链路**，作为基础设施，为后续接入真实模型适配器的流式推理铺路。本期后端为**测试性占位**（生成合成正弦波音频，不对接适配器层），前端交付可复用播放组件。

### 决策汇总

| 维度 | 决策 |
|------|------|
| 流式播放语义 | 缓冲播放：累计已到达 chunk 为 Blob，点击播放当前缓冲 |
| 后端占位数据 | 合成正弦波 PCM WAV（44100Hz / 16-bit / mono / 440Hz / ~2s），header chunk + PCM chunks 分块发送 |
| 组件挂载 | 本阶段仅创建可复用组件 + 单测，不挂载到任何页面 |
| 路径模式 | 本地文件绝对路径，前端 `convertFileSrc` 转 webview asset URL 播放 |
| 传输架构 | Tauri 2 `ipc::Channel<enum>` + 前端组件/hook 拆分 |
| 测试范围 | 仅 Rust 单测（复用现有 `tests/*.rs` + `#[cfg(test)]` 基建） |

## 2. 架构与数据流

```
[StreamableAudioPlayer.vue]  ──props(mode/taskId/contextId/audioPath)──►  [useStreamableAudioPlayer.ts]
        │                                                                          │
        │ stream 模式: startStreaming(taskId, contextId)                            │
        │   └─► invoke('stream_audio_placeholder', { taskId, contextId, onEvent: Channel })
        │                                                                          │
        │ path 模式: setAudioPath(path) ─► convertFileSrc(path) -> URL             │
        │                                                                          │
        │ togglePlayback() ─► 从当前累计 buffer / path URL 构建 Blob ─► HTMLAudioElement.play()
        ▼                                                                          ▼
[stream_audio_placeholder command]                                          [Channel.onmessage]
   task_id + context_id (占位，仅 tracing::info! 记录)                        started / chunk / finished / error
   │                                                                          │
   ├─► build_sine_wave_stream_events()  (纯函数: 生成 + 分块 -> Vec<AudioStreamEvent>)
   │      └─► generate_sine_wave_wav()  (纯函数: 完整 WAV 字节)
   │
   └─► for event in events { on_event.emit(event); sleep(50ms) }
```

### 核心边界

- 后端 hook **不接入 `Service` trait / 适配器层**，自包含生成合成音频。`task_id` + `context_id` 仅 `tracing::info!` 记录，不参与生成逻辑。
- 纯函数 `build_sine_wave_stream_events` / `generate_sine_wave_wav` 与 Channel emit 解耦——前者可单测，后者是薄封装（iterate + emit + sleep）。
- 前端 hook 拥有 Channel 订阅、chunk 累计、Blob 构建、`HTMLAudioElement` 生命周期；组件为纯 UI。
- 本期不触碰 `Service` trait、`LocalService`/`RemoteService`、适配器层、DB、迁移、现有 `AudioResultPlayer`。

## 3. 后端设计

### 3.1 新文件 `src-tauri/src/hooks/streaming.rs`

#### 事件协议 `AudioStreamEvent`

```rust
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AudioStreamEvent {
    Started,
    Chunk { bytes: Vec<u8> },          // -> {"type":"chunk","bytes":[...]}
    Finished,
    Error { message: String },
}
```

`bytes` 序列化为 JSON number 数组，与现有 `AudioAsset.bytes` 一致。占位阶段可接受（2s 正弦波约 176KB，分约 22 个 chunk，每 chunk 约 8KB → 约 28KB JSON）。真实接入适配器后可优化为 base64 或 Tauri 原生 binary 通道——本期不做。

#### 命令 `stream_audio_placeholder`

```rust
const SINE_WAVE_DURATION_SECS: f32 = 2.0;
const SINE_WAVE_FREQ: f32 = 440.0;
const SINE_WAVE_SAMPLE_RATE: u32 = 44100;
const CHUNK_SAMPLE_COUNT: usize = 4096;        // = 8192 bytes/chunk
const INTER_CHUNK_DELAY_MS: u64 = 50;          // 模拟真实流式，使 disable->enable 可观测

#[tauri::command]
pub async fn stream_audio_placeholder(
    task_id: i64,
    context_id: String,
    on_event: tauri::ipc::Channel<AudioStreamEvent>,
) -> std::result::Result<(), String> {
    tracing::info!(task_id, %context_id, "stream_audio_placeholder invoked (placeholder)");
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

- 不接收 `State<'_, ServiceState>`——不依赖服务层。
- `context_id` 类型为 `String`，纯占位，仅日志记录。
- 每个 chunk 后 sleep 50ms，模拟真实流式到达，使前端"无数据禁用 → 有数据启用"的转换可被观测。

#### 纯函数 1：`generate_sine_wave_wav`

```rust
pub fn generate_sine_wave_wav(duration_secs: f32, freq: f32, sample_rate: u32) -> Vec<u8>
```

- 生成 44 字节 WAV 头（RIFF/WAVE/fmt /data magic，audio_format=1 PCM，channels=1，bits_per_sample=16，sample_rate 来自参数）+ 正弦波 PCM 采样。
- 采样数 `num_samples = (duration_secs * sample_rate) as usize`；`data_size = num_samples * 2`。
- 正弦幅度 0.3（防削波）：`sample = sin(2π * freq * t) * 0.3`，转 `i16` 小端。
- RIFF chunk size = 36 + data_size；byte_rate = sample_rate * 2；block_align = 2。
- 输出完整、可播放 WAV 字节。

#### 纯函数 2：`build_sine_wave_stream_events`

```rust
pub fn build_sine_wave_stream_events(
    duration_secs: f32,
    freq: f32,
    sample_rate: u32,
    chunk_sample_count: usize,
) -> Vec<AudioStreamEvent>
```

- 调用 `generate_sine_wave_wav` 生成完整 WAV 字节。
- 分块策略：
  - 第 1 个 `Chunk` = WAV 头（44B）+ 首段 PCM（`chunk_sample_count` 个采样 = `chunk_sample_count * 2` 字节）。
  - 后续 `Chunk` = 后续 PCM 段（每段 `chunk_sample_count` 个采样，末段取剩余）。
- 返回 `[Started, Chunk, Chunk, ..., Finished]`。
- 不变量：所有 `Chunk.bytes` 按序拼接 == 原始完整 WAV 字节；首 `Chunk` 以 `b"RIFF"` 开头。

### 3.2 注册

`src-tauri/src/hooks/mod.rs`：

- 新增 `mod streaming;`
- `load_hooks` 的 `generate_handler!` 宏内追加 `streaming::stream_audio_placeholder,`。

## 4. 前端设计

### 4.1 新文件 `src/hooks/useStreamableAudioPlayer.ts`

镜像现有 `useTaskAudioPlayer.ts` 结构，新增 Channel 订阅与双模式源管理。

#### 类型

```ts
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
```

#### 响应式状态

- `isPlaying: Ref<boolean>` —— 是否正在播放
- `hasData: Ref<boolean>` —— 按钮启用门（stream 模式首次收到 chunk 置 true；path 模式设置有效 path 时置 true）
- `isStreaming: Ref<boolean>` —— 流式接收进行中
- `streamComplete: Ref<boolean>` —— 流已结束
- `playbackProgress: Ref<number>` —— 0–100

#### 内部状态（非响应式）

- `audioElement: HTMLAudioElement | null`
- `audioObjectUrl: string | null`
- `accumulated: number[]` —— stream 模式 chunk 累计缓冲
- `pathUrl: string | null` —— path 模式 URL
- `removeAudioListeners: (() => void) | null`

#### Stream 模式：`startStreaming(taskId: number, contextId: string): Promise<void>`

1. 重置：`accumulated = []`、`hasData = false`、`streamComplete = false`、`isStreaming = true`；释放旧 objectURL。
2. `const channel = new Channel<AudioStreamEvent>();`
3. `channel.onmessage` 分发：
   - `started`：无操作（流已开始标记）。
   - `chunk`：`accumulated.push(...msg.bytes)`；若 `!hasData.value` 则置 `hasData.value = true`（启用按钮）。
   - `finished`：`streamComplete = true`、`isStreaming = false`。
   - `error`：`isStreaming = false`；调用 `onStreamError?.(msg.message)`。
4. `await invoke('stream_audio_placeholder', { taskId, contextId, onEvent: channel })`；catch 时 `isStreaming = false` + `onStreamError`。

**不在每个 chunk 重建 Blob**——累计 bytes（廉价），仅点击播放时从当前 buffer 构建 Blob + objectURL（避免 22 次重建）。

#### Path 模式：`setAudioPath(filePath: string): void`

- 释放旧 objectURL；`pathUrl = convertFileSrc(filePath)`；`hasData.value = !!filePath`；清空 `accumulated`。

#### `togglePlayback(): void`（两模式通用）

1. `if (!hasData.value) return;`（按钮已禁用，双保险）。
2. `if (isPlaying.value) { pause(); return; }`。
3. 否则：从当前源构建/复用 objectURL（stream 模式从 `accumulated` 构建 `Blob([Uint8Array.from(accumulated)], { type: 'audio/wav' })`；path 模式用 `pathUrl`）→ 确保 `HTMLAudioElement` 已绑定该 URL → `play()`。
4. 流模式下若流未完成时点击：播放当前已缓冲内容（部分 WAV；header 已在首 chunk 故可解码）。流继续到达后再次点击播放更新后的缓冲——这是缓冲播放的既定语义。

#### 生命周期

- `onBeforeUnmount`：释放 objectURL（`URL.revokeObjectURL`）、销毁 audio element、移除监听。

#### 返回

`{ isPlaying, hasData, isStreaming, streamComplete, playbackProgress, startStreaming, setAudioPath, togglePlayback, reset }`。

### 4.2 新文件 `src/components/common/StreamableAudioPlayer.vue`

#### Props

```ts
interface Props {
  mode: 'stream' | 'path';
  taskId?: number;        // stream 模式
  contextId?: string;     // stream 模式，占位，默认 ''
  audioPath?: string;     // path 模式
}
```

`withDefaults`：`contextId: ''`。

#### 行为

- Stream 模式：`onMounted` + `watch(() => [props.taskId, props.contextId])` → 调 `startStreaming(taskId, contextId)`（仅当 `taskId != null` 时触发）。
- Path 模式：`watch(() => props.audioPath)` → 调 `setAudioPath(path)`。

#### UI（严格契合项目风格）

复用 `BaseButton`（tone=`ghost`）、`@heroicons/vue/24/outline` 的 `PlayIcon`/`PauseIcon`、brand 配色、`rounded-2xl border border-brand-200 bg-white/85`：

```vue
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

`actionLabel` 计算属性：
- `isPlaying` → `暂停播放`
- `!hasData` → `等待数据`
- 否则 → `播放音频`

按钮 `disabled` 绑定 `!hasData`，实现"刚开始没数据禁用，有数据启用"。

## 5. 测试设计（仅 Rust）

遵循项目"抽取纯函数、测纯函数"模式（对齐 `build_llm_task_script_args` 的处理方式：`pub` + 经 `test_support.rs` 重导出 + `tests/*.rs` 测试）。

### 5.1 重导出

`src-tauri/src/test_support.rs` 新增重导出：
- `build_sine_wave_stream_events`
- `generate_sine_wave_wav`（若测试需直接验证 WAV 字节）
- `AudioStreamEvent`

### 5.2 新文件 `src-tauri/tests/streaming_hooks.rs`

三个测试组：

**测试组 A：WAV 格式正确性**（`generate_sine_wave_wav`）
- RIFF/WAVE/fmt /data magic 正确。
- `data_size == num_samples * 2`（`num_samples = (duration * sample_rate) as usize`）。
- RIFF chunk size == 36 + data_size。
- audio_format == 1（PCM）、channels == 1、bits_per_sample == 16、sample_rate == 44100（小端）。
- 输出总长度 == 44 + data_size。

**测试组 B：事件序列正确性**（`build_sine_wave_stream_events`）
- 首事件为 `AudioStreamEvent::Started`。
- 末事件为 `AudioStreamEvent::Finished`。
- 中间事件全为 `Chunk` 变体。
- 所有 `Chunk.bytes` 按序拼接 == `generate_sine_wave_wav(...)` 输出。
- 首 `Chunk.bytes` 前 4 字节 == `b"RIFF"`（header 在前）。
- `Chunk` 数量 > 1（确证分块发生）。

**测试组 C：协议序列化**（`AudioStreamEvent` serde）
- `Started` → `{"type":"started"}`
- `Chunk { bytes: vec![0,1] }` → `{"type":"chunk","bytes":[0,1]}`
- `Finished` → `{"type":"finished"}`
- `Error { message: "x".into() }` → `{"type":"error","message":"x"}`
- 验证 `tag = "type"` 与 `rename_all = "camelCase"`。

### 5.3 不测范围（明确）

- `stream_audio_placeholder` 命令的 Channel emit 实际链路：需 Tauri IPC 上下文，无法脱离 app 单测。命令本身是 iterate + emit + sleep 的薄封装，正确性由纯函数保证。
- 前端 hook / 组件：本期不引入前端测试基建，不写自动化测试；后续挂载到页面时手动验证。

与项目既有测试哲学一致（参考记忆中"测试范围原则"）。

## 6. 文件清单

### 新增

| 文件 | 职责 |
|------|------|
| `src-tauri/src/hooks/streaming.rs` | hook 命令 + `AudioStreamEvent` 枚举 + `generate_sine_wave_wav` + `build_sine_wave_stream_events` 纯函数 |
| `src-tauri/tests/streaming_hooks.rs` | Rust 单测（WAV 格式 / 事件序列 / 协议序列化） |
| `src/hooks/useStreamableAudioPlayer.ts` | 流式 + 路径双模式播放 hook |
| `src/components/common/StreamableAudioPlayer.vue` | 播放按钮组件（纯 UI） |

### 修改

| 文件 | 改动 |
|------|------|
| `src-tauri/src/hooks/mod.rs` | 新增 `mod streaming;`；`generate_handler!` 注册 `stream_audio_placeholder` |
| `src-tauri/src/test_support.rs` | 重导出 `build_sine_wave_stream_events` / `generate_sine_wave_wav` / `AudioStreamEvent` |

### 不触碰

`Service` trait、`LocalService`/`RemoteService`、适配器层、DB、迁移、现有 `AudioResultPlayer.vue` / `useTaskAudioPlayer.ts`。

## 7. 后续演进（非本期）

- 后端：`stream_audio_placeholder` 替换为真实流式推理——对接适配器层，由适配器以生成器/流形式产出 PCM chunk，经同一 `AudioStreamEvent` 协议下发。`context_id` 届时承载会话/对话上下文。
- 前端：组件挂载到任务结果卡或独立播放面板；可选升级为渐进式实时播放（Web Audio API 播放 PCM chunk）。
- 协议优化：`bytes` 由 JSON number 数组改为 base64 或 Tauri 原生 binary 通道以降开销。
