# 流式语音流水线（Rust 端）设计

- 日期：2026-07-18
- 分支：v.0.12.0
- 范围：Rust 端流式语音流水线（DB / 迁移 / entity / service / pipeline / hooks / 进程管理 / 音频转发）+ 与当前前端接入。真实模型流式脚本 `streaming.py` 与 `begin_llm_task` 透传改造作为契约定义，不在本期实现。
- 关联：前端页面设计见 `docs/superpowers/plans/2026-07-18-streaming-speech-page.md`（该文档"不改动 HistoryTaskType"为纯前端阶段约束，本文档做真实后端时放开）。

## 1. 目标与关键决策

构建一个长期存活的流式语音会话流水线：会话进程随应用生命周期运行，用户在聊天页逐条发送文本，经中间缓存文件实时喂给脚本进程，脚本边合成边把音频分帧流回应用、经既有 `Channel<AudioStreamEvent>` 转发前端实时播放；可从页面强制终止；会话状态与上下文持久化入库以便历史/回放。

已确认决策：

| 维度 | 决策 |
|------|------|
| 体系归属 | 复用 `task_history` + `HistoryTaskType`（新增 `StreamingSpeech` 变体）+ 新增 `streaming_tasks` 详情表；一行会话 = 一行 `task_history` |
| 进程模型 | 一个会话 = 一个长期存活脚本进程（经 `begin_llm_task` 包装器拉起），生命周期与应用绑定 |
| 输入喂给 | `input.jsonl` 中间缓存文件，app 追加、脚本 tail |
| 音频回传 | stdout JSON 行分帧协议（base64 音频块）+ Rust 后台 tokio reader 按 `contextId` 分发到前端 Channel |
| 持久状态 | `context.json`（basic 配置 + messages 列表含生成音频路径），路径用 `%DATA_DIR_PATH%` 占位符 |
| 终态语义 | `Pending -> Running -> Cancelled`（页面终止 / 应用关闭清扫）/ `Failed`；不新增 `TaskStatus` 变体 |
| 取消机制 | 复用 `active_task_controls` + `request_active_task_cancel`（`watch::Sender<bool>`），取消即 `child.kill()` |
| 并发 | v1 同时只允许一个活跃流式会话 |
| 脚本范围 | 仅 Rust 侧 + 契约；真实 `streaming.py` / 包装器透传改造为后续任务 |
| `stream_audio_placeholder` | 保留（有单测、可回退），前端改调新命令 |

## 2. 数据库

### 2.1 新迁移

`src-tauri/src/migration/m20260718_000011_add_streaming_tasks.rs`：建 `streaming_tasks` 表 + 唯一索引 `idx_streaming_tasks_history_id`，外键 `fk_streaming_tasks_history` -> `task_history(id) ON DELETE CASCADE`（仿 `m20260604_000008_add_voice_design_tasks`）。`migration/mod.rs`：注册迁移、`LOCAL_SCHEMA_VERSION` 27 -> 28。

### 2.2 表结构

```sql
CREATE TABLE IF NOT EXISTS streaming_tasks (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    history_id INTEGER NOT NULL,
    base_model TEXT NOT NULL,
    model_version TEXT NOT NULL,
    language TEXT NOT NULL,
    device TEXT NOT NULL DEFAULT 'cpu',
    model_params_json TEXT NOT NULL DEFAULT '{}',
    context_file_path TEXT NOT NULL,        -- %DATA_DIR_PATH%/streaming_<id>/context.json
    input_cache_file_path TEXT NOT NULL,    -- %DATA_DIR_PATH%/streaming_<id>/input.jsonl
    output_audio_dir TEXT NOT NULL,         -- %DATA_DIR_PATH%/streaming_<id>/audio/
    message_count INTEGER NOT NULL DEFAULT 0,
    create_time TEXT NOT NULL,
    modify_time TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT fk_streaming_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_streaming_tasks_history_id ON streaming_tasks (history_id);
```

三个路径字段一律 `%DATA_DIR_PATH%` 占位符（`serialize_task_path` / `resolve_task_path`），与 `tts_tasks.output_file_path` 等一致。时间字段 `create_time` / `modify_time`（`string`，项目硬规范）。

### 2.3 同步

- `create_local_schema.rs`：新增 `StreamingTasks` Iden + 建表语句。
- `db/tables.sql` 与 `db/tables_pgsql.sql`：同步该表（pgsql 用 bigserial/varchar/text/timestamp 等，索引名一致）。

### 2.4 枚举与辅助

- `service/models.rs` `HistoryTaskType` 新增 `StreamingSpeech`：`as_str()` -> `"streaming-speech"`，`storage_dir()` -> `"streaming"`。
- `common/task_paths.rs` `task_log_file_prefix` 新增 `StreamingSpeech => "streaming"` 分支；新增 `streaming_sample_dir` / `streaming_context_json_path` / `streaming_input_cache_path` / `streaming_output_audio_dir` / `streaming_message_audio_path(sample_root, context_id, ext)` 辅助（仿既有 `tts_params_json_path` 等）。

## 3. 文件契约

### 3.1 `context.json`（app 拥有，随消息演进）

```jsonc
{
  "basic": {
    "taskId": 123,
    "baseModel": "gpt_sovits_cpufast",
    "modelVersion": "v1",
    "device": "cpu",
    "language": "chinese",
    "speakers": [
      { "id": "spk-1", "name": "A", "baseModel": "gpt_sovits_cpufast", "refAudioPath": "...", "refAudioName": "ref.wav", "refText": "..." }
    ]
  },
  "messages": [
    { "contextId": "msg-2", "speakerName": "A", "text": "你好", "audioPath": "%DATA_DIR_PATH%/streaming_123/audio/msg-2.wav" }
  ]
}
```

- 会话创建时写 `basic`（含 taskId + 说话人列表）+ 空 `messages`。
- 发送消息时 append 一条 `messages`（`audioPath` 先填入已知目标路径），生成完成后确认回填（路径不变，仅状态推进）。

### 3.2 `input.jsonl`（app 追加，脚本 tail）

每行一个 JSON：
```jsonc
{ "contextId": "msg-2", "speakerName": "A", "text": "你好", "audioPath": "%DATA_DIR_PATH%/streaming_123/audio/msg-2.wav" }
```
脚本读到新行 -> 合成 -> 音频写入 `audioPath` -> 分帧流回 -> `finished`。

## 4. stdout 分帧协议（脚本 -> Rust）

脚本 stdout 每行一个 JSON；stderr 经包装器落 task-log。帧类型：

```jsonc
{"type":"started","contextId":"msg-2"}
{"type":"chunk","contextId":"msg-2","bytes":"<base64 PCM/WAV>"}
{"type":"finished","contextId":"msg-2"}
{"type":"error","contextId":"msg-2","message":"..."}
```

Rust `parse_streaming_frame(line) -> StreamingFrame` 解析 -> 映射到既有 `AudioStreamEvent`（`Started` / `Chunk{bytes}` / `Finished` / `Error{message}`，base64 解码为 `Vec<u8>`）-> 经 Channel 下发前端。帧 schema 即契约，纯函数单测覆盖。

**chunk 分帧约定**（对齐前端 `useStreamableAudioPlayer` 将所有 chunk 累积为单个 `Blob(audio/wav)` 的语义，且与既有 `stream_audio_placeholder` 一致）：首条 `chunk` 携带完整 WAV 头 + 首段 PCM，后续 `chunk` 仅含 raw PCM；所有 `chunk` 字节按序拼接即为完整可播 WAV。

## 5. Rust 模块

| 文件 | 职责 |
|------|------|
| `migration/m20260718_000011_add_streaming_tasks.rs` | 建表 + 唯一索引（§2.1） |
| `service/local/entity/streaming_task.rs` | SeaORM Entity，字段对齐 §2.2（仿 `voice_design_task.rs`）；`entity/mod.rs` 注册 |
| `service/models.rs` | `HistoryTaskType::StreamingSpeech`；`CreateStreamingSpeechTaskPayload` / `StreamingSpeechTaskResult` / `SendStreamingMessagePayload` |
| `service/local/streaming.rs` | `create_streaming_speech_task_impl` / `send_streaming_message_impl` / `cancel_streaming_task_impl` + 启动清扫 |
| `service/pipeline/streaming.rs` | 长期进程管理：构造 `PythonScriptInvocationSpec`、spawn `begin_llm_task`、拥有 `child.stdout` 增量读取、帧分发、cancel 即 kill |
| `service/pipeline/api/mod.rs` | 新增 `PythonScriptTaskKind::StreamingSpeech` + `StreamingArgs` |
| `service/pipeline/mod.rs` | `StreamingPipelineRequest { task_id }`；流式 runner 为 `pipeline/streaming.rs` 内独立函数，**不**扩展 `ModelTaskPipeline` trait（长期运行 + 拥有 stdout，不适合一次性 trait 方法形状） |
| `service/local/mod.rs` | `Service` trait 增 3 方法；`StreamingSessionControl`（cancel_tx + child 句柄 + `message_channels`） |
| `hooks/streaming.rs` | 保留 `stream_audio_placeholder`；新增 3 个 `#[tauri::command]` |
| `hooks/mod.rs` | `load_hooks` 注册 3 新命令 |
| `common/task_paths.rs` | 日志前缀分支 + 流式路径辅助（§2.4） |

### 5.1 payload / result 类型

```rust
pub struct CreateStreamingSpeechTaskPayload {
    pub base_model: String, pub model_version: String, pub device: HardwareType,
    pub language: AppLanguage, pub model_params: Value,
    pub speakers: Vec<StreamingSpeakerInput>,   // 镜像前端 StreamingSpeakerConfig
}
pub struct StreamingSpeechTaskResult { pub task_id: i64, pub context_file_path: String, pub input_cache_file_path: String, pub output_audio_dir: String, pub status: TaskStatus, pub created_at: String }
pub struct SendStreamingMessagePayload { pub task_id: i64, pub context_id: String, pub speaker_name: String, pub text: String }
```
（时间字段一律后端生成、不进 payload，与既有 `Create*TaskPayload` 一致。）

### 5.2 `StreamingArgs`（params 文件）

```rust
pub struct StreamingArgs {
    pub context_file_path: String,    // 绝对路径（脚本侧用）
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub speakers: Vec<StreamingSpeakerArg>,
}
```
经 `begin_llm_task --params-file` 透传给脚本；路径字段写绝对路径（脚本非 Rust 侧，无需占位符）。

### 5.3 进程 runner（关键差异）

`run_logged_shell_script_cancellable` 会把 stdout 吃进日志，**不能**用于流式。新增流式专用 runner：

- spawn `begin_llm_task`（脚本侧需透传 stdout，见 §9 契约），以 `Command::stdout(Stdio::piped())` / `stderr(Stdio::piped())` 取管道；
- 启动一个后台 tokio task：`tokio::BufReader::new(child.stdout).lines()` 逐行读 -> `parse_streaming_frame` -> 查 `message_channels[contextId]` 分发（chunk -> `Channel.send(Chunk)`；finished/error -> 转发 + 移除 + `done` oneshot 通知）；
- stderr 异步落 task-log-file（仿既有日志写入）；
- cancel：`watch::Receiver<bool>` 置 true -> `child.kill()` -> 后台 reader 收到 EOF 退出。

### 5.4 `StreamingSessionControl`（活跃会话状态）

流式会话状态随活跃任务控制条目一同存入既有 `active_task_controls`（键为 task_id）：扩展 `ActiveTaskControl` 增加可选的流式附加态

```rust
struct StreamingSessionExtra {
    child_kill_handle: ChildKillHandle,                              // cancel 时 kill 进程
    message_channels: Arc<RwLock<HashMap<String, MessageChannel>>>, // contextId -> { on_event, done }
}
```

cancel 信号**复用既有 `watch::Sender<bool>`**：runner 经 `active_task_cancel_receiver(task_id, StreamingSpeech)` 订阅，置 true 时 `child.kill()`；`cancel_streaming_task` -> `request_active_task_cancel`（与既有取消路径一致）。`send_streaming_message` 在 `message_channels` 注册 `contextId -> { on_event: Channel, done: oneshot::Sender }`，等 `done` oneshot 或 cancel 信号。

## 6. 会话生命周期 + 取消

- **创建** `create_streaming_speech_task`：校验模型/设备（`find_supported_model_variant` + `supported_devices`）-> 事务写 `task_history`(StreamingSpeech, Pending) + `streaming_tasks` -> `ensure_task_sample_dir` 建会话目录 `streaming_<id>` -> 写 `context.json`(basic) -> `serialize_task_path` 存三路径 -> `register_active_task_control` + spawn 长期进程 + 后台 stdout reader -> 置 `Running` -> 返回 `taskId` / 路径。
- **发消息** `send_streaming_message(taskId, contextId, speakerName, text, on_event: Channel)`：校验会话 `Running` -> append `context.json` messages + append `input.jsonl` -> 注册 `message_channels[contextId]` -> 等该 contextId 的 `finished` / `error`（chunk 期间经 Channel 转发）-> 确认 `context.json` 该消息音频已生成（`audioPath` 发送时即已知、不变）、`message_count++` -> 返回。`taskId` / `contextId` 由前端生成（沿用 store 种子），后端用其寻址。
- **强制终止** `cancel_streaming_task(taskId)`：`request_active_task_cancel`（`watch::Sender` 置 true）-> runner `child.kill()` -> 注销控制 -> 置 `Cancelled`。复用 `cancel_task_impl` 的状态校验路径（仅 Pending/Running 可终止）。
- **应用关闭**：进程随应用亡；下次启动 `init_db` 后清扫 `task_type='streaming-speech' AND status='running'` -> `Cancelled`（新增针对性清扫，不影响其他类型）。

## 7. 前端接入

- `stores/streamingSpeech.ts`：`sendMessage` 改异步 -- 首条消息时（无活跃会话）`invoke('create_streaming_speech_task', payload)` 建会话、拿 `taskId` 回填 store；随后消息复用该 `taskId`。`taskId` / `contextId` 仍前端生成。
- `hooks/useStreamableAudioPlayer.ts`：`startStreaming` 改调 `invoke('send_streaming_message', { taskId, contextId, speakerName, text, onEvent })`（替代 `stream_audio_placeholder`）；签名扩为接收 `speakerName` / `text`。Channel `onmessage` 协议不变。
- `views/StreamingSpeechView.vue`：`StreamableAudioPlayer(mode='stream')` 透传 `speakerName` / `text`（取自消息）；新增「终止会话」按钮 -> `invoke('cancel_streaming_task', { taskId })` + 清 store 活跃态（与其他页面取消按钮一致），无活跃会话时禁用。
- `stream_audio_placeholder`：保留，前端不再调用。

## 8. 测试策略（对齐规则4）

不真正拉起脚本/进程。抽取纯函数单测：

- `parse_streaming_frame(line) -> StreamingFrame`（started / chunk / finished / error + 坏行）
- `build_streaming_script_args(...)`（仿 `build_llm_task_script_args`，经 `test_support` 重导出）
- `StreamingArgs` 的 params 文件内容（`pipeline/api/mod.rs` 内部 `#[cfg(test)]`）
- `serialize` / `deserialize_context_json` + `serialize_input_entry`（round-trip）
- 迁移 / 建表用既有 `LocalServiceHarness`（临时库隔离，规则 1&2）

## 9. 不在本次范围

- 真实模型流式脚本 `streaming.py`（各模型子模块）-- 后续任务。
- `begin_llm_task` 流式 stdout 透传改造 / 独立 `begin_streaming_task` 包装器 -- 作为契约定义（脚本侧需让 stdout 透传、stderr 落日志），实现属脚本侧后续。本期 Rust 代码按"spawn `begin_llm_task` 且 stdout 可读"假设编写，spawn 路径不进单测（规则 4）。
- 历史页回放流式会话的 UI（`audioPath` 已持久化，回放可行，但历史视图集成后续）。
- 多并发流式会话（v1 单会话）。
- 远程 `RemoteService` 的流式实现（v0.12.0 remote 仍占位，流式暂不支持）。

## 10. 实现顺序（供 writing-plans 细化）

1. DB：迁移 + `create_local_schema` + `db/tables*.sql` + `HistoryTaskType`/`task_paths` 辅助
2. Entity + models（payload/result/`StreamingArgs`）
3. `service/local/streaming.rs` 创建/发消息/取消/清扫 + `pipeline/streaming.rs` 进程 runner + 帧解析纯函数
4. `Service` trait + hooks + `load_hooks` 注册
5. 前端：store `sendMessage` 异步化 + `useStreamableAudioPlayer` 改调新命令 + 视图终止按钮
6. 单测：帧解析 / args / params 文件 / context.json round-trip / 迁移
7. 手动验证（待脚本侧就绪后）
