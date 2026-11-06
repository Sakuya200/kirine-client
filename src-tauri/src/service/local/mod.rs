mod db;
pub(crate) mod entity;
mod history;
mod model_info;
mod speaker;
pub(crate) mod streaming;
pub(crate) mod supported_models;
mod training;
mod tts;
mod voice_clone;
mod voice_design;

use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::SystemTime,
};

use anyhow::Context;
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::error::{codes, AppError};
use crate::{
    common::local_paths::{resolve_task_path, serialize_task_path},
    config::{
        load_ui_configs, resolve_storage_dir, EnvConfig, UiComponentType,
        UiConfigCatalog,
    },
    migration,
    service::{
        generated_audio::GeneratedAudioService, history::HistoryService, model::ModelService,
        models::HistoryTaskType, speaker::SpeakerService, streaming::StreamingSpeechService,
        training::ModelTrainingService, tts::TextToSpeechService, voice_clone::VoiceCloneService,
        voice_design::VoiceDesignService, Service,
    },
    Result,
};

#[derive(Debug, Clone)]
struct ActiveTaskControl {
    task_type: HistoryTaskType,
    cancel_tx: watch::Sender<bool>,
    _cancel_rx_guard: watch::Receiver<bool>,
    streaming_extra: Option<Arc<crate::service::pipeline::streaming::StreamingSessionExtra>>,
}

#[derive(Debug, Clone)]
pub struct LocalService {
    app_dir: PathBuf,
    data_dir: String,
    model_dir: String,
    runtime_config: Arc<RwLock<EnvConfig>>,
    orm: DatabaseConnection,
    active_task_controls: Arc<RwLock<HashMap<i64, ActiveTaskControl>>>,
    ui_config: Arc<UiConfigCatalog>,
    /// context.json 解析缓存（键 = 文件路径，值 = mtime + 解析结果）：
    /// 消息级高频读取（头像/回放详情）不再全量 read-parse；写入方靠 mtime 变化自然失效。
    context_json_cache:
        Arc<Mutex<HashMap<PathBuf, (SystemTime, Arc<crate::service::pipeline::streaming::StreamingContextJson>)>>>,
    /// 流式会话进程 PID 注册表（task_id -> 直系子进程 PID，运行期内存态）。
    /// 仅用于应用退出时按登记 PID 整树强杀；手工终止走 cancel 信号。
    streaming_session_pids: Arc<Mutex<HashMap<i64, u32>>>,
}

#[async_trait]
impl Service for LocalService {
    async fn new(config: &EnvConfig) -> Result<LocalService> {
        let app_dir =
            current_dir().context("failed to resolve current directory for local service")?;
        let data_dir = resolve_storage_dir(config.data_dir(), "data")
            .context("failed to resolve local data directory")?;
        let model_dir = resolve_storage_dir(config.model_dir(), "models")
            .context("failed to resolve local model directory")?;
        Self::from_paths_with_config(app_dir, data_dir, model_dir, config.clone()).await
    }

    async fn close(&self) -> Result<()> {
        info!("Closing local service and database connection");
        // 应用退出收尾：终止所有运行中任务。
        // ① 经 cancel 信号走管线自身的子进程回收路径；
        // ② 流式会话为长期进程，另按登记 PID 整树强杀（cancel 亦会触发，双保险）。
        self.terminate_all_active_tasks();
        self.orm.clone().close().await?;
        Ok(())
    }

    fn speaker_service(&self) -> &dyn SpeakerService {
        self
    }

    fn model_service(&self) -> &dyn ModelService {
        self
    }

    fn history_service(&self) -> &dyn HistoryService {
        self
    }

    fn generated_audio_service(&self) -> &dyn GeneratedAudioService {
        self
    }

    fn text_to_speech_service(&self) -> &dyn TextToSpeechService {
        self
    }

    fn model_training_service(&self) -> &dyn ModelTrainingService {
        self
    }

    fn voice_clone_service(&self) -> &dyn VoiceCloneService {
        self
    }

    fn voice_design_service(&self) -> &dyn VoiceDesignService {
        self
    }

    fn streaming_speech_service(&self) -> &dyn StreamingSpeechService {
        self
    }
}

impl LocalService {
    pub(crate) async fn from_paths(
        app_dir: PathBuf,
        data_dir: PathBuf,
        model_dir: PathBuf,
    ) -> Result<Self> {
        Self::from_paths_with_config(app_dir, data_dir, model_dir, EnvConfig::default()).await
    }

    pub(crate) async fn from_paths_with_config(
        app_dir: PathBuf,
        data_dir: PathBuf,
        model_dir: PathBuf,
        runtime_config: EnvConfig,
    ) -> Result<Self> {
        if !data_dir.exists() {
            std::fs::create_dir_all(&data_dir).with_context(|| {
                format!(
                    "failed to create local data directory: {}",
                    data_dir.display()
                )
            })?;
        }
        if !model_dir.exists() {
            std::fs::create_dir_all(&model_dir).with_context(|| {
                format!(
                    "failed to create local model directory: {}",
                    model_dir.display()
                )
            })?;
        }

        let db_path = data_dir.join("app.db");
        let orm = db::connect_local_database(&db_path).await?;
        Self::init_db(&orm, &data_dir).await?;

        let service = LocalService {
            app_dir,
            data_dir: data_dir.to_string_lossy().to_string(),
            model_dir: model_dir.to_string_lossy().to_string(),
            runtime_config: Arc::new(RwLock::new(runtime_config)),
            orm,
            active_task_controls: Arc::new(RwLock::new(HashMap::new())),
            ui_config: Arc::new(load_ui_configs().unwrap_or_default()),
            context_json_cache: Arc::new(Mutex::new(HashMap::new())),
            streaming_session_pids: Arc::new(Mutex::new(HashMap::new())),
        };
        Ok(service)
    }

    /// 读取带 mtime 缓存的 context.json（解析结果 Arc 共享）。
    /// 缓存键为文件路径，mtime 变化即重读解析——写入方（创建任务/更新说话人/发消息）
    /// 无需显式失效。
    pub(crate) fn load_streaming_context_cached(
        &self,
        context_json_path: &Path,
    ) -> Result<Arc<crate::service::pipeline::streaming::StreamingContextJson>> {
        let mtime = std::fs::metadata(context_json_path)
            .and_then(|meta| meta.modified())
            .with_context(|| format!("读取流式会话上下文失败: {}", context_json_path.display()))?;
        let mut cache = self
            .context_json_cache
            .lock()
            .expect("context.json cache poisoned");
        if let Some((cached_mtime, cached)) = cache.get(context_json_path) {
            if *cached_mtime == mtime {
                return Ok(Arc::clone(cached));
            }
        }
        let context = Arc::new(
            serde_json::from_str::<crate::service::pipeline::streaming::StreamingContextJson>(
                &fs::read_to_string(context_json_path)
                    .with_context(|| format!("读取流式会话上下文失败: {}", context_json_path.display()))?,
            )
            .with_context(|| format!("解析流式会话上下文失败: {}", context_json_path.display()))?,
        );
        // 简单防膨胀：会话数量级远低于该上限，超出即整体清空
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(context_json_path.to_path_buf(), (mtime, Arc::clone(&context)));
        Ok(context)
    }

    pub(crate) fn app_dir(&self) -> &Path {
        &self.app_dir
    }

    pub fn data_dir(&self) -> &str {
        &self.data_dir
    }

    pub(crate) fn model_dir(&self) -> &str {
        &self.model_dir
    }

    pub(crate) fn orm(&self) -> &DatabaseConnection {
        &self.orm
    }

    pub(crate) fn runtime_config(&self) -> Result<EnvConfig> {
        self.runtime_config
            .read()
            .map(|config| config.clone())
            .map_err(|_| anyhow::anyhow!("无法读取运行时配置状态"))
    }

    pub(crate) fn replace_runtime_config(&self, next_config: EnvConfig) -> Result<()> {
        let mut config = self
            .runtime_config
            .write()
            .map_err(|_| anyhow::anyhow!("无法写入运行时配置状态"))?;
        *config = next_config;
        Ok(())
    }

    pub(crate) fn register_active_task_control(
        &self,
        task_id: i64,
        task_type: HistoryTaskType,
        cancel_tx: watch::Sender<bool>,
        cancel_rx_guard: watch::Receiver<bool>,
    ) {
        if let Ok(mut controls) = self.active_task_controls.write() {
            controls.insert(
                task_id,
                ActiveTaskControl {
                    task_type,
                    cancel_tx,
                    _cancel_rx_guard: cancel_rx_guard,
                    streaming_extra: None,
                },
            );
        }
    }

    pub(crate) fn unregister_active_task_control(&self, task_id: i64) {
        if let Ok(mut controls) = self.active_task_controls.write() {
            controls.remove(&task_id);
        }
    }

    pub(crate) fn active_task_cancel_receiver(
        &self,
        task_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<watch::Receiver<bool>> {
        let controls = self
            .active_task_controls
            .read()
            .map_err(|_| AppError::coded(codes::TASK_HANDLE_READ_FAILED, "无法读取运行中任务句柄").into_anyhow())?;
        let control = controls.get(&task_id).ok_or_else(|| {
            anyhow::anyhow!("当前任务没有可用的终止句柄，可能已经结束或应用已重启")
        })?;
        if control.task_type != task_type {
            anyhow::bail!("任务类型与运行中任务句柄不匹配，无法订阅终止信号");
        }
        Ok(control.cancel_tx.subscribe())
    }

    pub(crate) fn request_active_task_cancel(
        &self,
        task_id: i64,
        task_type: HistoryTaskType,
    ) -> Result<bool> {
        info!(task_id, task_type = %task_type.as_str(), "received task cancellation request");
        let controls = self
            .active_task_controls
            .read()
            .map_err(|_| AppError::coded(codes::TASK_HANDLE_READ_FAILED, "无法读取运行中任务句柄").into_anyhow())?;
        let control = controls.get(&task_id).ok_or_else(|| {
            anyhow::anyhow!("当前任务没有可用的终止句柄，可能已经结束或应用已重启")
        })?;

        if control.task_type != task_type {
            anyhow::bail!("任务类型与运行中任务句柄不匹配，无法发送终止信号");
        }

        if *control.cancel_tx.borrow() {
            warn!(task_id, task_type = %task_type.as_str(), "task cancellation already requested");
            return Ok(false);
        }

        control
            .cancel_tx
            .send(true)
            .map_err(|_| AppError::coded(codes::TASK_TERMINATE_SIGNAL_FAILED, "任务终止信号发送失败").into_anyhow())?;
        info!(task_id, task_type = %task_type.as_str(), "task cancellation signal sent");
        Ok(true)
    }

    /// 应用退出收尾：终止所有运行中任务。
    /// ① 全部任务发 cancel 信号，走管线自身的子进程回收路径；
    /// ② 流式会话为长期进程，另按登记 PID 整树强杀（cancel 驱动循环也会触发，双保险）。
    fn terminate_all_active_tasks(&self) {
        if let Ok(controls) = self.active_task_controls.read() {
            for (task_id, control) in controls.iter() {
                info!(
                    task_id,
                    task_type = %control.task_type.as_str(),
                    "terminating active task on app exit"
                );
                let _ = control.cancel_tx.send(true);
            }
        }
        if let Ok(registry) = self.streaming_session_pids.lock() {
            for (task_id, pid) in registry.iter() {
                info!(
                    task_id,
                    pid,
                    "force terminating streaming session process on app exit"
                );
                if let Err(err) = crate::utils::process::force_terminate_process(*pid) {
                    tracing::warn!(
                        error = %err,
                        task_id,
                        pid,
                        "failed to terminate streaming session on app exit"
                    );
                }
            }
        }
    }

    async fn init_db(orm: &DatabaseConnection, data_dir: &Path) -> Result<()> {
        migration::run_local_migrations(orm).await.map_err(|e| {
            anyhow::anyhow!(
                "failed to run SeaORM local migrations in {}: {}",
                data_dir.display(),
                e
            )
        })?;

        supported_models::sync_supported_models(orm)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "failed to sync model config catalog into local database in {}: {}",
                    data_dir.display(),
                    e
                )
            })?;

        // 清扫上次应用退出后残留的流式会话 Running 行（DB 状态修正）；
        // 进程回收不在此处：应用退出路径已按 PID 注册表整树强杀（见 `close`）
        if let Err(err) = streaming::sweep_stale_streaming_sessions_on_orm(orm).await {
            tracing::warn!(error = %err, "failed to sweep stale streaming sessions during init");
        }

        Ok(())
    }

    pub(crate) fn ui_config(&self) -> &UiConfigCatalog {
        &self.ui_config
    }

    /// `copy_model_param_files` 的异步包装：模型参数文件拷贝（可能是数十 MB 的
    /// 参考音频/文本文件）经 spawn_blocking 执行，避免阻塞 async runtime worker。
    pub(crate) async fn copy_model_param_files_async(
        &self,
        base_model: &str,
        task_kind: HistoryTaskType,
        model_params: &mut serde_json::Value,
        sample_dir: &Path,
    ) -> Result<()> {
        let data_dir = PathBuf::from(self.data_dir());
        let ui_config = Arc::clone(&self.ui_config);
        let sample_dir = sample_dir.to_path_buf();
        let base_model = base_model.to_string();
        let mut params = model_params.clone();

        let updated = crate::utils::blocking::run_blocking(move || {
            copy_model_param_files(
                &base_model,
                task_kind,
                &mut params,
                &sample_dir,
                &data_dir,
                &ui_config,
            )?;
            Ok(params)
        })
        .await?;

        *model_params = updated;
        Ok(())
    }
}

/// Copies any model-specific file-picker param values (componentType: input-audio-file or
/// input-text-file) into the task's sample directory and rewrites the corresponding entries in
/// `model_params` to the serialized (relative) destination paths.
///
/// If no matching `TaskParamConfig` exists for the given `base_model`/`task_kind`, this is a
/// no-op and returns `Ok(())`.
pub(crate) fn copy_model_param_files(
    base_model: &str,
    task_kind: HistoryTaskType,
    model_params: &mut serde_json::Value,
    sample_dir: &Path,
    data_dir: &Path,
    ui_config: &UiConfigCatalog,
) -> crate::Result<()> {
    use anyhow::Context as _;

    let task_config = ui_config
        .task_configs
        .iter()
        .find(|c| c.base_model == base_model && c.task == task_kind);

    let task_config = match task_config {
        Some(c) => c,
        None => return Ok(()),
    };

    for param in &task_config.params {
        if param.component_type != UiComponentType::InputAudioFile
            && param.component_type != UiComponentType::InputTextFile
        {
            continue;
        }

        let raw_value = match model_params.get(&param.name).and_then(|v| v.as_str()) {
            Some(s) if !s.trim().is_empty() => s.to_string(),
            _ => continue,
        };

        let src_path = resolve_task_path(data_dir, &raw_value);
        if !src_path.exists() {
            tracing::warn!(
                param = %param.name,
                path = %src_path.display(),
                "model param file not found, skipping copy"
            );
            continue;
        }

        let ext = src_path
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| !e.trim().is_empty())
            .map(|e| format!(".{}", e))
            .unwrap_or_default();
        let dest_name = format!("{}{}", param.name, ext);
        let dest_path = sample_dir.join(&dest_name);

        fs::copy(&src_path, &dest_path).with_context(|| {
            format!(
                "failed to copy model param file '{}' from {} to {}",
                param.name,
                src_path.display(),
                dest_path.display()
            )
        })?;

        let serialized = serialize_task_path(data_dir, &dest_path);
        if let Some(obj) = model_params.as_object_mut() {
            obj.insert(param.name.clone(), serde_json::Value::String(serialized));
        }
    }

    Ok(())
}

pub(crate) fn build_task_title(
    task_type_label: &str,
    speaker_label: Option<&str>,
    create_time: &str,
) -> String {
    let normalized_speaker = speaker_label
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-");

    match normalized_speaker {
        Some(speaker_label) => format!("{}-{}-{}", task_type_label, speaker_label, create_time),
        None => format!("{}-{}", task_type_label, create_time),
    }
}

pub(crate) fn build_sample_file_name(sample_id: &str, file_name: &str) -> String {
    let sanitized = file_name
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();
    format!("{}_{}", sample_id, sanitized)
}

pub(crate) fn build_task_audio_file_name(source_path: &Path) -> String {
    let extension = source_path
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.trim().is_empty())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();
    format!("audio{}", extension)
}

pub(crate) fn sanitize_path_segment(value: &str) -> String {
    let illegal_chars = [
        '<', '>', ':', '"', '/', '\\', '|', '?', '*', '=', '&', ';', '#', '%', '{', '}', '$', '!',
        '`', '^', '(', ')', '+', ',', '[', ']', '~', '\'', '@',
    ];
    let sanitized = value
        .chars()
        .filter(|ch| !illegal_chars.contains(ch) && !ch.is_control())
        .collect::<String>();
    let sanitized = sanitized.trim_end_matches([' ', '.']).to_string();

    if sanitized.is_empty() {
        "speaker".to_string()
    } else {
        sanitized
    }
}

pub(crate) fn sanitize_file_stem(value: &str, default_stem: &str) -> String {
    let sanitized = sanitize_path_segment(value);
    let trimmed = sanitized.trim_matches('.').trim();

    if trimmed.is_empty() || trimmed == "speaker" {
        default_stem.to_string()
    } else {
        trimmed.to_string()
    }
}
