use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::{oneshot, watch, Mutex},
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
            script_paths::{
                resolve_src_model_root, src_model_begin_llm_task_script_path,
                src_model_model_python_script_path, ScriptPlatform,
            },
            StreamingPipelineRequest, build_llm_task_script_args,
        },
    },
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
    Ok(Some(StreamingFrame { context_id: raw.context_id, payload }))
}

/// 将帧映射为下发前端的 `AudioStreamEvent`。
pub fn frame_to_event(frame: &StreamingFrame) -> Option<AudioStreamEvent> {
    match &frame.payload {
        StreamingFramePayload::Started => Some(AudioStreamEvent::Started),
        StreamingFramePayload::Chunk { bytes } => Some(AudioStreamEvent::Chunk { bytes: bytes.clone() }),
        StreamingFramePayload::Finished => Some(AudioStreamEvent::Finished),
        StreamingFramePayload::Error { message } => Some(AudioStreamEvent::Error { message: message.clone() }),
    }
}

fn base64_decode(value: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(value).context("failed to base64-decode chunk bytes")
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
        params_json_path,
    })
}

pub(crate) fn build_streaming_invocation(
    paths: &ResolvedStreamingPaths,
    base_model: &str,
    model_version: &str,
    device: HardwareType,
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
            speakers: speakers
                .into_iter()
                .map(|s| StreamingSpeakerArg {
                    name: s.name,
                    ref_audio_path: s.ref_audio_path,
                    ref_text: s.ref_text,
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
        f.debug_struct("MessageChannel")
            .finish_non_exhaustive()
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
    pub speakers: Vec<StreamingSpeakerInput>,
}

/// 长期存活的流式会话 runner：spawn `begin_llm_task` -> streaming.py，逐行读 stdout
/// 分帧、按 contextId 分发到 `message_channels`；cancel 信号到达即 kill 子进程并
/// 通知所有 pending 消息。会话状态机：Running（spawn 后）-> Cancelled/Failed（退出）。
/// `detail` / `extra` 由 `start_streaming_session`（Task 5）从 DB / 会话注册表取后传入，
/// 本函数不调用 `load_streaming_task_detail` / `streaming_session_extra`，避免前向依赖。
pub(crate) async fn run_streaming_session(
    service: &LocalService,
    request: StreamingPipelineRequest,
    base_model: &str,
    detail: LoadedStreamingDetail,
    extra: Arc<StreamingSessionExtra>,
) -> Result<()> {
    let task_id = request.task_id;
    if detail.base_model.trim() != base_model {
        bail!(
            "streaming task base model mismatch: expected {}, got {}",
            base_model,
            detail.base_model
        );
    }
    let paths = resolve_streaming_paths(service, task_id, &detail.base_model, &detail.model_version)?;
    let invocation = build_streaming_invocation(
        &paths,
        &detail.base_model,
        &detail.model_version,
        detail.device,
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
    shell_args.push(paths.begin_llm_task_script_path.to_string_lossy().to_string());
    shell_args.extend(build_llm_task_script_args(
        &paths.streaming_python_script_path,
        &paths.params_json_path,
        &task_log_path,
        &detail.base_model,
    ));

    let mut child = Command::new(platform.shell_program())
        .args(&shell_args)
        .current_dir(&paths.src_model_root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| "failed to spawn streaming begin_llm_task")?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let task_log_path_clone = task_log_path.clone();
    // stderr -> task-log-file（异步落盘）
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&task_log_path_clone)
        {
            Ok(f) => f,
            Err(_) => return,
        };
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = writeln!(file, "{line}");
        }
    });

    let outcome = drive_streaming_stdout(&mut child, stdout, &extra, &mut cancel_rx).await;

    // 退出前清理所有 pending 消息（cancel / 进程退出）
    drain_pending_channels(&extra, outcome.is_cancelled());

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

async fn drive_streaming_stdout(
    child: &mut Child,
    stdout: tokio::process::ChildStdout,
    extra: &StreamingSessionExtra,
    cancel_rx: &mut watch::Receiver<bool>,
) -> StreamSessionOutcome {
    let mut reader = BufReader::new(stdout).lines();
    let mut failed = false;
    let mut cancelled = false;
    loop {
        tokio::select! {
            biased;
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    info!("streaming cancel signal received, killing child");
                    let _ = child.kill().await;
                    cancelled = true;
                    break;
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(raw)) => {
                        match parse_streaming_frame(&raw) {
                            Ok(Some(frame)) => {
                                if let Some(event) = frame_to_event(&frame) {
                                    forward_event(extra, &frame.context_id, event);
                                }
                                if matches!(frame.payload, StreamingFramePayload::Error { .. }) {
                                    warn!(context_id = %frame.context_id, "streaming frame error");
                                }
                            }
                            Ok(None) => {}
                            Err(err) => {
                                warn!(error = %err, line = %raw, "ignoring unparseable streaming line");
                            }
                        }
                    }
                    Ok(None) => {
                        // stdout EOF：进程结束
                        break;
                    }
                    Err(err) => {
                        error!(error = %err, "streaming stdout read error");
                        failed = true;
                        break;
                    }
                }
            }
        }
    }

    // 始终 reap 子进程，避免僵尸进程（对齐既有 run_logged_shell_script_cancellable 的 wait 语义）。
    let status = child.wait().await;
    if cancelled {
        StreamSessionOutcome::Cancelled
    } else if failed || status.map(|s| !s.success()).unwrap_or(true) {
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
            let _ = mc.done.send(StreamMessageOutcome { cancelled: false, errored });
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
        let _ = mc.done.send(StreamMessageOutcome { cancelled, errored: !cancelled });
    }
}

