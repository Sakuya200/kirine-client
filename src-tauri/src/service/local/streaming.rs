use std::sync::Arc;
use std::time::Duration;
use std::{io, path::Path};

use anyhow::bail;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait,
    EntityTrait, QueryFilter, TransactionTrait,
};
use tauri::ipc::Channel;
use tokio::sync::{oneshot, watch};
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::{
    common::{
        local_paths::{resolve_task_path, serialize_task_path},
        task_paths::{
            ensure_task_sample_dir, streaming_context_json_path, streaming_input_cache_path,
            streaming_message_audio_path, streaming_output_audio_dir,
        },
    },
    config::{BaseModel, HardwareType},
    service::{
        local::entity::{
            streaming_task as streaming_task_entity, task_history as task_history_entity,
        },
        models::{
            CreateStreamingSpeechTaskPayload, HistoryTaskType, SendStreamingMessagePayload,
            StreamingReplayMessage, StreamingReplaySnapshot, StreamingSpeechTaskResult, TaskStatus,
            UpdateTaskStatusPayload,
        },
        pipeline::streaming::{
            serialize_input_entry, MessageChannel, StreamingContextBasic, StreamingContextJson,
            StreamingMessageEntry, StreamingSpeaker,
        },
        LocalService,
    },
    utils::time::now_string,
    Result,
};

/// 单条流式消息等待终帧的超时上限。脚本在产出 finished/error 前卡住、帧缓冲文件
/// 丢失或 runner 异常退出未发 done 信号时，强制收尾避免 `send_streaming_message` 永久挂起。
/// 按真实合成耗时调整（建议单消息上限 5 分钟）。
const STREAMING_MESSAGE_TIMEOUT: Duration = Duration::from_secs(300);
/// 新建流式会话时，等模型加载/voice prompt 编码完成、发出 `session_ready` 后才算建立成功。
const STREAMING_SESSION_READY_TIMEOUT: Duration = Duration::from_secs(900);

impl LocalService {
    pub(crate) async fn get_streaming_replay_snapshot_impl(
        &self,
        history_id: i64,
    ) -> Result<StreamingReplaySnapshot> {
        let history = task_history_entity::Entity::find_by_id(history_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到历史任务: {history_id}"))?;
        if history.task_type != HistoryTaskType::StreamingSpeech.as_str() {
            bail!("任务 {history_id} 不是流式语音会话");
        }

        let detail = streaming_task_entity::Entity::find()
            .filter(streaming_task_entity::Column::HistoryId.eq(history_id))
            .filter(streaming_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话详情: {history_id}"))?;

        let data_dir = Path::new(self.data_dir());
        let context_json_path = resolve_task_path(data_dir, &detail.context_file_path);
        let context: StreamingContextJson =
            serde_json::from_str(&std::fs::read_to_string(context_json_path)?)?;

        let messages = context
            .messages
            .into_iter()
            .map(|message| StreamingReplayMessage {
                history_id: history_id,
                message_id: message.context_id.clone(),
                context_id: message.context_id,
                speaker_name: message.speaker_name,
                text: message.text,
                audio_path: resolve_task_path(data_dir, &message.audio_path)
                    .to_string_lossy()
                    .to_string(),
            })
            .collect::<Vec<_>>();

        let speakers = context
            .basic
            .speakers
            .into_iter()
            .map(|speaker| crate::service::models::StreamingSpeakerInput {
                name: speaker.name,
                base_model: speaker.base_model,
                model_version: speaker.model_version,
                ref_audio_path: speaker.ref_audio_path,
                ref_audio_name: speaker.ref_audio_name,
                ref_text: speaker.ref_text,
                description: speaker.description,
                speaker_dir_name: speaker.speaker_dir_name,
                category: speaker.category,
            })
            .collect::<Vec<_>>();

        Ok(StreamingReplaySnapshot {
            task_id: history_id,
            base_model: detail.base_model,
            model_version: detail.model_version,
            language: detail
                .language
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            device: detail
                .device
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            model_params: serde_json::from_str(&detail.model_params_json)
                .unwrap_or(serde_json::Value::Null),
            speakers,
            messages,
        })
    }

    pub(crate) async fn create_streaming_speech_task_impl(
        &self,
        payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult> {
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let model_version = payload.model_version.trim().to_string();
        if let Err(err) = self
            .ensure_model_current_device_resolved_impl(&base_model, &model_version)
            .await
        {
            warn!(error = %err, base_model, model_version, "failed to resolve model current_device before streaming task");
        }
        let device = payload.device;
        let selected_model_info = self
            .find_supported_model_variant(&base_model, &model_version)
            .await?;
        if !selected_model_info.supported_devices.contains(&device) {
            bail!(
                "模型 {} {} 不支持设备 {}，请切换为 {:?}",
                selected_model_info.model_name,
                selected_model_info.model_version,
                device,
                selected_model_info.supported_devices
            );
        }
        if payload.speakers.is_empty() {
            bail!("流式语音会话至少需要一个说话人");
        }
        // trained 说话人校验：一会话至多一个 trained，且必须提供 speakerDirName；
        // voice-clone（category 缺省视为 voice-clone）须有参考音频。
        let trained_count = payload
            .speakers
            .iter()
            .filter(|s| s.category == "trained")
            .count();
        if trained_count > 1 {
            bail!("流式会话至多支持一个已训练说话人");
        }
        for s in &payload.speakers {
            if s.category == "trained" {
                if s.speaker_dir_name
                    .as_deref()
                    .map(str::is_empty)
                    .unwrap_or(true)
                {
                    bail!("已训练说话人必须提供 speakerDirName");
                }
            } else if s.ref_audio_path.trim().is_empty() {
                bail!("语音克隆说话人必须提供参考音频");
            }
        }

        let mut model_params = payload.model_params.clone();
        let title = super::build_task_title("流式语音", None, &create_time);

        let txn = self.orm().begin().await?;
        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::StreamingSpeech.as_str().to_string()),
            title: Set(title),
            speaker_id: Set(None),
            speaker_name_snapshot: Set("-".to_string()),
            status: Set(TaskStatus::Pending.as_str().to_string()),
            duration_seconds: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            finished_time: Set(None),
            device: Set(device.as_str().to_string()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;
        let task_id = task_history.id;

        let sample_dir = ensure_task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::StreamingSpeech,
            task_id,
        )?;
        super::copy_model_param_files(
            &base_model,
            HistoryTaskType::StreamingSpeech,
            &mut model_params,
            &sample_dir,
            Path::new(self.data_dir()),
            self.ui_config(),
        )?;

        let context_json_path = streaming_context_json_path(&sample_dir);
        let input_cache_path = streaming_input_cache_path(&sample_dir);
        let output_audio_dir = streaming_output_audio_dir(&sample_dir);
        std::fs::create_dir_all(&output_audio_dir)?;

        let context = StreamingContextJson {
            basic: StreamingContextBasic {
                task_id,
                base_model: base_model.clone(),
                model_version: model_version.clone(),
                device: device.as_str().to_string(),
                language: payload.language.as_str().to_string(),
                speakers: payload
                    .speakers
                    .iter()
                    .map(|s| StreamingSpeaker {
                        id: s.name.clone(),
                        name: s.name.clone(),
                        base_model: s.base_model.trim().to_string(),
                        model_version: s.model_version.clone(),
                        ref_audio_path: s.ref_audio_path.clone(),
                        ref_audio_name: s.ref_audio_name.clone(),
                        ref_text: s.ref_text.clone(),
                        description: s.description.clone(),
                        speaker_dir_name: s.speaker_dir_name.clone(),
                        category: s.category.clone(),
                    })
                    .collect(),
            },
            messages: Vec::new(),
        };
        std::fs::write(&context_json_path, serde_json::to_vec_pretty(&context)?)?;

        let serialized_context =
            serialize_task_path(Path::new(self.data_dir()), &context_json_path);
        let serialized_input = serialize_task_path(Path::new(self.data_dir()), &input_cache_path);
        let serialized_audio_dir =
            serialize_task_path(Path::new(self.data_dir()), &output_audio_dir);

        streaming_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            base_model: Set(base_model.clone()),
            model_version: Set(model_version.clone()),
            language: Set(payload.language.as_str().to_string()),
            device: Set(device.as_str().to_string()),
            model_params_json: Set(serde_json::to_string(&model_params)?),
            context_file_path: Set(serialized_context.clone()),
            input_cache_file_path: Set(serialized_input.clone()),
            output_audio_dir: Set(serialized_audio_dir.clone()),
            message_count: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        // 注册会话附加态 + 拉起长期进程，阻塞等待真正就绪。
        self.register_streaming_session(task_id);
        let (ready_tx, ready_rx) = oneshot::channel();
        if let Err(err) = self.start_streaming_session(base_model.clone(), task_id, Some(ready_tx))
        {
            error!(error = %err, task_id, "failed to start streaming session");
            self.unregister_active_task_control(task_id);
            let _ = self
                .update_task_status_impl(UpdateTaskStatusPayload {
                    task_id,
                    status: TaskStatus::Failed,
                    duration_seconds: None,
                })
                .await;
            bail!("流式会话启动失败: {err}");
        }

        match timeout(STREAMING_SESSION_READY_TIMEOUT, ready_rx).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(msg))) => {
                self.request_active_task_cancel(task_id, HistoryTaskType::StreamingSpeech)?;
                self.unregister_active_task_control(task_id);
                let _ = self
                    .update_task_status_impl(UpdateTaskStatusPayload {
                        task_id,
                        status: TaskStatus::Failed,
                        duration_seconds: None,
                    })
                    .await;
                bail!("流式会话启动失败: {msg}");
            }
            Ok(Err(_)) => {
                self.request_active_task_cancel(task_id, HistoryTaskType::StreamingSpeech)?;
                self.unregister_active_task_control(task_id);
                let _ = self
                    .update_task_status_impl(UpdateTaskStatusPayload {
                        task_id,
                        status: TaskStatus::Failed,
                        duration_seconds: None,
                    })
                    .await;
                bail!("流式会话启动失败：runner 早退");
            }
            Err(_) => {
                let _ = self.request_active_task_cancel(task_id, HistoryTaskType::StreamingSpeech);
                self.unregister_active_task_control(task_id);
                let _ = self
                    .update_task_status_impl(UpdateTaskStatusPayload {
                        task_id,
                        status: TaskStatus::Failed,
                        duration_seconds: None,
                    })
                    .await;
                bail!("流式会话启动超时（模型加载未在 15 分钟内完成）");
            }
        }

        Ok(StreamingSpeechTaskResult {
            task_id,
            context_file_path: serialized_context,
            input_cache_file_path: serialized_input,
            output_audio_dir: serialized_audio_dir,
            status: TaskStatus::Running,
            created_at: create_time,
        })
    }

    pub(crate) async fn send_streaming_message_impl(
        &self,
        payload: SendStreamingMessagePayload,
        on_event: Channel<tauri::ipc::InvokeResponseBody>,
    ) -> Result<()> {
        let task_id = payload.task_id;
        let context_id = payload.context_id.clone();

        let history = task_history_entity::Entity::find_by_id(task_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话任务: {task_id}"))?;
        let status = history
            .status
            .parse::<TaskStatus>()
            .map_err(|e| anyhow::anyhow!(e))?;
        if status != TaskStatus::Running {
            bail!("流式会话未在运行（当前状态 {status}），无法发送消息");
        }
        if history.task_type != HistoryTaskType::StreamingSpeech.as_str() {
            bail!("任务 {task_id} 不是流式语音会话");
        }

        let detail = streaming_task_entity::Entity::find()
            .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
            .filter(streaming_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话详情: {task_id}"))?;

        let data_dir = Path::new(self.data_dir());
        let audio_dir = resolve_task_path(data_dir, &detail.output_audio_dir);
        let context_json_path = resolve_task_path(data_dir, &detail.context_file_path);
        let input_cache_path = resolve_task_path(data_dir, &detail.input_cache_file_path);
        // context_id 来自前端，清洗后再 join 音频文件名，防御路径穿越（当前固定 msg-N
        // 已安全，属纵深防御）。channel key 与 context.json 的 contextId 字段仍用原值，
        // 与脚本回传帧的 contextId 对齐。
        let sanitized_context_id = super::sanitize_path_segment(&context_id);
        let audio_path = streaming_message_audio_path(&audio_dir, &sanitized_context_id);
        let serialized_audio_path = serialize_task_path(data_dir, &audio_path);

        // 先注册 Channel，再喂入输入：避免脚本在 Channel 注册前产出终帧导致丢失。
        let extra = self
            .streaming_session_extra(task_id)?
            .ok_or_else(|| anyhow::anyhow!("流式会话进程未运行: {task_id}"))?;
        // 串行化 context.json 的 read-modify-write：同一会话并发发送时避免后写覆盖前写
        // 导致 messages 缺条目（input.jsonl 为 append-only 不受影响）。
        let context_lock = extra.context_lock.clone();
        let (done_tx, done_rx) = oneshot::channel();
        {
            let mut guard = extra
                .message_channels
                .write()
                .map_err(|_| anyhow::anyhow!("会话信道锁损坏"))?;
            guard.insert(context_id.clone(), MessageChannel::new(on_event, done_tx));
        }

        // 追加 context.json messages + input.jsonl（审计日志），并经 Socket 推送 input 帧
        let feed_result: Result<()> = async {
            let _guard = context_lock.lock().await;
            let ctx_bytes = std::fs::read(&context_json_path)?;
            let mut ctx: StreamingContextJson = serde_json::from_slice(&ctx_bytes)?;
            ctx.messages.push(StreamingMessageEntry {
                context_id: context_id.clone(),
                speaker_name: payload.speaker_name.clone(),
                text: payload.text.clone(),
                audio_path: serialized_audio_path,
            });
            std::fs::write(&context_json_path, serde_json::to_vec_pretty(&ctx)?)?;

            let entry_line = serialize_input_entry(
                &context_id,
                &payload.speaker_name,
                &payload.text,
                &audio_path.to_string_lossy(),
            );
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&input_cache_path)?;
            use std::io::Write;
            writeln!(file, "{entry_line}")?;

            // Socket 推送（Python 阻塞 recv，即刻可读；替代此前的 input.jsonl 轮询）。
            extra.send_input_frame(
                crate::service::pipeline::streaming_transport::encode_input_frame(&entry_line),
            )?;
            Ok(())
        }
        .await;

        if let Err(err) = feed_result {
            // 喂入失败：移除已注册的 Channel，避免悬挂等待
            if let Ok(mut guard) = extra.message_channels.write() {
                guard.remove(&context_id);
            }
            return Err(err);
        }

        let outcome = match timeout(STREAMING_MESSAGE_TIMEOUT, done_rx).await {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => bail!("流式会话进程退出，未收到完成信号"),
            Err(_) => {
                // 超时：移除 channel 并经其下发 Error，避免前端永久转圈、Channel 不结束。
                // 若 runner 已先行 remove（终帧已到），则不下发，避免成功后误报错误。
                if let Ok(mut guard) = extra.message_channels.write() {
                    if let Some(mc) = guard.remove(&context_id) {
                        let _ = mc.on_event.send(
                            crate::service::pipeline::streaming::error_event("流式生成超时".into()),
                        );
                    }
                }
                bail!("流式生成超时（contextId={context_id}）");
            }
        };

        // 完成则自增 message_count
        if !outcome.cancelled && !outcome.errored {
            if let Err(e) = streaming_task_entity::Entity::update_many()
                .col_expr(
                    streaming_task_entity::Column::MessageCount,
                    Expr::col(streaming_task_entity::Column::MessageCount).add(1),
                )
                .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
                .exec(self.orm())
                .await
            {
                warn!(error = %e, task_id, "failed to increment message_count");
            }
        }
        if outcome.errored {
            bail!("流式生成失败（contextId={context_id}）");
        }
        Ok(())
    }

    pub(crate) async fn cancel_streaming_task_impl(&self, task_id: i64) -> Result<bool> {
        // 复用既有取消信号路径（watch::Sender -> runner kill child）
        self.request_active_task_cancel(task_id, HistoryTaskType::StreamingSpeech)
    }

    /// 启动清扫：将上次应用退出后残留的 Running 流式会话标记为 Cancelled。
    pub(crate) async fn sweep_stale_streaming_sessions_impl(&self) -> Result<()> {
        sweep_stale_streaming_sessions_on_orm(self.orm()).await
    }

    /// 注册流式会话的运行句柄：创建 cancel 通道并注入 `StreamingSessionExtra`，
    /// 供 runner 与 `send_streaming_message` 共享 `message_channels`。
    pub(crate) fn register_streaming_session(&self, task_id: i64) {
        let extra = Arc::new(crate::service::pipeline::streaming::StreamingSessionExtra::default());
        if let Ok(mut controls) = self.active_task_controls.write() {
            let (cancel_tx, cancel_rx_guard) = watch::channel(false);
            controls.insert(
                task_id,
                super::ActiveTaskControl {
                    task_type: HistoryTaskType::StreamingSpeech,
                    cancel_tx,
                    _cancel_rx_guard: cancel_rx_guard,
                    streaming_extra: Some(extra),
                },
            );
        }
    }

    pub(crate) fn streaming_session_extra(
        &self,
        task_id: i64,
    ) -> Result<Option<Arc<crate::service::pipeline::streaming::StreamingSessionExtra>>> {
        let controls = self
            .active_task_controls
            .read()
            .map_err(|_| anyhow::anyhow!("无法读取运行中任务句柄"))?;
        Ok(controls
            .get(&task_id)
            .and_then(|c| c.streaming_extra.clone()))
    }

    /// 从 DB + `context.json` 加载流式会话详情，供 `run_streaming_session` 使用。
    pub(crate) async fn load_streaming_task_detail(
        &self,
        task_id: i64,
    ) -> Result<crate::service::pipeline::streaming::LoadedStreamingDetail> {
        let detail = streaming_task_entity::Entity::find()
            .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
            .filter(streaming_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话详情: {task_id}"))?;

        let context_path = resolve_task_path(Path::new(self.data_dir()), &detail.context_file_path);
        let ctx: crate::service::pipeline::streaming::StreamingContextJson =
            serde_json::from_str(&std::fs::read_to_string(context_path)?)?;
        let speakers: Vec<crate::service::models::StreamingSpeakerInput> = ctx
            .basic
            .speakers
            .into_iter()
            .map(|s| crate::service::models::StreamingSpeakerInput {
                name: s.name,
                base_model: s.base_model,
                model_version: s.model_version,
                ref_audio_path: s.ref_audio_path,
                ref_audio_name: s.ref_audio_name,
                ref_text: s.ref_text,
                description: s.description,
                speaker_dir_name: s.speaker_dir_name,
                category: s.category,
            })
            .collect();

        Ok(crate::service::pipeline::streaming::LoadedStreamingDetail {
            base_model: detail.base_model,
            model_version: detail.model_version,
            device: detail.device.parse().unwrap_or(HardwareType::Cpu),
            model_params: serde_json::from_str(&detail.model_params_json)
                .unwrap_or(serde_json::Value::Null),
            speakers,
        })
    }

    /// 拉起长期存活的流式会话进程（`begin_llm_task` -> streaming.py）。
    /// cancel 通道由 `register_streaming_session` 预先注册，本方法仅 spawn runner。
    pub(crate) fn start_streaming_session(
        &self,
        base_model: BaseModel,
        task_id: i64,
        ready_tx: Option<oneshot::Sender<Result<(), String>>>,
    ) -> Result<()> {
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            let result = async {
                let detail = service.load_streaming_task_detail(task_id).await?;
                let extra = service.streaming_session_extra(task_id)?.ok_or_else(|| {
                    anyhow::anyhow!("streaming session extra not registered for task {task_id}")
                })?;
                crate::service::pipeline::streaming::run_streaming_session(
                    &service,
                    crate::service::pipeline::StreamingPipelineRequest { task_id },
                    &base_model,
                    detail,
                    extra,
                    ready_tx,
                )
                .await
            }
            .await;
            service.unregister_active_task_control(task_id);
            if let Err(err) = result {
                tracing::error!(error = %err, "local streaming session failed");
                // 兜底：runner 在置 Running 前早退 / panic 会令任务停在 Pending/Running，
                // 前端误以为会话已建立。run_streaming_session 已在自身退出路径更新最终状态
                // （Cancelled/Failed），此处对 Failed 幂等，仅补齐未触达最终状态更新的早退路径。
                let _ = service
                    .update_task_status_impl(UpdateTaskStatusPayload {
                        task_id,
                        status: TaskStatus::Failed,
                        duration_seconds: None,
                    })
                    .await;
            }
        });
        Ok(())
    }
}

/// 仅依赖 `orm` 的清扫实现，供 `init_db`（静态方法，无 `&self`）与 `&self` 方法共用。
pub(crate) async fn sweep_stale_streaming_sessions_on_orm(
    orm: &sea_orm::DatabaseConnection,
) -> Result<()> {
    let stale = task_history_entity::Entity::find()
        .filter(task_history_entity::Column::TaskType.eq(HistoryTaskType::StreamingSpeech.as_str()))
        .filter(task_history_entity::Column::Status.eq(TaskStatus::Running.as_str()))
        .filter(task_history_entity::Column::Deleted.eq(0))
        .all(orm)
        .await?;
    if stale.is_empty() {
        return Ok(());
    }
    info!(count = stale.len(), "sweeping stale streaming sessions");
    let now = now_string()?;
    for record in stale {
        let _ = task_history_entity::Entity::update_many()
            .col_expr(
                task_history_entity::Column::Status,
                Expr::value(TaskStatus::Cancelled.as_str()),
            )
            .col_expr(
                task_history_entity::Column::ModifyTime,
                Expr::value(now.clone()),
            )
            .col_expr(
                task_history_entity::Column::FinishedTime,
                Expr::value(now.clone()),
            )
            .filter(task_history_entity::Column::Id.eq(record.id))
            .exec(orm)
            .await;
    }
    Ok(())
}
