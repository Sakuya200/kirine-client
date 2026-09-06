---
name: streaming-speech-architecture
description: 流式语音生成功能的前后端完整架构（会话生命周期、环回 Socket 二进制帧协议、长期进程 runner、并发模型、清扫、远程不支持）
metadata: 
  node_type: memory
  type: project
  originSessionId: f3951a22-6e9b-44c0-90cf-5e1c985197d8
---

# 流式语音生成（StreamingSpeech）架构

> 状态截至 2026-09-06 · 分支 `v0.12.2`

区别于 TTS/克隆/设计的「一次性脚本跑完出文件」，流式语音是**会话级长期进程**：一个 task 拉起一个常驻 `streaming.py`，多条聊天消息复用同一进程，音频以**裸 PCM 二进制帧**经环回 TCP Socket（Python→Rust）与 Tauri IPC Channel Raw 路径（Rust→前端）实时下发，全程无 base64/JSON 音频编码。

## 任务类型与 DB

- `HistoryTaskType::StreamingSpeech`（`service/models.rs`）：`as_str()` = `"streaming-speech"`，`storage_dir()` = `"streaming"`。前端 `enums/task.ts` 同步（见 [[history-task-type-sync-rule]]）。
- 新表 `streaming_tasks`（`service/local/entity/streaming_task.rs`）：`id / history_id / base_model / model_version / language / device / model_params_json / context_file_path / input_cache_file_path / output_audio_dir / message_count / create_time / modify_time / deleted`。与 `task_history` 经 `history_id` 关联。
- 迁移 `m20260718_000011_add_streaming_tasks.rs`；`LOCAL_SCHEMA_VERSION` 27 -> **28**。`db/tables.sql` + `db/tables_pgsql.sql` 同步（见 [[db-schema-sync-rule]]）。

## Hooks 层（`hooks/streaming.rs`）

- `AudioStreamEvent` enum（仅控制事件）：`Started / Finished / Error{message}`，`#[serde(tag="type", rename_all="camelCase")]` -> JSON `{type:"started"|"finished"|"error", message?}`。chunk 不在枚举中——音频以二进制直接下发。
- `send_streaming_message` 的 `on_event: Channel<tauri::ipc::InvokeResponseBody>`：控制事件 `InvokeResponseBody::Json`（serde JSON 字符串），chunk 为 `InvokeResponseBody::Raw(Vec<u8>)`。>1024 字节载荷走 Tauri 2.10 fetch 快速路径，JS `channel.onmessage` 收到 **ArrayBuffer**（无 JSON number[] ~4 倍膨胀）。
- 6 个命令：`create_streaming_speech_task` / `send_streaming_message`（带 `on_event: Channel`）/ `cancel_streaming_task` / `get_streaming_replay_snapshot`（历史回放恢复）/ `get_streaming_speaker_avatar` / `update_streaming_speaker_avatar`（说话人头像读取与替换），注册于 `hooks/mod.rs`。

## Service trait（`service/mod.rs`）

含 6 个流式方法：`create_streaming_speech_task` / `send_streaming_message` / `cancel_streaming_task` / `get_streaming_replay_snapshot` / `read_streaming_speaker_avatar` / `update_streaming_speaker_avatar`（Service trait 共 27 业务方法）。`RemoteService` 的流式五方法（含回放与头像）均 `bail!("远程存储模式暂不支持流式语音…")`（Remote 模式不支持流式；create/send/cancel 三方法为既有 bail）。

## LocalService 会话层（`service/local/streaming.rs`）

- `create_streaming_speech_task_impl`：校验模型/设备/说话人 -> 事务写 `task_history`(Pending) + `streaming_tasks` -> `copy_model_param_files` -> 写初始 `context.json` -> `register_streaming_session` + `start_streaming_session`。**启动失败回滚运行句柄并把任务置 Failed**（避免卡 Pending 误导前端）。
- `send_streaming_message_impl`：校验任务 Running 且为 StreamingSpeech -> 取 detail 解析音频目录 -> **先注册 Channel 再喂入输入**（防终帧早到丢失）-> `context_lock` 串行化 `context.json` read-modify-write + append `input.jsonl`（审计日志）+ 经 `StreamingSessionExtra::send_input_frame` 推送 INPUT 二进制帧 -> 等 `oneshot` 终帧信号（`STREAMING_MESSAGE_TIMEOUT = 300s`）-> 成功自增 `message_count`。超时/进程退出经 `error_event` 下发 Error，避免前端永久转圈。
- `cancel_streaming_task_impl`：复用 `request_active_task_cancel`（`watch::Sender` -> runner kill child）。
- `sweep_stale_streaming_sessions_impl` / `sweep_stale_streaming_sessions_on_orm`：`init_db` 时把残留 Running 流式会话标 Cancelled（仅扫 StreamingSpeech，不误伤其它 Running 任务）。
- `register_streaming_session` / `streaming_session_extra` / `load_streaming_task_detail` / `start_streaming_session`：会话运行句柄管理。`start_streaming_session` spawn runner，退出路径兜底置 Failed。
- `ActiveTaskControl`（`service/local/mod.rs`）含 `streaming_extra: Option<Arc<StreamingSessionExtra>>` 字段。

## Pipeline 层（`service/pipeline/streaming.rs`）- 核心 runner

- **帧解析纯函数**：`parse_streaming_frame`（控制帧 JSON -> `StreamingFrame`）、`frame_to_event`（-> `InvokeResponseBody`：Chunk->`Raw`、控制->`Json`）、`error_event`（本地构造 error 事件）、`serialize_input_entry`。`StreamingFramePayload` = Started/Chunk/Finished/Error。帧源是环回 Socket 二进制帧（非 stdout、非文件），见下「帧协议 / 传输」。
- **类型**：`StreamingContextJson`/`StreamingContextBasic`/`StreamingSpeaker`/`StreamingMessageEntry`（context.json 结构）；`ResolvedStreamingPaths`（含 `base_model`/`model_version`/`sample_root`/`model_root_path` 等待用字段，见 [[retain-future-use-fields]]）；`LoadedStreamingDetail`（含 `model_params`）。
- `StreamingSpeaker`/`StreamingSpeakerArg`/`StreamingSpeakerInput` 扩展 `category`（"voice-clone"|"trained"，缺省视为 voice-clone）+ `speaker_dir_name: Option<String>`（trained=speaker_id）+ `side`（"left"|"right"，缺省视为 right）+ `avatar_path`/`avatar_name`（头像原图绝对路径与原始文件名，缺省 None）；`StreamingArgs` 含 `model_root_path`（= service.model_dir()）+ `model_params_json`（流式 UI 参数透传至 Python）+ `streaming_socket_addr`/`streaming_socket_token`（环回 Socket 端点与鉴权令牌，`#[serde(default)]`）。
- `resolve_streaming_paths` + `build_streaming_invocation`（`PythonScriptTaskKind::StreamingSpeech` + `StreamingArgs`，写 `streaming.params.json`；签名含 `socket_addr`/`socket_token`/`model_params: serde_json::Value`，填 `model_root_path` + 映射 speaker 新字段）。
- `StreamingSessionExtra`：`message_channels: Arc<RwLock<HashMap<String, MessageChannel>>>` + `context_lock: Arc<Mutex<()>>` + `input_tx: Arc<std::sync::Mutex<Option<mpsc::Sender<Vec<u8>>>>>`（向 Python 推 input 帧的通道句柄），存于 `ActiveTaskControl.streaming_extra`，runner 与 send 共享。
- `MessageChannel { on_event: Channel<InvokeResponseBody>, done: oneshot::Sender<StreamMessageOutcome> }`；`StreamMessageOutcome { cancelled, errored }`。
- `run_streaming_session`：spawn `begin_llm_task` -> `streaming.py`（参数含 Socket 地址/令牌）-> bind `TcpListener("127.0.0.1:0")` 随机端口 -> `timeout(120s)` accept -> 首帧 AUTH 令牌校验（失败/超时 `cleanup_child_after_transport_error` 进程树强杀，防孤儿）-> `into_split` 后 **writer task**（`streaming_socket_writer` 消费 mpsc<Vec<u8>> 逐帧写 Socket）+ **reader task**（`streaming_socket_reader` 按 5 字节头分帧 `read_exact`，EOF 投递 None）-> `drive_streaming_socket` 主循环（`select! { frame_rx.recv() | cancel_rx.changed() | sleep(2s) child.try_wait() 探活 }`；EOF 后 `timeout(5s, child.wait())` 超时强杀标 Failed；cancel 时 `drain_buffered_frames` 排空残余帧）-> `close_input_sender` + `drain_pending_channels` + await tasks。cancel 信号 kill 子进程；状态机 **Running(spawn 后) -> Cancelled/Failed/Exited(退出)**，进程自然退出视为 Exited（映射 Cancelled 终态）。子进程 stdout/stderr 均 null——stderr 经 PS1 `2>&1 | Out-File` 落 task-log（PS1 独占写 task log，Rust 不持有该句柄，消除写-写争用）。
- `forward_event`：**终帧**（Finished/Error）写锁 remove channel + `on_event.send` + `done.send`；**非终帧**（Chunk）读锁 `on_event.send`（避免高频 chunk 写锁竞争）。

## Socket 帧编解码（`service/pipeline/streaming_transport.rs`）

纯函数、无 I/O：`encode_frame(kind, payload)`（`[u32 LE len(kind+payload)][kind u8][payload]`）、`encode_control_frame`/`encode_input_frame`（JSON）、`encode_chunk_frame(context_id, bytes)`（`[u16 LE idLen][contextId UTF-8][音频字节]`）、`decode_frame_header`、`parse_chunk_payload`、`parse_auth_payload`、`generate_session_token`（rand 32 字节 hex）。常量 `FRAME_KIND_AUTH=0x01 / CONTROL=0x02 / CHUNK=0x03 / INPUT=0x10`、`MAX_FRAME_PAYLOAD=16MB`。

## 帧协议 / 传输（环回 Socket 二进制协议）

**Python ↔ Rust**：会话进程启动时 Rust bind `127.0.0.1` 随机端口（环回绑定不触发 Windows 防火墙弹窗），地址与 32 字节随机 hex 令牌随 `streaming.params.json` 传给脚本。Python `connect_session_socket` 在**模型加载前**连接并发首帧 AUTH（`{"token"}`，JSON）；Rust 校验失败强杀进程。之后双向二进制帧（编解码见上节）：

- **CHUNK 0x03**（Python→Rust）：payload = `contextId 长度前缀 + contextId + 音频裸字节`，首 chunk 前置 44 字节 WAV 哨兵头（data size=0xFFFFFFFF，浏览器按实际到达数据解码、支持半截播放），后续 chunk 为裸 PCM16——**无 base64、无 JSON 包裹**。
- **CONTROL 0x02**（Python→Rust）：payload 为 JSON `{type:"started"|"finished"|"error"|"session_ready", contextId, message?}`（低频小体积保留可读性）；Rust `parse_streaming_frame` 解析、`frame_to_event` 映射为 `InvokeResponseBody` 下发前端（`session_ready` 不下发）。
- **INPUT 0x10**（Rust→Python）：payload 为 JSON `{contextId, speakerName, text, audioPath}`；`send_streaming_message_impl` 经 `StreamingSessionExtra::send_input_frame` 推送（`mpsc` 容量 128，`try_send`）。Python 主循环阻塞 `read_frame`，非 INPUT 帧 skip，EOF 退出。
- `contextId` 由前端生成，多路复用同一会话进程。

**Rust → 前端**：`Channel<InvokeResponseBody>`，控制事件 Json、chunk Raw（>1024 字节走 Tauri fetch 快速路径），JS 收 ArrayBuffer。

**历史演进**（为何是 Socket）：第一代走 stdout 有 stderr 写-写争用与 chunk 丢帧缺陷；第二代改为 `frames.jsonl` 缓冲文件轮询（20ms poll、base64 编码），解决正确性但引入磁盘 I/O 轮询延迟 + base64 33% 膨胀 + 双重编解码开销；第三代（当前）环回 TCP 二进制帧消除全部中间编码与轮询。`input.jsonl` 仍保留 append 写入（仅作审计日志，Python 不再轮询它，输入走 INPUT 帧）。`context.json`/音频目录产物语义不变。

## 并发与一致性模型

- **多路分发**：`message_channels` 按 `contextId` 索引，同一会话并发发送多条消息时各走各的 Channel。
- **context.json 一致性**：`context_lock`（每会话一把 `Mutex`）串行化 read-modify-write，防后写覆盖前写丢消息条目。
- **input.jsonl**：append-only，无需锁；实际输入经 `input_tx`（`mpsc`，天然串行）以 INPUT 帧下发，jsonl 仅审计。
- **路径穿越防御**：`context_id` 经 `sanitize_path_segment` 清洗后再 join 音频文件名（当前固定 `msg-N` 已安全，纵深防御）；channel key 与 context.json 的 `contextId` 仍用原值对齐脚本回传帧。

## 会话产物路径（`common/task_paths.rs`，sample_dir 下）

`streaming_context_json_path`（context.json）/ `streaming_input_cache_path`（input.jsonl，Rust->Python 输入审计日志；实际输入经 Socket INPUT 帧下发）/ `streaming_output_audio_dir`（按 contextId 分音频文件）/ `streaming.params.json`（含 `streaming_socket_addr`/`streaming_socket_token`）/ task-log-file。`history.rs::load_streaming_detail` 支持历史回放 StreamingSpeech 详情。

## 前端与历史回放（详见 [[tech-stack-frontend]] / [[data-flow-and-types]]）

`StreamingSpeechView`（ChatUI，消息行由 `StreamingMessageItem` 渲染，见上「说话人头像与消息侧别」）+ `StreamingConfigDrawer` + `StreamingSpeakerForm` + `StreamableAudioPlayer`(mode='stream') + `useStreamableAudioPlayer`。`stores/streamingSpeech.ts`：首条消息 `invoke create_streaming_speech_task` 拿 taskId 回填，后续 `invoke send_streaming_message`（带 Channel），取消 `invoke cancel_streaming_task`；防连点产生僵尸会话。**chunk 接收路径**：`Channel` 的 `onmessage` 收到 **ArrayBuffer**（Chunk Raw 路径）时以 `new Uint8Array(message)` 引用追加进 `audioBuffers: Map<messageId, Uint8Array[]>`（无逐 chunk 拷贝）；`getAudioUrl` 用 `new Blob(chunks, {type:'audio/wav'})` 聚合（Blob 接受分块数组，无需拼接单缓冲），URL 缓存按 `chunks.length` 失效，`length === -1` 表示历史文件 URL。`StreamingSpeakerCategory` 现支持 `voice-clone`（前端本地，ref 音频+台词）与 `trained`（经 `StreamingSpeakerForm` 从 `list_speaker_infos`(status=Ready) 按 baseModel 过滤选择，存 `speakerDirName=speaker.id`）两类；payload speakers 携带 `category` + `speakerDirName`。

历史会话由 `get_streaming_replay_snapshot(historyId)` 恢复消息和配置。历史消息的音频播放与另存为统一使用 `GeneratedAudioSource::StreamingSpeech { historyId, messageId }`：前端调用 `get_generated_audio` 取得字节、创建 Blob URL 并缓存到组件卸载；下载调用 `save_generated_audio_as`。该回放路径按持久化的历史和消息 ID 读取文件，不依赖已退出的长期会话进程或 WebView 本地文件 URL。

## 说话人头像与消息侧别（v0.12.2）

- **存储**：创建任务时把头像原图复制进任务 sample 目录，命名 `avatar_{idx}_{speakerName}.{ext}`，序列化路径写入 context.json 说话人条目（`avatar_path`/`avatar_name`）；`StreamingSpeakerInput`/`StreamingSpeaker`（context.json）均含 `side`/`avatar_path`/`avatar_name`。扩展名白名单 `STREAMING_AVATAR_IMAGE_EXTENSIONS`（png/jpg/jpeg/webp/gif，与前端 `IMAGE_FILE_EXTENSIONS` 对齐）；content-type 由扩展名推断。
- **读取**：`read_streaming_speaker_avatar(historyId, speakerName)` -> `StreamingSpeakerAvatarAsset { historyId, speakerName, fileName, contentType, bytes }`（图片字节资产，语义为图片，不复用音频资产类型）。替换：`update_streaming_speaker_avatar(historyId, speakerName, avatarPath?, avatarName?)` 复制新图入 sample 目录、更新 context.json、清理旧头像文件（无参传入即清除头像）。
- **前端**：`stores/streamingSpeech.ts` 维护 `avatarUrls: Map<taskId:speakerName, Blob URL>` 缓存与 `avatarCacheVersion`（`clearAvatarUrls` 递增，驱动头像组件 watch 重载）；`ensureSpeakerAvatar` 失败或无头像缓存空串（组件回退首字占位）。说话人改头像时 invoke `update_streaming_speaker_avatar` 后清缓存。
- **展示**：消息行由 `StreamingMessageItem` 渲染（头像 `StreamingMessageAvatar`（Blob URL + 首字占位回退）/ 昵称 / 状态标签 `StreamingMessageStatusPill` / 气泡 / 播放器），列表 `<TransitionGroup name="message-stack">` 进出场动画（transform+opacity，不动画高度）；`compact` prop 隐藏状态标签与播放/下载区。

## trained 说话人回接（moss_tts_realtime 首例）

- **moss_tts_realtime 是首个实现会话级 `streaming.py` 契约的适配器**（此前流式功能 Rust+前端就绪但无模型实现该契约，流式下拉为空、端到端未跑通）。当前契约为环回 Socket 二进制帧：`connect_session_socket`（模型加载前连接+AUTH）→ `session_ready` → 阻塞 `read_frame` 收 INPUT 帧 → `emit_chunk`/`emit_frame` 回传；帧编解码函数与 Rust `streaming_transport.rs` 布局逐字节对齐（Python 测试 `tests/test_wav_frames.py`/`test_streaming_error_logging.py` 覆盖）。
- trained 说话人经 `speakerStore` 体系回接：微调产出 `<model_root_path>/<speaker_dir_name>/checkpoint_final/`，streaming.py 启动时扫 speakers，存在 `category=="trained"` -> 加载该 checkpoint（否则基座）。**约束：一会话至多一个 trained 说话人**（`create_streaming_speech_task_impl` 校验；trained 须有 speakerDirName，voice-clone 须有 refAudioPath）。
- voice-clone 说话人仍纯前端本地，不接 speakerStore；仅 trained 走 speakerStore。
- Remote 模式流式本就不支持，trained 回接仅 Local。

## 测试（`src-tauri/tests/`）

- `streaming_frames.rs`：控制帧解析与 `frame_to_event` 映射（Json/Raw）、context.json 往返、input entry 序列化，12 用例。
- `streaming_transport.rs`：Socket 帧编解码（长度前缀布局、往返、截断/超限拒绝、多字节 contextId、auth payload、token 唯一性），13 用例。
- `streaming_schema.rs`：`streaming_tasks` 表 14 列存在性 + `StreamingSpeech.as_str()`/`storage_dir()`。
- `streaming_contract.rs`：`StreamingArgs`/`StreamingSpeakerArg`/`StreamingSpeaker` serde 往返（含 `model_root_path`/`streaming_socket_addr`/`streaming_socket_token`/`model_params_json`/`category`/`speaker_dir_name` 字段，及缺省回填）。
- `streaming_service.rs`：启动清扫--残留 Running 流式会话标 Cancelled，且不误伤非 streaming Running 任务。
- `streaming_hooks.rs`：`AudioStreamEvent` serde 协议（tag/camelCase）单测。
- Python `src-model/moss_tts_realtime/tests/`：`test_wav_frames.py`（WAV 哨兵头/emit_chunk 帧布局/encode_frame/read_frame 含 socketpair 分次到达）+ `test_streaming_error_logging.py`（error 控制帧 + SocketFrameSink 长度前缀）。

## 关联记忆

- [[data-flow-and-types]] [[tech-stack-backend]] [[tech-stack-frontend]] [[history-task-type-sync-rule]] [[db-schema-sync-rule]] [[retain-future-use-fields]] [[tests-dir-over-inline]] [[time-field-naming-rule]]
