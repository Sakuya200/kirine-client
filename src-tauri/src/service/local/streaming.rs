use std::path::Path;

use anyhow::bail;
use sea_orm::{
    sea_query::Expr, ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait,
    EntityTrait, QueryFilter, TransactionTrait,
};
use tauri::ipc::Channel;
use tokio::sync::oneshot;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_task_path, serialize_task_path},
        task_paths::{
            ensure_task_sample_dir, streaming_context_json_path, streaming_input_cache_path,
            streaming_message_audio_path, streaming_output_audio_dir,
        },
    },
    hooks::streaming::AudioStreamEvent,
    service::{
        local::entity::{
            streaming_task as streaming_task_entity, task_history as task_history_entity,
        },
        models::{
            CreateStreamingSpeechTaskPayload, HistoryTaskType, SendStreamingMessagePayload,
            StreamingSpeechTaskResult, TaskStatus,
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

impl LocalService {
    pub(crate) async fn create_streaming_speech_task_impl(
        &self,
        payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult> {
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let model_version = payload.model_version.trim().to_string();
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
                    })
                    .collect(),
            },
            messages: Vec::new(),
        };
        std::fs::write(&context_json_path, serde_json::to_vec_pretty(&context)?)?;

        let serialized_context = serialize_task_path(Path::new(self.data_dir()), &context_json_path);
        let serialized_input = serialize_task_path(Path::new(self.data_dir()), &input_cache_path);
        let serialized_audio_dir = serialize_task_path(Path::new(self.data_dir()), &output_audio_dir);

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

        // 注册会话附加态 + 拉起长期进程
        self.register_streaming_session(task_id);
        if let Err(err) = self.start_streaming_session(base_model.clone(), task_id) {
            error!(error = %err, task_id, "failed to start streaming session");
        }

        Ok(StreamingSpeechTaskResult {
            task_id,
            context_file_path: serialized_context,
            input_cache_file_path: serialized_input,
            output_audio_dir: serialized_audio_dir,
            status: TaskStatus::Pending,
            created_at: create_time,
        })
    }

    pub(crate) async fn send_streaming_message_impl(
        &self,
        payload: SendStreamingMessagePayload,
        on_event: Channel<AudioStreamEvent>,
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
        let audio_path = streaming_message_audio_path(&audio_dir, &context_id);
        let serialized_audio_path = serialize_task_path(data_dir, &audio_path);

        // 先注册 Channel，再喂入输入：避免脚本在 Channel 注册前产出终帧导致丢失。
        let extra = self
            .streaming_session_extra(task_id)?
            .ok_or_else(|| anyhow::anyhow!("流式会话进程未运行: {task_id}"))?;
        let (done_tx, done_rx) = oneshot::channel();
        {
            let mut guard = extra
                .message_channels
                .write()
                .map_err(|_| anyhow::anyhow!("会话信道锁损坏"))?;
            guard.insert(context_id.clone(), MessageChannel::new(on_event, done_tx));
        }

        // 追加 context.json messages + input.jsonl
        let feed_result: Result<()> = async {
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

        let outcome = done_rx
            .await
            .map_err(|_| anyhow::anyhow!("流式会话进程退出，未收到完成信号"))?;

        // 完成则自增 message_count
        if !outcome.cancelled && !outcome.errored {
            let _ = streaming_task_entity::Entity::update_many()
                .col_expr(
                    streaming_task_entity::Column::MessageCount,
                    Expr::col(streaming_task_entity::Column::MessageCount).add(1),
                )
                .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
                .exec(self.orm())
                .await;
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
