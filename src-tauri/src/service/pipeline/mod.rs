pub mod api;
pub mod model_artifacts;
pub mod model_paths;
pub mod pipeline;
pub mod script_paths;
pub mod streaming;
pub mod streaming_transport;
pub mod training;
pub mod tts;
pub mod voice_clone;
pub mod voice_design;

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::{bail, Ok};
use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::watch;
use tracing::{info, warn};

use crate::{
    common::task_paths::task_log_file_path,
    config::{EnvConfig, HardwareType},
    service::{
        local::{entity::task_history as task_history_entity, LocalService},
        models::{HistoryTaskType, ModelDownloadType, ModelInfo, TaskStatus},
    },
    utils::{
        file_ops::remove_file_if_exists,
        process::{
            run_logged_shell_script, run_logged_shell_script_cancellable, LoggedCommandResult,
        },
    },
    Result,
};

use self::{
    api::PythonScriptInvocationSpec, pipeline::CommonModelTaskPipeline,
    script_paths::ScriptPlatform,
};

static COMMON_TASK_PIPELINE: CommonModelTaskPipeline = CommonModelTaskPipeline::new();

#[derive(Debug, Clone)]
pub(crate) struct TrainingPipelineRequest {
    pub task_id: i64,
    pub speaker_id: i64,
    pub speaker_name: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TtsPipelineRequest {
    pub task_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VoiceClonePipelineRequest {
    pub task_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VoiceDesignPipelineRequest {
    pub task_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamingPipelineRequest {
    pub task_id: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct CommonRuntimeOptions {
    device: HardwareType,
    attn_implementation: String,
}

impl CommonRuntimeOptions {
    pub(crate) fn from_task_device(task_device: HardwareType, config: &EnvConfig) -> Result<Self> {
        Ok(Self {
            device: task_device,
            attn_implementation: config.attn_implementation().as_str().to_string(),
        })
    }

    pub(crate) fn is_cpu(&self) -> bool {
        self.device == HardwareType::Cpu
    }

    pub(crate) fn device(&self) -> &str {
        self.device.runtime_arg()
    }

    pub(crate) fn attn_implementation(&self) -> &str {
        &self.attn_implementation
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PipelineBootstrapPaths<'a> {
    pub base_model: &'a str,
    pub model_version: &'a str,
    pub log_dir: &'a Path,
    pub src_model_root: &'a Path,
    pub begin_llm_task_script_path: &'a Path,
    pub init_task_runtime_script_path: &'a Path,
    pub download_models_script_path: &'a Path,
}

pub(crate) const INIT_MODEL_RUNTIME_LABEL: &str = "初始化本地模型运行时环境";
pub(crate) const DOWNLOAD_MODEL_ARTIFACTS_LABEL: &str = "下载基础模型权重";

fn model_install_stage_log_path(paths: PipelineBootstrapPaths<'_>, stage: &str) -> PathBuf {
    paths.log_dir.join(format!(
        "install-{}-{}-{}.log",
        paths.base_model, paths.model_version, stage
    ))
}

#[async_trait]
pub(crate) trait ModelTaskPipeline: Send + Sync {
    async fn run_training_pipeline(
        &self,
        base_model: String,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()>;

    async fn run_tts_pipeline(
        &self,
        base_model: String,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()>;

    async fn run_voice_clone_pipeline(
        &self,
        base_model: String,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()>;

    async fn run_voice_design_pipeline(
        &self,
        base_model: String,
        service: &LocalService,
        request: VoiceDesignPipelineRequest,
    ) -> Result<()>;
}

pub(crate) async fn validate_and_init<RunStage, Fut, Label>(
    paths: PipelineBootstrapPaths<'_>,
    task_id: i64,
    use_cpu_mode: bool,
    init_label: Label,
    run_stage: RunStage,
) -> Result<()>
where
    RunStage: Fn(PathBuf, PathBuf, i64, PathBuf, PathBuf, Vec<String>, Label) -> Fut,
    Fut: Future<Output = Result<()>>,
    Label: Copy,
{
    ensure_required_path_exists(paths.init_task_runtime_script_path, "运行时初始化脚本")?;
    ensure_required_path_exists(paths.download_models_script_path, "模型下载脚本")?;
    let init_log_path = model_install_stage_log_path(paths, "init");
    remove_file_if_exists(&init_log_path, "previous install init log")?;

    info!(
        base_model = %paths.base_model,
        model_version = %paths.model_version,
        use_cpu_mode,
        init_script = %paths.init_task_runtime_script_path.display(),
        "开始校验并初始化本地模型运行时环境"
    );

    let mut init_script_args = vec!["--base-model".to_string(), paths.base_model.to_string()];
    if use_cpu_mode {
        init_script_args.push("--cpu-mode".to_string());
    }

    run_stage(
        paths.init_task_runtime_script_path.to_path_buf(),
        paths.src_model_root.to_path_buf(),
        task_id,
        paths.log_dir.to_path_buf(),
        init_log_path,
        init_script_args,
        init_label,
    )
    .await?;

    info!(
        base_model = %paths.base_model,
        model_version = %paths.model_version,
        "本地模型运行时环境校验完成"
    );
    Ok(())
}

pub(crate) async fn validate_and_download<RunStage, Fut, Validate, Label>(
    service: &LocalService,
    paths: PipelineBootstrapPaths<'_>,
    task_id: i64,
    model_info: &ModelInfo,
    download_label: Label,
    run_stage: RunStage,
    validate_downloads: Validate,
) -> Result<()>
where
    RunStage: Fn(PathBuf, PathBuf, i64, PathBuf, PathBuf, Vec<String>, Label) -> Fut,
    Fut: Future<Output = Result<()>>,
    Validate: Fn() -> Result<()>,
    Label: Copy + AsRef<str>,
{
    if service
        .model_downloaded_impl(paths.base_model, paths.model_version)
        .await?
    {
        validate_downloads()?;
        info!(
            base_model = %paths.base_model,
            model_version = %paths.model_version,
            "基础模型权重已存在且校验通过，跳过下载阶段"
        );
        return Ok(());
    }

    let download_log_path = model_install_stage_log_path(paths, "download");
    remove_file_if_exists(&download_log_path, "previous install download log")?;

    info!(
        base_model = %paths.base_model,
        model_version = %paths.model_version,
        download_type = %model_info.download_type,
        "当前模型未标记为已下载，开始准备模型运行时产物"
    );

    match model_info.download_type {
        ModelDownloadType::HfLike => {
            let mut args = vec!["--base-model".to_string(), paths.base_model.to_string()];
            args.extend(self::model_artifacts::build_model_download_script_args(
                paths.src_model_root,
                model_info,
            )?);

            run_stage(
                paths.download_models_script_path.to_path_buf(),
                paths.src_model_root.to_path_buf(),
                task_id,
                paths.log_dir.to_path_buf(),
                download_log_path,
                args,
                download_label,
            )
            .await?;
        }
        ModelDownloadType::Custom => {
            let download_script_path =
                self::model_artifacts::resolve_custom_model_download_script_path(
                    paths.src_model_root,
                    model_info,
                )?;
            let platform = ScriptPlatform::current();

            let mut begin_llm_args = vec![
                "--base-model".to_string(),
                paths.base_model.to_string(),
                "--script-path".to_string(),
                download_script_path.to_string_lossy().to_string(),
                "--log-path".to_string(),
                paths.log_dir.to_string_lossy().to_string(),
                "--task-log-file".to_string(),
                download_log_path.to_string_lossy().to_string(),
                "--".to_string(),
            ];
            begin_llm_args.extend(vec![
                "--base-model".to_string(),
                paths.base_model.to_string(),
                "--model-version".to_string(),
                paths.model_version.to_string(),
                "--target-root-dir".to_string(),
                paths
                    .src_model_root
                    .join(self::model_artifacts::MODEL_ARTIFACTS_DIR)
                    .to_string_lossy()
                    .to_string(),
                "--log-path".to_string(),
                paths.log_dir.to_string_lossy().to_string(),
                "--task-log-file".to_string(),
                download_log_path.to_string_lossy().to_string(),
            ]);

            run_logged_shell_script(
                Path::new(platform.shell_program()),
                paths.begin_llm_task_script_path,
                paths.src_model_root,
                download_label.as_ref(),
                &download_log_path,
                "python script completed successfully",
                platform.shell_base_args(),
                begin_llm_args,
            )
            .await?;
        }
    }

    validate_downloads()?;
    service
        .set_model_downloaded_impl(paths.base_model, paths.model_version, true)
        .await?;
    info!(
        base_model = %paths.base_model,
        model_version = %paths.model_version,
        "基础模型权重下载完成，并已更新本地下载状态"
    );
    Ok(())
}

fn ensure_required_path_exists(path: &Path, label: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    bail!("缺少{}: {}", label, path.display())
}

pub(crate) fn resolve_model_task_pipeline(
    _base_model: &str,
) -> Result<&'static dyn ModelTaskPipeline> {
    Ok(&COMMON_TASK_PIPELINE)
}

/// 构造 `begin_llm_task` 包装器脚本的 CLI 参数向量。
///
/// 抽取为纯函数以便单元测试断言调用模型层脚本时的参数契约（规则4），
/// 同时供 `run_llm_task_invocation` 与 `run_llm_task_invocation_cancellable` 共用。
/// 标记为 `pub` 并经 `test_support` 重导出，供集成测试直接调用。
pub fn build_llm_task_script_args(
    script_path: &Path,
    params_json_path: &Path,
    task_log_path: &Path,
    base_model: &str,
) -> Vec<String> {
    vec![
        "--base-model".into(),
        base_model.into(),
        "--script-path".into(),
        script_path.to_string_lossy().into(),
        "--params-file".into(),
        params_json_path.to_string_lossy().into(),
        "--log-path".into(),
        task_log_path.to_string_lossy().into(),
        "--task-log-file".into(),
        task_log_path.to_string_lossy().into(),
    ]
}

pub(crate) async fn run_llm_task_invocation(
    begin_llm_task_script_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    params_json_path: &Path,
    invocation: &PythonScriptInvocationSpec,
) -> Result<()> {
    invocation.write_to_json_file(params_json_path)?;
    let platform = ScriptPlatform::current();

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        begin_llm_task_script_path,
        current_dir,
        label,
        task_log_path,
        "python command completed successfully",
        platform.shell_base_args(),
        build_llm_task_script_args(
            script_path,
            params_json_path,
            task_log_path,
            &invocation.base_model,
        ),
    )
    .await
}

pub(crate) async fn run_llm_task_invocation_cancellable(
    begin_llm_task_script_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    params_json_path: &Path,
    invocation: &PythonScriptInvocationSpec,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    invocation.write_to_json_file(params_json_path)?;
    let platform = ScriptPlatform::current();

    run_logged_shell_script_cancellable(
        Path::new(platform.shell_program()),
        begin_llm_task_script_path,
        current_dir,
        label,
        task_log_path,
        "python command completed successfully",
        platform.shell_base_args(),
        build_llm_task_script_args(
            script_path,
            params_json_path,
            task_log_path,
            &invocation.base_model,
        ),
        cancel_rx,
    )
    .await
}

pub(crate) async fn run_pipeline_stage_shell_script(
    script_path: &Path,
    current_dir: &Path,
    task_kind: HistoryTaskType,
    task_id: i64,
    log_dir: &Path,
    label: &str,
    success_message: &str,
    script_args: Vec<String>,
) -> Result<()> {
    let platform = ScriptPlatform::current();
    let task_log_path = task_log_file_path(log_dir, task_kind, task_id);
    let mut forwarded_script_args = vec![
        "--log-path".to_string(),
        log_dir.to_string_lossy().to_string(),
        "--task-log-file".to_string(),
        task_log_path.to_string_lossy().to_string(),
    ];
    forwarded_script_args.extend(script_args);

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        script_path,
        current_dir,
        label,
        &task_log_path,
        success_message,
        platform.shell_base_args(),
        forwarded_script_args,
    )
    .await
}

pub(crate) async fn run_pipeline_stage_shell_script_cancellable(
    script_path: &Path,
    current_dir: &Path,
    task_kind: HistoryTaskType,
    task_id: i64,
    log_dir: &Path,
    label: &str,
    success_message: &str,
    script_args: Vec<String>,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    let platform = ScriptPlatform::current();
    let task_log_path = task_log_file_path(log_dir, task_kind, task_id);
    let mut forwarded_script_args = vec![
        "--log-path".to_string(),
        log_dir.to_string_lossy().to_string(),
        "--task-log-file".to_string(),
        task_log_path.to_string_lossy().to_string(),
    ];
    forwarded_script_args.extend(script_args);

    run_logged_shell_script_cancellable(
        Path::new(platform.shell_program()),
        script_path,
        current_dir,
        label,
        &task_log_path,
        success_message,
        platform.shell_base_args(),
        forwarded_script_args,
        cancel_rx,
    )
    .await
}

impl LocalService {
    pub(crate) async fn cancel_task_impl(&self, history_id: i64) -> Result<bool> {
        let record = task_history_entity::Entity::find_by_id(history_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到目标任务"))?;
        let task_type = record
            .task_type
            .parse::<HistoryTaskType>()
            .map_err(|err| anyhow::anyhow!(err))?;
        let status = record
            .status
            .parse::<TaskStatus>()
            .map_err(|err| anyhow::anyhow!(err))?;
        info!(
            task_id = history_id,
            task_type = %task_type.as_str(),
            status = %status.as_str(),
            "handling cancel_history_task request"
        );
        if !matches!(status, TaskStatus::Pending | TaskStatus::Running) {
            warn!(
                task_id = history_id,
                task_type = %task_type.as_str(),
                status = %status.as_str(),
                "rejecting cancellation because task is already finished"
            );
            bail!("当前任务已经结束，无法再次终止");
        }

        self.request_active_task_cancel(history_id, task_type)
    }
}
