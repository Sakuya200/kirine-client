---
name: streaming-speech-architecture
description: 流式语音生成功能的前后端完整架构（会话生命周期、帧协议、长期进程 runner、并发模型、清扫、远程不支持）
metadata: 
  node_type: memory
  type: project
  originSessionId: f3951a22-6e9b-44c0-90cf-5e1c985197d8
---

# 流式语音生成（StreamingSpeech）架构

> 状态截至 2026-07-21 · 分支 `v.0.12.0`

区别于 TTS/克隆/设计的「一次性脚本跑完出文件」，流式语音是**会话级长期进程**：一个 task 拉起一个常驻 `streaming.py`，多条聊天消息复用同一进程，音频按 chunk 实时经 Tauri IPC Channel 下发前端播放。

## 任务类型与 DB

- `HistoryTaskType::StreamingSpeech`（`service/models.rs`）：`as_str()` = `"streaming-speech"`，`storage_dir()` = `"streaming"`。前端 `enums/task.ts` 同步（见 [[history-task-type-sync-rule]]）。
- 新表 `streaming_tasks`（`service/local/entity/streaming_task.rs`）：`id / history_id / base_model / model_version / language / device / model_params_json / context_file_path / input_cache_file_path / output_audio_dir / message_count / create_time / modify_time / deleted`。与 `task_history` 经 `history_id` 关联。
- 迁移 `m20260718_000011_add_streaming_tasks.rs`；`LOCAL_SCHEMA_VERSION` 27 -> **28**。`db/tables.sql` + `db/tables_pgsql.sql` 同步（见 [[db-schema-sync-rule]]）。

## Hooks 层（`hooks/streaming.rs`）

- `AudioStreamEvent` enum：`Started / Chunk{bytes} / Finished / Error{message}`，`#[serde(tag="type", rename_all="camelCase")]` -> JSON `{type:"started"|"chunk"|"finished"|"error", bytes?, message?}`。前端经 `ipc::Channel<AudioStreamEvent>` 订阅。
- 3 个命令：`create_streaming_speech_task` / `send_streaming_message`（带 `on_event: Channel`）/ `cancel_streaming_task`，注册于 `hooks/mod.rs`。

## Service trait（`service/mod.rs`）

含 3 个流式方法：`create_streaming_speech_task` / `send_streaming_message` / `cancel_streaming_task`（Service trait 共 24 业务方法）。`RemoteService` 三者均 `bail!("远程存储模式暂不支持流式语音会话")`（Remote 模式不支持流式）。

## LocalService 会话层（`service/local/streaming.rs`）

- `create_streaming_speech_task_impl`：校验模型/设备/说话人 -> 事务写 `task_history`(Pending) + `streaming_tasks` -> `copy_model_param_files` -> 写初始 `context.json` -> `register_streaming_session` + `start_streaming_session`。**启动失败回滚运行句柄并把任务置 Failed**（避免卡 Pending 误导前端）。
- `send_streaming_message_impl`：校验任务 Running 且为 StreamingSpeech -> 取 detail 解析音频目录 -> **先注册 Channel 再喂入输入**（防终帧早到丢失）-> `context_lock` 串行化 `context.json` read-modify-write + append `input.jsonl` -> 等 `oneshot` 终帧信号（`STREAMING_MESSAGE_TIMEOUT = 300s`）-> 成功自增 `message_count`。超时/进程退出经 Channel 下发 Error，避免前端永久转圈。
- `cancel_streaming_task_impl`：复用 `request_active_task_cancel`（`watch::Sender` -> runner kill child）。
- `sweep_stale_streaming_sessions_impl` / `sweep_stale_streaming_sessions_on_orm`：`init_db` 时把残留 Running 流式会话标 Cancelled（仅扫 StreamingSpeech，不误伤其它 Running 任务）。
- `register_streaming_session` / `streaming_session_extra` / `load_streaming_task_detail` / `start_streaming_session`：会话运行句柄管理。`start_streaming_session` spawn runner，退出路径兜底置 Failed。
- `ActiveTaskControl`（`service/local/mod.rs`）含 `streaming_extra: Option<Arc<StreamingSessionExtra>>` 字段。

## Pipeline 层（`service/pipeline/streaming.rs`）- 核心 runner

- **帧解析纯函数**：`parse_streaming_frame`（stdout JSON 行 -> `StreamingFrame`）、`frame_to_event`（-> `AudioStreamEvent`）、`serialize_input_entry`。`StreamingFramePayload` = Started/Chunk/Finished/Error。
- **类型**：`StreamingContextJson`/`StreamingContextBasic`/`StreamingSpeaker`/`StreamingMessageEntry`（context.json 结构）；`ResolvedStreamingPaths`（含 `base_model`/`model_version`/`sample_root`/`model_root_path` 等待用字段，见 [[retain-future-use-fields]]）；`LoadedStreamingDetail`（含 `model_params`）。
- `StreamingSpeaker`/`StreamingSpeakerArg`/`StreamingSpeakerInput` 扩展 `category`（"voice-clone"|"trained"，缺省视为 voice-clone）+ `speaker_dir_name: Option<String>`（trained=speaker_id）；`StreamingArgs` 扩展 `model_root_path`（= service.model_dir()）+ `model_params_json`（流式 UI 参数透传至 Python）。
- `resolve_streaming_paths` + `build_streaming_invocation`（`PythonScriptTaskKind::StreamingSpeech` + `StreamingArgs`，写 `streaming.params.json`；签名含 `model_params: serde_json::Value`，填 `model_root_path` + 映射 speaker 新字段）。
- `StreamingSessionExtra`：`message_channels: Arc<RwLock<HashMap<String, MessageChannel>>>` + `context_lock: Arc<Mutex<()>>`，存于 `ActiveTaskControl.streaming_extra`，runner 与 send 共享。
- `MessageChannel { on_event: Channel, done: oneshot::Sender<StreamMessageOutcome> }`；`StreamMessageOutcome { cancelled, errored }`。
- `run_streaming_session`：spawn `begin_llm_task` -> `streaming.py`，逐行读 stdout 分帧按 `contextId` 分发；cancel 信号 kill 子进程；状态机 **Running(spawn 后) -> Cancelled/Failed(退出)**，进程自然退出视为 Cancelled；stderr 异步落 task-log；退出前 `drain_pending_channels` 通知所有 pending 消息。
- `drive_streaming_stdout`：`tokio::select! { cancel_rx.changed() | reader.next_line() }`，biased 优先 cancel；始终 `child.wait()` reap 防僵尸。
- `forward_event`：**终帧**（Finished/Error）写锁 remove channel + `on_event.send` + `done.send`；**非终帧**（Chunk）读锁 `on_event.send`（避免高频 chunk 写锁竞争）。

## 帧协议

脚本 stdout 每行一个 JSON：`{type:"started"|"chunk"|"finished"|"error", contextId, bytes?(base64), message?}` ↔ Rust `StreamingFrame` ↔ `AudioStreamEvent`（serde tag=type camelCase）下发给前端。`contextId` 由前端生成，多路复用同一会话进程。

## 并发与一致性模型

- **多路分发**：`message_channels` 按 `contextId` 索引，同一会话并发发送多条消息时各走各的 Channel。
- **context.json 一致性**：`context_lock`（每会话一把 `Mutex`）串行化 read-modify-write，防后写覆盖前写丢消息条目。
- **input.jsonl**：append-only，无需锁。
- **路径穿越防御**：`context_id` 经 `sanitize_path_segment` 清洗后再 join 音频文件名（当前固定 `msg-N` 已安全，纵深防御）；channel key 与 context.json 的 `contextId` 仍用原值对齐脚本回传帧。

## 会话产物路径（`common/task_paths.rs`，sample_dir 下）

`streaming_context_json_path`（context.json）/ `streaming_input_cache_path`（input.jsonl）/ `streaming_output_audio_dir`（按 contextId 分音频文件）/ `streaming.params.json` / task-log-file。`history.rs::load_streaming_detail` 支持历史回放 StreamingSpeech 详情。

## 前端（详见 [[tech-stack-frontend]] / [[data-flow-and-types]]）

`StreamingSpeechView`（ChatUI）+ `StreamingConfigDrawer` + `StreamingSpeakerForm` + `StreamableAudioPlayer`(mode='stream') + `useStreamableAudioPlayer`。`stores/streamingSpeech.ts`：首条消息 `invoke create_streaming_speech_task` 拿 taskId 回填，后续 `invoke send_streaming_message`（带 Channel），取消 `invoke cancel_streaming_task`；防连点产生僵尸会话。`StreamingSpeakerCategory` 现支持 `voice-clone`（前端本地，ref 音频+台词）与 `trained`（经 `StreamingSpeakerForm` 从 `list_speaker_infos`(status=Ready) 按 baseModel 过滤选择，存 `speakerDirName=speaker.id`）两类；payload speakers 携带 `category` + `speakerDirName`。

## trained 说话人回接（moss_tts_realtime 首例）

- **moss_tts_realtime 是首个实现会话级 `streaming.py` 契约的适配器**（此前流式功能 Rust+前端就绪但无模型实现该契约，流式下拉为空、端到端未跑通）。
- trained 说话人经 `speakerStore` 体系回接：微调产出 `<model_root_path>/<speaker_dir_name>/checkpoint_final/`，streaming.py 启动时扫 speakers，存在 `category=="trained"` -> 加载该 checkpoint（否则基座）。**约束：一会话至多一个 trained 说话人**（`create_streaming_speech_task_impl` 校验；trained 须有 speakerDirName，voice-clone 须有 refAudioPath）。
- voice-clone 说话人仍纯前端本地，不接 speakerStore；仅 trained 走 speakerStore。
- Remote 模式流式本就不支持，trained 回接仅 Local。

## 测试（`src-tauri/tests/`）

- `streaming_frames.rs`：帧解析 9 用例（started/chunk/finished/error/blank/malformed/unknown_type/context_json_round_trip/input_entry + `frame_to_event` 映射）。
- `streaming_schema.rs`：`streaming_tasks` 表 14 列存在性 + `StreamingSpeech.as_str()`/`storage_dir()`。
- `streaming_contract.rs`：`StreamingArgs`/`StreamingSpeakerArg`/`StreamingSpeaker` serde 往返（含 `model_root_path`/`model_params_json`/`category`/`speaker_dir_name` 新字段，及缺省回填）。
- `streaming_service.rs`：启动清扫--残留 Running 流式会话标 Cancelled，且不误伤非 streaming Running 任务。
- `streaming_hooks.rs`：`AudioStreamEvent` serde 协议（tag/camelCase）单测。

## 关联记忆
- [[data-flow-and-types]] [[tech-stack-backend]] [[tech-stack-frontend]] [[history-task-type-sync-rule]] [[db-schema-sync-rule]] [[retain-future-use-fields]] [[tests-dir-over-inline]] [[time-field-naming-rule]]
