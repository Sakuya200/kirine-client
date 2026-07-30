use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tokio::{
    process::Child,
    sync::{oneshot, watch, Mutex},
};
use tracing::{error, info, warn};

use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{
            streaming_context_json_path, streaming_frames_path, streaming_input_cache_path,
            streaming_output_audio_dir, task_log_file_path, task_sample_dir,
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
            StreamingPipelineRequest,
        },
    },
    utils::process::{append_task_log_text, spawn_logged_child_with_stderr},
    Result,
};

/// 脚本 stdout 单帧（JSON 行）。`bytes` 为 base64。
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
    bytes: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// 解析脚本 stdout 一行。空行返回 `Ok(None)`；坏行返回 `Err`。
pub fn parse_streaming_frame(line: &str) -> Result<Option<StreamingFrame>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let raw: RawFrame = serde_json::from_str(trimmed).context("failed to parse streaming frame")?;
    let payload = match raw.kind.as_str() {
        "started" => StreamingFramePayload::Started,
        "session_ready" => StreamingFramePayload::SessionReady,
        "chunk" => {
            let bytes = base64_decode(raw.bytes.as_deref().unwrap_or(""))?;
            StreamingFramePayload::Chunk { bytes }
        }
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

/// 将帧映射为下发前端的 `AudioStreamEvent`。
pub fn frame_to_event(frame: &StreamingFrame) -> Option<AudioStreamEvent> {
    match &frame.payload {
        StreamingFramePayload::Started => Some(AudioStreamEvent::Started),
        StreamingFramePayload::SessionReady => None,
        StreamingFramePayload::Chunk { bytes } => Some(AudioStreamEvent::Chunk {
            bytes: bytes.clone(),
        }),
        StreamingFramePayload::Finished => Some(AudioStreamEvent::Finished),
        StreamingFramePayload::Error { message } => Some(AudioStreamEvent::Error {
            message: message.clone(),
        }),
    }
}

fn base64_decode(value: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD
        .decode(value)
        .context("failed to base64-decode chunk bytes")
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
    /// Python->Rust 帧缓冲文件（NDJSON append），runner 轮询 tail。
    pub frames_path: PathBuf,
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
    let frames_path = streaming_frames_path(&sample_root);
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
        frames_path,
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
            frames_file_path: paths.frames_path.to_string_lossy().to_string(),
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

/// 单条消息的前端 Channel + 完成信号。
pub(crate) struct MessageChannel {
    pub on_event: Channel<AudioStreamEvent>,
    pub done: oneshot::Sender<StreamMessageOutcome>,
}

impl MessageChannel {
    pub(crate) fn new(
        on_event: Channel<AudioStreamEvent>,
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

/// 长期存活的流式会话 runner：spawn `begin_llm_task` -> streaming.py，轮询 tail
/// `frames.jsonl` 缓冲文件分帧、按 contextId 分发到 `message_channels`；cancel 信号
/// 到达即 kill 子进程并通知所有 pending 消息。会话状态机：Running（spawn 后）->
/// Cancelled/Failed/Exited（退出）。`detail` / `extra` 由 `start_streaming_session`
/// （Task 5）从 DB / 会话注册表取后传入，本函数不调用 `load_streaming_task_detail` /
/// `streaming_session_extra`，避免前向依赖。
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
    let invocation = build_streaming_invocation(
        &paths,
        &detail.base_model,
        &detail.model_version,
        detail.device,
        detail.model_params.clone(),
        detail.speakers,
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

    // 清理可能残留的旧帧缓冲（上次会话异常退出未清理），避免 tail 读到陈旧帧。
    let _ = std::fs::remove_file(&paths.frames_path);

    // 帧走缓冲文件 frames.jsonl（Python append -> Rust tail），不再经 stdout。
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

    let mut ready_tx = ready_tx;
    let outcome = drive_streaming_buffer(
        &mut child,
        &paths.frames_path,
        &extra,
        &mut cancel_rx,
        &mut ready_tx,
        stream_tasks,
        &task_log_path,
    )
    .await;

    // 退出前清理所有 pending 消息（cancel / 进程退出）
    drain_pending_channels(&extra, outcome.is_cancelled());

    // 帧缓冲文件是会话级临时传输载体（非产物），会话结束清理。
    let _ = std::fs::remove_file(&paths.frames_path);

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

/// 帧缓冲文件 tail：持有读句柄，每次 poll 从上次位置读出新增完整行并转发。
///
/// 与 `input.jsonl`（Rust 写 -> Python 轮询读）同构，方向相反：Python append 写
/// `frames.jsonl`，本结构轮询读取。读写分属不同进程的不同访问模式（Python 写 /
/// Rust 读），Windows 文件共享互不排斥（Rust `File::open` 默认共享 R|W|DELETE，
/// Python 写句柄默认共享 R），不会重蹈此前"写-写争用同一 task log"的覆辙。
struct FrameFileTail {
    reader: Option<std::io::BufReader<std::fs::File>>,
    path: PathBuf,
    /// 跨 poll 保留的半行：`read_until` 在 EOF 处读到不含 `'\n'` 的尾巴时缓存，
    /// 下次 poll 追加新字节继续拼接，避免把半截 JSON 当坏行丢弃。
    pending: Vec<u8>,
}

impl FrameFileTail {
    fn new(path: PathBuf) -> Self {
        Self {
            reader: None,
            path,
            pending: Vec::new(),
        }
    }

    /// 读出文件当前所有可用完整帧行并转发；文件尚未创建时静默返回。
    fn poll_and_forward(&mut self) -> Vec<StreamingFrame> {
        let mut frames = Vec::new();
        if self.reader.is_none() {
            let file = match std::fs::File::open(&self.path) {
                Ok(f) => f,
                Err(_) => return frames, // Python 尚未创建 frames.jsonl
            };
            self.reader = Some(std::io::BufReader::new(file));
        }
        let reader = self.reader.as_mut().expect("frames reader initialized");
        loop {
            let n = match reader.read_until(b'\n', &mut self.pending) {
                Ok(n) => n,
                Err(_) => break,
            };
            if n == 0 {
                break; // EOF，本次无新数据（pending 中可能仍残留半行待下次）
            }
            if !self.pending.ends_with(b"\n") {
                break; // 半行，等下次 poll 拼接
            }
            // 完整一行（含 '\n'）：剥离换行后解析转发
            let line_bytes = std::mem::take(&mut self.pending);
            let line = String::from_utf8_lossy(&line_bytes[..line_bytes.len() - 1]);
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match parse_streaming_frame(trimmed) {
                Ok(Some(frame)) => frames.push(frame),
                Ok(None) => {}
                Err(err) => {
                    warn!(error = %err, line = %trimmed, "ignoring unparseable streaming line");
                }
            }
        }
        frames
    }
}

/// 帧缓冲轮询间隔。文件增长无 readiness 事件，靠轮询 tail；20ms 对音频流足够
///（chunk 本就 ~100ms 级），CPU 可忽略。
const FRAME_POLL_INTERVAL: Duration = Duration::from_millis(20);

/// 流式会话驱动：轮询 tail `frames.jsonl` 取帧并按 contextId 分发；`try_wait` 探活
/// 子进程退出；cancel 信号到达即 kill。会话状态机：Running（spawn 后）->
/// Cancelled/Failed/Exited（退出）。
async fn drive_streaming_buffer(
    child: &mut Child,
    frames_path: &Path,
    extra: &StreamingSessionExtra,
    cancel_rx: &mut watch::Receiver<bool>,
    ready_tx: &mut Option<oneshot::Sender<Result<(), String>>>,
    stream_tasks: Vec<tokio::task::JoinHandle<()>>,
    task_log_path: &Path,
) -> StreamSessionOutcome {
    let mut tail = FrameFileTail::new(frames_path.to_path_buf());
    let mut failed = false;
    let mut cancelled = false;
    loop {
        // 1. 排空当前可用帧
        for frame in tail.poll_and_forward() {
            match &frame.payload {
                StreamingFramePayload::SessionReady => {
                    if let Some(tx) = ready_tx.take() {
                        let _ = tx.send(Ok(()));
                    }
                }
                StreamingFramePayload::Error { message } => {
                    warn!(context_id = %frame.context_id, "streaming frame error");
                    if let Err(err) = append_task_log_text(
                        &task_log_path,
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
                forward_event(extra, &frame.context_id, event);
            }
        }

        // 2. 非阻塞探活：进程退出则再 tail 一次（收尾帧，含 finished/error）后结束
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    failed = true;
                    if let Err(err) = append_task_log_text(
                        &task_log_path,
                        &format!("[streaming] child exited with status {}", status),
                    ) {
                        warn!(error = %err, "failed to append child exit status to task log");
                    }
                }
                tail.poll_and_forward();
                break;
            }
            Ok(None) => {}
            Err(err) => {
                error!(error = %err, "streaming child try_wait failed");
                failed = true;
                break;
            }
        }

        // 3. 等待 cancel 或轮询间隔
        tokio::select! {
            biased;
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    info!("streaming cancel signal received, killing child");
                    // child.kill() 仅 TerminateProcess 直系 shell 子进程（powershell.exe），
                    // 不杀 PS1 经 `& $venvPython | Out-File` 拉起的 python.exe 孙进程，
                    // 致其孤儿化、长期占用 GPU/模型——表现为“终止会话”后进程仍阻塞。
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
                    tail.poll_and_forward();
                    break;
                }
            }
            _ = tokio::time::sleep(FRAME_POLL_INTERVAL) => {}
        }
    }

    if let Some(tx) = ready_tx.take() {
        let _ = tx.send(Err("streaming session terminated before ready".to_string()));
    }

    // 始终 reap 子进程，避免僵尸进程（对齐既有 run_logged_shell_script_cancellable 的 wait 语义）。
    let _ = child.wait().await;
    for task in stream_tasks {
        let _ = task.await;
    }
    if cancelled {
        StreamSessionOutcome::Cancelled
    } else if failed {
        StreamSessionOutcome::Failed
    } else {
        StreamSessionOutcome::Exited
    }
}

fn forward_event(extra: &StreamingSessionExtra, context_id: &str, event: AudioStreamEvent) {
    let is_terminal = matches!(
        event,
        AudioStreamEvent::Finished | AudioStreamEvent::Error { .. }
    );
    if is_terminal {
        let mut guard = match extra.message_channels.write() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(mc) = guard.remove(context_id) {
            let errored = matches!(event, AudioStreamEvent::Error { .. });
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
        if let Some(mc) = guard.get(context_id) {
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
        let _ = mc.on_event.send(AudioStreamEvent::Error {
            message: if cancelled {
                "流式会话已终止".to_string()
            } else {
                "流式会话已结束".to_string()
            },
        });
        let _ = mc.done.send(StreamMessageOutcome {
            cancelled,
            errored: !cancelled,
        });
    }
}
