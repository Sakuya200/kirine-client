use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use tauri::ipc::{Channel, InvokeResponseBody};
use tokio::{
    net::{tcp::OwnedReadHalf, tcp::OwnedWriteHalf, TcpListener},
    process::Child,
    sync::{mpsc, oneshot, watch, Mutex},
    time::timeout,
};
use tracing::{error, info, warn};

use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{
            streaming_context_json_path, streaming_input_cache_path, streaming_output_audio_dir,
            task_log_file_path, task_sample_dir,
        },
    },
    config::HardwareType,
    hooks::streaming::AudioStreamEvent,
    service::{
        local::LocalService,
        models::{HistoryTaskType, StreamingSpeakerInput, TaskStatus, UpdateTaskStatusPayload},
        pipeline::{
            api::{
                PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
                PythonScriptTaskKind, StreamingArgs, StreamingSpeakerArg,
            },
            build_llm_task_script_args,
            script_paths::{
                resolve_src_model_root, src_model_begin_llm_task_script_path,
                src_model_model_python_script_path, ScriptPlatform,
            },
            streaming_transport::{
                decode_frame_header, generate_session_token, parse_auth_payload,
                parse_chunk_payload, FRAME_HEADER_LEN, FRAME_KIND_AUTH, FRAME_KIND_CHUNK,
                FRAME_KIND_CONTROL,
            },
            StreamingPipelineRequest,
        },
    },
    utils::process::{append_task_log_text, spawn_logged_child_with_stderr},
    Result,
};

/// 流式会话环回 Socket 帧承载的一条消息（Python -> Rust）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingFrame {
    pub context_id: String,
    pub payload: StreamingFramePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamingFramePayload {
    Started,
    SessionReady,
    Chunk { bytes: Vec<u8> },
    Finished,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingContextJson {
    pub basic: StreamingContextBasic,
    pub messages: Vec<StreamingMessageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingContextBasic {
    pub task_id: i64,
    pub base_model: String,
    pub model_version: String,
    pub device: String,
    pub language: String,
    pub speakers: Vec<StreamingSpeaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingSpeaker {
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
    /// trained 说话人 = speaker_id；voice-clone 为 None。
    #[serde(default)]
    pub speaker_dir_name: Option<String>,
    /// "voice-clone" | "trained"；缺省视为 "voice-clone"。
    #[serde(default)]
    pub category: String,
    /// 消息展示侧："left" | "right"；空串/缺省视为 "right"。
    #[serde(default)]
    pub side: String,
    /// 头像路径（任务创建时复制进 sample 目录后的 %DATA_DIR_PATH% 序列化路径）。
    #[serde(default)]
    pub avatar_path: Option<String>,
    /// 头像原始文件名（展示用）。
    #[serde(default)]
    pub avatar_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingMessageEntry {
    pub context_id: String,
    pub speaker_name: String,
    pub text: String,
    pub audio_path: String,
}

pub fn serialize_input_entry(
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
    message: Option<String>,
}

/// 解析控制帧 payload（JSON 文本）。空文本返回 `Ok(None)`；坏行返回 `Err`。
/// chunk 不经此路径（二进制帧，见 `streaming_transport::parse_chunk_payload`）。
pub fn parse_streaming_frame(line: &str) -> Result<Option<StreamingFrame>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let raw: RawFrame = serde_json::from_str(trimmed).context("failed to parse streaming frame")?;
    let payload = match raw.kind.as_str() {
        "started" => StreamingFramePayload::Started,
        "session_ready" => StreamingFramePayload::SessionReady,
        "finished" => StreamingFramePayload::Finished,
        "error" => StreamingFramePayload::Error {
            message: raw.message.unwrap_or_default(),
        },
        other => anyhow::bail!("unknown streaming frame type: {other}"),
    };
    Ok(Some(StreamingFrame {
        context_id: raw.context_id,
        payload,
    }))
}

/// 将帧映射为下发前端的 IPC 载荷：控制事件走 JSON，chunk 走 `Raw` 二进制
/// （前端收到 ArrayBuffer，>1KB 由 Tauri 经 fetch 快速路径回传，避免 JSON
/// 数字数组 ~4 倍膨胀与逐字节物化）。
pub fn frame_to_event(frame: &StreamingFrame) -> Option<InvokeResponseBody> {
    match &frame.payload {
        StreamingFramePayload::Started => Some(json_event(&AudioStreamEvent::Started)),
        StreamingFramePayload::SessionReady => None,
        StreamingFramePayload::Chunk { bytes } => Some(InvokeResponseBody::Raw(bytes.clone())),
        StreamingFramePayload::Finished => Some(json_event(&AudioStreamEvent::Finished)),
        StreamingFramePayload::Error { message } => Some(json_event(&AudioStreamEvent::Error {
            message: message.clone(),
        })),
    }
}

fn json_event(event: &AudioStreamEvent) -> InvokeResponseBody {
    InvokeResponseBody::Json(serde_json::to_string(event).unwrap_or_default())
}

/// 构造 error 控制事件的 IPC 载荷（会话层超时/收尾路径直接下发前端用）。
pub fn error_event(message: String) -> InvokeResponseBody {
    json_event(&AudioStreamEvent::Error { message })
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedStreamingPaths {
    pub base_model: String,
    pub model_version: String,
    pub src_model_root: PathBuf,
    pub begin_llm_task_script_path: PathBuf,
    pub streaming_python_script_path: PathBuf,
    pub sample_root: PathBuf,
    pub context_json_path: PathBuf,
    pub input_cache_path: PathBuf,
    pub output_audio_dir: PathBuf,
    /// = service.model_dir()，trained 说话人 checkpoint 解析所需。
    pub model_root_path: PathBuf,
    pub params_json_path: PathBuf,
}

pub(crate) fn resolve_streaming_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_version: &str,
) -> Result<ResolvedStreamingPaths> {
    let src_model_root = resolve_src_model_root(service.app_dir())?;
    let begin_llm_task_script_path = src_model_begin_llm_task_script_path(&src_model_root);
    let streaming_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "streaming.py")?;
    let sample_root = task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::StreamingSpeech,
        task_id,
    );
    let context_json_path = streaming_context_json_path(&sample_root);
    let input_cache_path = streaming_input_cache_path(&sample_root);
    let output_audio_dir = streaming_output_audio_dir(&sample_root);
    let params_json_path = sample_root.join("streaming.params.json");

    Ok(ResolvedStreamingPaths {
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        src_model_root,
        begin_llm_task_script_path,
        streaming_python_script_path,
        sample_root,
        context_json_path,
        input_cache_path,
        output_audio_dir,
        model_root_path: PathBuf::from(service.model_dir()),
        params_json_path,
    })
}

pub(crate) fn build_streaming_invocation(
    paths: &ResolvedStreamingPaths,
    base_model: &str,
    model_version: &str,
    device: HardwareType,
    model_params: serde_json::Value,
    speakers: Vec<StreamingSpeakerInput>,
    socket_addr: &str,
    socket_token: &str,
) -> PythonScriptInvocationSpec {
    PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        kind: PythonScriptTaskKind::StreamingSpeech,
        runtime: PythonScriptRuntimeOptions {
            device: Some(device.runtime_arg().to_string()),
            logging_dir: None,
            attn_implementation: None,
        },
        args: PythonScriptTaskArgs::Streaming(StreamingArgs {
            context_file_path: paths.context_json_path.to_string_lossy().to_string(),
            input_cache_file_path: paths.input_cache_path.to_string_lossy().to_string(),
            output_audio_dir: paths.output_audio_dir.to_string_lossy().to_string(),
            model_root_path: paths.model_root_path.to_string_lossy().to_string(),
            streaming_socket_addr: socket_addr.to_string(),
            streaming_socket_token: socket_token.to_string(),
            model_params_json: model_params,
            speakers: speakers
                .into_iter()
                .map(|s| StreamingSpeakerArg {
                    name: s.name,
                    ref_audio_path: s.ref_audio_path,
                    ref_text: s.ref_text,
                    speaker_dir_name: s.speaker_dir_name,
                    category: s.category,
                })
                .collect(),
        }),
    }
}

/// 单条消息的前端 Channel + 完成信号。Channel 载荷为 `InvokeResponseBody`：
/// 控制事件 JSON、chunk 二进制。
pub(crate) struct MessageChannel {
    pub on_event: Channel<InvokeResponseBody>,
    pub done: oneshot::Sender<StreamMessageOutcome>,
}

impl MessageChannel {
    pub(crate) fn new(
        on_event: Channel<InvokeResponseBody>,
        done: oneshot::Sender<StreamMessageOutcome>,
    ) -> Self {
        Self { on_event, done }
    }
}

// 手动实现 Debug：`tauri::ipc::Channel` 不保证实现 `Debug`，且 `oneshot::Sender` 仅在
// `T: Debug` 时实现。这里以非穷尽形式输出，满足 `StreamingSessionExtra` / `ActiveTaskControl`
// 派生 `Debug` 的约束。
impl std::fmt::Debug for MessageChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MessageChannel").finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamMessageOutcome {
    pub cancelled: bool,
    pub errored: bool,
}

/// 流式会话附加态：存于 `ActiveTaskControl.streaming_extra`，runner 与
/// `send_streaming_message` 共享 `message_channels`。
#[derive(Debug, Default)]
pub(crate) struct StreamingSessionExtra {
    pub message_channels: Arc<RwLock<HashMap<String, MessageChannel>>>,
    /// 串行化 context.json 的 read-modify-write，避免同会话并发发送时后写覆盖前写、
    /// 丢失先到达的消息记录。每会话独立一把锁（`register_streaming_session` 时创建）。
    pub context_lock: Arc<Mutex<()>>,
    /// Rust->Python input 帧通道：Socket 连接建立后由 runner 注入，
    /// `send_streaming_message` 持此推送消息（替代 input.jsonl 轮询）。
    pub input_tx: Arc<std::sync::Mutex<Option<mpsc::Sender<Vec<u8>>>>>,
}

impl StreamingSessionExtra {
    /// 注入 input 帧通道（Socket 连接建立后由 runner 调用）。
    pub(crate) fn set_input_sender(&self, tx: mpsc::Sender<Vec<u8>>) {
        if let Ok(mut guard) = self.input_tx.lock() {
            *guard = Some(tx);
        }
    }

    /// 会话结束时关闭 input 通道，令 Socket writer 任务退出。
    pub(crate) fn close_input_sender(&self) {
        if let Ok(mut guard) = self.input_tx.lock() {
            *guard = None;
        }
    }

    /// 推送一条 input 帧到 Python。通道未建立（会话未就绪/已结束）或阻塞时报错。
    pub(crate) fn send_input_frame(&self, frame: Vec<u8>) -> Result<()> {
        let guard = self
            .input_tx
            .lock()
            .map_err(|_| anyhow::anyhow!("流式会话输入通道锁损坏"))?;
        let tx = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("流式会话 Socket 未连接"))?;
        tx.try_send(frame)
            .map_err(|_| anyhow::anyhow!("流式会话输入通道阻塞"))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedStreamingDetail {
    pub base_model: String,
    pub model_version: String,
    pub device: HardwareType,
    /// 流式 UI 参数（temperature/topP 等），透传到 streaming.params.json。
    pub model_params: serde_json::Value,
    pub speakers: Vec<StreamingSpeakerInput>,
}

/// 等待 Python 连接会话 Socket 的超时。Python 启动后即刻连接（模型加载在连接之后），
/// 预留 shell/venv 启动开销。
const STREAMING_SOCKET_ACCEPT_TIMEOUT: Duration = Duration::from_secs(120);
/// Rust->Python input 帧的通道缓冲。帧体为小体积 JSON，Python 合成期间到达的消息
/// 在此排队（语义对齐此前的 input.jsonl append）。
const STREAMING_INPUT_CHANNEL_CAPACITY: usize = 128;
/// Python->Rust 帧通道缓冲。
const STREAMING_FRAME_CHANNEL_CAPACITY: usize = 256;
/// Socket EOF 后等待子进程退出的宽限：shell 滞后于 Python 退出属正常，超时则强杀
/// 进程树，避免孤儿进程占用 GPU/模型。
const CHILD_EXIT_GRACE: Duration = Duration::from_secs(5);
/// 帧静默时的周期性 try_wait 间隔：捕获「shell 存活但 Python 已死」等 Socket 未及时
/// 关闭的场景。
const CHILD_LIVENESS_POLL_INTERVAL: Duration = Duration::from_secs(2);
/// cancel/进程退出后尽力回收已缓冲帧的等待时长。
const FRAME_DRAIN_WAIT: Duration = Duration::from_millis(50);

/// 长期存活的流式会话 runner：spawn `begin_llm_task` -> streaming.py，监听环回
/// Socket 接收二进制帧、按 contextId 分发到 `message_channels`，input 消息经同一
/// Socket 推送给 Python；cancel 信号到达即 kill 子进程并通知所有 pending 消息。
/// 会话状态机：Running（spawn 后）-> Cancelled/Failed/Exited（退出）。
/// `detail` / `extra` 由 `start_streaming_session` 从 DB / 会话注册表取后传入，
/// 本函数不调用 `load_streaming_task_detail` / `streaming_session_extra`，避免前向依赖。
pub(crate) async fn run_streaming_session(
    service: &LocalService,
    request: StreamingPipelineRequest,
    base_model: &str,
    detail: LoadedStreamingDetail,
    extra: Arc<StreamingSessionExtra>,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) -> Result<()> {
    let task_id = request.task_id;
    if detail.base_model.trim() != base_model {
        bail!(
            "streaming task base model mismatch: expected {}, got {}",
            base_model,
            detail.base_model
        );
    }
    let paths =
        resolve_streaming_paths(service, task_id, &detail.base_model, &detail.model_version)?;

    // 先 bind 再 spawn：listener 先于 Python 启动存在，连接无竞态；
    // 环回绑定不触发 Windows 防火墙弹窗，令牌防本机其它进程注入。
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .with_context(|| "failed to bind streaming session socket")?;
    let socket_addr = listener.local_addr()?.to_string();
    let session_token = generate_session_token()?;

    let invocation = build_streaming_invocation(
        &paths,
        &detail.base_model,
        &detail.model_version,
        detail.device,
        detail.model_params.clone(),
        detail.speakers,
        &socket_addr,
        &session_token,
    );
    invocation.write_to_json_file(&paths.params_json_path)?;

    let runtime_config = service.runtime_config()?;
    let log_dir = resolve_local_log_dir(&runtime_config)?;
    let task_log_path = task_log_file_path(&log_dir, HistoryTaskType::StreamingSpeech, task_id);

    let mut cancel_rx =
        service.active_task_cancel_receiver(task_id, HistoryTaskType::StreamingSpeech)?;

    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
        })
        .await?;

    let platform = ScriptPlatform::current();
    let mut shell_args = platform.shell_base_args();
    shell_args.push(
        paths
            .begin_llm_task_script_path
            .to_string_lossy()
            .to_string(),
    );
    shell_args.extend(build_llm_task_script_args(
        &paths.streaming_python_script_path,
        &paths.params_json_path,
        &task_log_path,
        &detail.base_model,
    ));

    // 帧与输入均走环回 Socket（Python 连接 <-> Rust 分帧读写），不再经缓冲文件。
    // 这里统一通过 process.rs 的日志封装把 stderr 接管到任务日志，避免
    // Python/PowerShell 错误只落在子进程管道而未进入统一的 task log。
    let (mut child, stream_tasks) = spawn_logged_child_with_stderr(
        Path::new(platform.shell_program()),
        &shell_args,
        &paths.src_model_root,
        "streaming begin_llm_task",
        &task_log_path,
    )
    .await
    .with_context(|| "failed to spawn streaming begin_llm_task")?;

    // 等待 Python 连接并完成令牌鉴权；失败则收尾子进程后交由上层置 Failed。
    let socket = match timeout(
        STREAMING_SOCKET_ACCEPT_TIMEOUT,
        listener.accept(),
    )
    .await
    {
        Ok(Ok((socket, _peer))) => socket,
        Ok(Err(err)) => {
            cleanup_child_after_transport_error(&mut child, stream_tasks).await;
            bail!("流式会话 Socket 接受连接失败: {err}");
        }
        Err(_) => {
            cleanup_child_after_transport_error(&mut child, stream_tasks).await;
            bail!("流式会话 Socket 连接超时（Python 未在 120 秒内连接）");
        }
    };
    let (mut read_half, write_half) = socket.into_split();
    let (kind, payload) = match read_socket_frame(&mut read_half).await {
        Ok(frame) => frame,
        Err(err) => {
            cleanup_child_after_transport_error(&mut child, stream_tasks).await;
            bail!("流式会话 Socket 鉴权帧读取失败: {err}");
        }
    };
    if kind != FRAME_KIND_AUTH || parse_auth_payload(&payload)? != session_token {
        cleanup_child_after_transport_error(&mut child, stream_tasks).await;
        bail!("流式会话 Socket 鉴权失败");
    }

    // input 帧写入任务：`send_streaming_message` 经 mpsc 投递，writer 落到 Socket。
    let (input_tx, input_rx) = mpsc::channel::<Vec<u8>>(STREAMING_INPUT_CHANNEL_CAPACITY);
    extra.set_input_sender(input_tx);
    let writer_task = tokio::spawn(streaming_socket_writer(write_half, input_rx));

    // 帧读取任务：阻塞分帧读取，EOF/错误时发送 `None` 通知 drive 循环收尾。
    let (frame_tx, frame_rx) = mpsc::channel::<Option<StreamingFrame>>(STREAMING_FRAME_CHANNEL_CAPACITY);
    let reader_task = tokio::spawn(streaming_socket_reader(read_half, frame_tx));

    let mut ready_tx = ready_tx;
    let outcome = drive_streaming_socket(
        &mut child,
        frame_rx,
        &extra,
        &mut cancel_rx,
        &mut ready_tx,
        &task_log_path,
    )
    .await;

    // 退出前清理：关闭 input 通道（writer 随之退出）、通知所有 pending 消息、回收任务。
    extra.close_input_sender();
    drain_pending_channels(&extra, outcome.is_cancelled());
    let _ = reader_task.await;
    let _ = writer_task.await;
    for task in stream_tasks {
        let _ = task.await;
    }

    let final_status = match &outcome {
        StreamSessionOutcome::Cancelled => TaskStatus::Cancelled,
        StreamSessionOutcome::Failed => TaskStatus::Failed,
        StreamSessionOutcome::Exited => TaskStatus::Cancelled, // 进程自然退出视为会话终止
    };
    let _ = service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: final_status,
            duration_seconds: None,
        })
        .await;

    match outcome {
        StreamSessionOutcome::Cancelled | StreamSessionOutcome::Exited => Ok(()),
        StreamSessionOutcome::Failed => bail!("streaming session failed for task {task_id}"),
    }
}

/// 传输层建立失败（accept 超时/鉴权失败）时的子进程收尾：强杀进程树并回收日志任务，
/// 避免孤儿进程占用 GPU/模型。
async fn cleanup_child_after_transport_error(child: &mut Child, stream_tasks: Vec<tokio::task::JoinHandle<()>>) {
    if let Some(pid) = child.id() {
        if let Err(err) = crate::utils::process::force_terminate_process(pid) {
            warn!(error = %err, "force terminate process tree failed, falling back to child.kill");
            let _ = child.kill().await;
        }
    } else {
        let _ = child.kill().await;
    }
    let _ = child.wait().await;
    for task in stream_tasks {
        let _ = task.await;
    }
}

#[derive(Debug, Clone, Copy)]
enum StreamSessionOutcome {
    Cancelled,
    Failed,
    Exited,
}

impl StreamSessionOutcome {
    fn is_cancelled(self) -> bool {
        matches!(self, StreamSessionOutcome::Cancelled)
    }
}

/// 从 Socket 读取一帧：`[u32 LE len][kind][payload]`。
async fn read_socket_frame(read_half: &mut OwnedReadHalf) -> Result<(u8, Vec<u8>)> {
    use tokio::io::AsyncReadExt;
    let mut header = [0u8; FRAME_HEADER_LEN];
    read_half
        .read_exact(&mut header)
        .await
        .context("failed to read streaming frame header")?;
    let (kind, payload_len) = decode_frame_header(&header)?;
    let mut payload = vec![0u8; payload_len];
    read_half
        .read_exact(&mut payload)
        .await
        .context("failed to read streaming frame payload")?;
    Ok((kind, payload))
}

/// 帧读取任务：持续分帧读取并投递解析结果；EOF/读错误投递 `None` 后退出。
async fn streaming_socket_reader(
    mut read_half: OwnedReadHalf,
    frame_tx: mpsc::Sender<Option<StreamingFrame>>,
) {
    loop {
        match read_socket_frame(&mut read_half).await {
            Ok((kind, payload)) => {
                let frame = match kind {
                    FRAME_KIND_CONTROL => match std::str::from_utf8(&payload) {
                        Ok(text) => parse_streaming_frame(text),
                        Err(err) => Err(anyhow::anyhow!("control frame is not utf-8: {err}")),
                    },
                    FRAME_KIND_CHUNK => parse_chunk_payload(&payload).map(|(context_id, bytes)| {
                        Some(StreamingFrame {
                            context_id: context_id.to_string(),
                            payload: StreamingFramePayload::Chunk {
                                bytes: bytes.to_vec(),
                            },
                        })
                    }),
                    other => {
                        warn!(kind = other, "ignoring unknown streaming socket frame kind");
                        continue;
                    }
                };
                match frame {
                    Ok(Some(frame)) => {
                        if frame_tx.send(Some(frame)).await.is_err() {
                            break; // drive 循环已结束
                        }
                    }
                    Ok(None) => {}
                    Err(err) => {
                        warn!(error = %err, "ignoring unparseable streaming frame");
                    }
                }
            }
            Err(err) => {
                // EOF 或读错误：由 drive 循环收尾探活。
                warn!(error = %err, "streaming socket read ended");
                let _ = frame_tx.send(None).await;
                break;
            }
        }
    }
}

/// input 帧写入任务：消费 mpsc 队列并写到 Socket；通道关闭（会话结束）即退出。
async fn streaming_socket_writer(
    mut write_half: OwnedWriteHalf,
    mut input_rx: mpsc::Receiver<Vec<u8>>,
) {
    use tokio::io::AsyncWriteExt;
    while let Some(frame) = input_rx.recv().await {
        if write_half.write_all(&frame).await.is_err() {
            break; // Python 侧已断开
        }
        let _ = write_half.flush().await;
    }
    let _ = write_half.shutdown().await;
}

/// 流式会话驱动：消费帧读取任务产出的帧并按 contextId 分发；Socket EOF 收尾探活；
/// cancel 信号到达即 kill。会话状态机：Running（spawn 后）-> Cancelled/Failed/Exited。
async fn drive_streaming_socket(
    child: &mut Child,
    mut frame_rx: mpsc::Receiver<Option<StreamingFrame>>,
    extra: &StreamingSessionExtra,
    cancel_rx: &mut watch::Receiver<bool>,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
    task_log_path: &Path,
) -> StreamSessionOutcome {
    let mut failed = false;
    let mut cancelled = false;
    loop {
        tokio::select! {
            biased;
            maybe_frame = frame_rx.recv() => {
                match maybe_frame {
                    Some(Some(frame)) => {
                        dispatch_frame(extra, ready_tx, task_log_path, frame);
                    }
                    Some(None) => {
                        // Socket EOF：Python 侧关闭（正常收尾或异常退出）。
                        match timeout(CHILD_EXIT_GRACE, child.wait()).await {
                            Ok(Ok(status)) => {
                                if !status.success() {
                                    failed = true;
                                    log_child_exit(task_log_path, status);
                                }
                            }
                            Ok(Err(err)) => {
                                error!(error = %err, "streaming child wait failed");
                                failed = true;
                            }
                            Err(_) => {
                                warn!("streaming child still alive after socket EOF, force terminating");
                                if let Some(pid) = child.id() {
                                    let _ = crate::utils::process::force_terminate_process(pid);
                                }
                                let _ = child.kill().await;
                                failed = true;
                            }
                        }
                        break;
                    }
                    None => break, // reader 任务终止且未显式发 EOF，按已退出处理
                }
            }
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    info!("streaming cancel signal received, killing child");
                    // child.kill() 仅 TerminateProcess 直系 shell 子进程（powershell.exe），
                    // 不杀 PS1 经 `& $venvPython | Out-File` 拉起的 python.exe 孙进程，
                    // 致其孤儿化、长期占用 GPU/模型--表现为“终止会话”后进程仍阻塞。
                    // 按 PID 强杀整棵进程树（Windows = taskkill /T /F），与
                    // run_logged_command_cancellable 一致；PID 未知则回退 child.kill()。
                    if let Some(pid) = child.id() {
                        if let Err(err) = crate::utils::process::force_terminate_process(pid) {
                            warn!(error = %err, "force terminate process tree failed, falling back to child.kill");
                            let _ = child.kill().await;
                        }
                    } else {
                        let _ = child.kill().await;
                    }
                    cancelled = true;
                    drain_buffered_frames(&mut frame_rx, extra, ready_tx, task_log_path).await;
                    break;
                }
            }
            _ = tokio::time::sleep(CHILD_LIVENESS_POLL_INTERVAL) => {
                if let Ok(Some(status)) = child.try_wait() {
                    if !status.success() {
                        failed = true;
                        log_child_exit(task_log_path, status);
                    }
                    drain_buffered_frames(&mut frame_rx, extra, ready_tx, task_log_path).await;
                    break;
                }
            }
        }
    }

    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(Err("streaming session terminated before ready".to_string()));
    }

    // 始终 reap 子进程，避免僵尸进程（对齐既有 run_logged_shell_script_cancellable 的 wait 语义；
    // tokio Child 缓存退出状态，重复 wait 幂等）。
    let _ = child.wait().await;
    if cancelled {
        StreamSessionOutcome::Cancelled
    } else if failed {
        StreamSessionOutcome::Failed
    } else {
        StreamSessionOutcome::Exited
    }
}

fn log_child_exit(task_log_path: &Path, status: std::process::ExitStatus) {
    if let Err(err) = append_task_log_text(
        task_log_path,
        &format!("[streaming] child exited with status {}", status),
    ) {
        warn!(error = %err, "failed to append child exit status to task log");
    }
}

/// 单帧处理：session_ready 唤醒就绪等待、error 帧落任务日志，并向前端 Channel 分发。
fn dispatch_frame(
    extra: &StreamingSessionExtra,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
    task_log_path: &Path,
    frame: StreamingFrame,
) {
    match &frame.payload {
        StreamingFramePayload::SessionReady => {
            if let Some(tx) = ready_tx.take() {
                let _ = tx.send(Ok(()));
            }
        }
        StreamingFramePayload::Error { message } => {
            warn!(context_id = %frame.context_id, "streaming frame error");
            if let Err(err) = append_task_log_text(
                task_log_path,
                &format!(
                    "[streaming][error] contextId={} {}",
                    frame.context_id, message
                ),
            ) {
                warn!(error = %err, context_id = %frame.context_id, "failed to append streaming frame error to task log");
            }
        }
        _ => {}
    }
    if let Some(event) = frame_to_event(&frame) {
        forward_event(extra, &frame, event);
    }
}

/// 尽力回收已缓冲在帧通道中的收尾帧（cancel/进程退出后调用）。
async fn drain_buffered_frames(
    frame_rx: &mut mpsc::Receiver<Option<StreamingFrame>>,
    extra: &StreamingSessionExtra,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
    task_log_path: &Path,
) {
    tokio::time::sleep(FRAME_DRAIN_WAIT).await;
    while let Ok(maybe_frame) = frame_rx.try_recv() {
        if let Some(frame) = maybe_frame {
            dispatch_frame(extra, ready_tx, task_log_path, frame);
        }
    }
}

fn forward_event(
    extra: &StreamingSessionExtra,
    frame: &StreamingFrame,
    event: InvokeResponseBody,
) {
    let is_terminal = matches!(
        frame.payload,
        StreamingFramePayload::Finished | StreamingFramePayload::Error { .. }
    );
    if is_terminal {
        let mut guard = match extra.message_channels.write() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(mc) = guard.remove(&frame.context_id) {
            let errored = matches!(frame.payload, StreamingFramePayload::Error { .. });
            let _ = mc.on_event.send(event);
            let _ = mc.done.send(StreamMessageOutcome {
                cancelled: false,
                errored,
            });
        }
    } else {
        let guard = match extra.message_channels.read() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(mc) = guard.get(&frame.context_id) {
            // Channel::send 取 &self，读锁即可，避免 Chunk 高频时不必要的写锁竞争。
            let _ = mc.on_event.send(event);
        }
    }
}

fn drain_pending_channels(extra: &StreamingSessionExtra, cancelled: bool) {
    let mut guard = match extra.message_channels.write() {
        Ok(g) => g,
        Err(_) => return,
    };
    let pending = guard.drain().collect::<Vec<_>>();
    drop(guard);
    for (_, mc) in pending {
        let _ = mc.on_event.send(json_event(&AudioStreamEvent::Error {
            message: if cancelled {
                "流式会话已终止".to_string()
            } else {
                "流式会话已结束".to_string()
            },
        }));
        let _ = mc.done.send(StreamMessageOutcome {
            cancelled,
            errored: !cancelled,
        });
    }
}
