pub mod api;
pub mod model_artifacts;
pub mod model_paths;
pub mod pipeline;
pub mod script_paths;
pub mod training;
pub mod tts;
pub mod voice_clone;

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::{bail, Ok};
use async_trait::async_trait;
use tokio::sync::watch;
use tracing::info;

use crate::{
    common::task_paths::task_log_file_path,
    config::{EnvConfig, HardwareType},
    service::local::LocalService,
    service::models::HistoryTaskType,
    utils::process::{
        run_logged_python_script, run_logged_python_script_cancellable, run_logged_shell_script,
        LoggedCommandResult,
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

#[derive(Debug, Clone)]
pub(crate) struct CommonRuntimeOptions {
    hardware_type: HardwareType,
    attn_implementation: String,
}

impl CommonRuntimeOptions {
    pub(crate) fn from_env_config(config: &EnvConfig) -> Self {
        Self {
            hardware_type: config.hardware_type(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
        }
    }

    pub(crate) const fn is_cpu(&self) -> bool {
        matches!(self.hardware_type, HardwareType::Cpu)
    }

    pub(crate) const fn device(&self) -> &'static str {
        if self.is_cpu() {
            "cpu"
        } else {
            "cuda:0"
        }
    }

    pub(crate) fn attn_implementation(&self) -> &str {
        &self.attn_implementation
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PipelineBootstrapPaths<'a> {
    pub base_model: &'a str,
    pub model_scale: &'a str,
    pub src_model_root: &'a Path,
    pub venv_python_path: &'a Path,
    pub init_task_runtime_script_path: &'a Path,
    pub download_models_script_path: &'a Path,
}

pub(crate) const INIT_MODEL_RUNTIME_LABEL: &str = "初始化本地模型运行时环境";
pub(crate) const DOWNLOAD_MODEL_ARTIFACTS_LABEL: &str = "下载基础模型权重";

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
}

pub(crate) async fn validate_and_init<RunStage, Fut, Label>(
    paths: PipelineBootstrapPaths<'_>,
    task_id: i64,
    log_dir: &Path,
    use_cpu_mode: bool,
    init_label: Label,
    run_stage: RunStage,
) -> Result<()>
where
    RunStage: Fn(PathBuf, PathBuf, i64, PathBuf, Vec<String>, Label) -> Fut,
    Fut: Future<Output = Result<()>>,
    Label: Copy,
{
    ensure_required_path_exists(paths.init_task_runtime_script_path, "运行时初始化脚本")?;
    ensure_required_path_exists(paths.download_models_script_path, "模型下载脚本")?;

    info!(
        base_model = %paths.base_model,
        model_scale = %paths.model_scale,
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
        log_dir.to_path_buf(),
        init_script_args,
        init_label,
    )
    .await?;

    ensure_required_path_exists(paths.venv_python_path, "虚拟环境 Python")?;
    info!(
        base_model = %paths.base_model,
        model_scale = %paths.model_scale,
        venv_python = %paths.venv_python_path.display(),
        "本地模型运行时环境校验完成"
    );
    Ok(())
}

pub(crate) async fn validate_and_download<RunStage, Fut, Validate, Label>(
    service: &LocalService,
    paths: PipelineBootstrapPaths<'_>,
    task_id: i64,
    log_dir: &Path,
    download_script_args: Vec<String>,
    download_label: Label,
    run_stage: RunStage,
    validate_downloads: Validate,
) -> Result<()>
where
    RunStage: Fn(PathBuf, PathBuf, i64, PathBuf, Vec<String>, Label) -> Fut,
    Fut: Future<Output = Result<()>>,
    Validate: Fn() -> Result<()>,
    Label: Copy,
{
    if service
        .model_downloaded_impl(paths.base_model, paths.model_scale)
        .await?
    {
        validate_downloads()?;
        info!(
            base_model = %paths.base_model,
            model_scale = %paths.model_scale,
            "基础模型权重已存在且校验通过，跳过下载阶段"
        );
        return Ok(());
    }

    info!(
        base_model = %paths.base_model,
        model_scale = %paths.model_scale,
        download_script = %paths.download_models_script_path.display(),
        "当前模型未标记为已下载，开始下载基础模型权重"
    );

    let mut args = vec!["--base-model".to_string(), paths.base_model.to_string()];
    args.extend(download_script_args);

    run_stage(
        paths.download_models_script_path.to_path_buf(),
        paths.src_model_root.to_path_buf(),
        task_id,
        log_dir.to_path_buf(),
        args,
        download_label,
    )
    .await?;

    validate_downloads()?;
    service
        .set_model_downloaded_impl(paths.base_model, paths.model_scale, true)
        .await?;
    info!(
        base_model = %paths.base_model,
        model_scale = %paths.model_scale,
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

pub(crate) async fn run_python_params_file_invocation(
    python_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    params_json_path: &Path,
    invocation: &PythonScriptInvocationSpec,
) -> Result<()> {
    invocation.write_to_json_file(params_json_path)?;

    run_logged_python_script(
        python_path,
        script_path,
        current_dir,
        label,
        task_log_path,
        "python command completed successfully",
        vec![
            "--params-file".to_string(),
            params_json_path.to_string_lossy().to_string(),
        ],
    )
    .await
}

pub(crate) async fn run_python_params_file_invocation_cancellable(
    python_path: &Path,
    script_path: &Path,
    current_dir: &Path,
    label: &str,
    task_log_path: &Path,
    params_json_path: &Path,
    invocation: &PythonScriptInvocationSpec,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    invocation.write_to_json_file(params_json_path)?;

    run_logged_python_script_cancellable(
        python_path,
        script_path,
        current_dir,
        label,
        task_log_path,
        "python command completed successfully",
        vec![
            "--params-file".to_string(),
            params_json_path.to_string_lossy().to_string(),
        ],
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
