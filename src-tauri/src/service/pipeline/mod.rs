pub mod api;
pub mod llm_models;
pub mod model_paths;
pub mod qwen3_tts;
pub mod script_paths;
pub mod vox_cpm2;

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::bail;
use async_trait::async_trait;
use tracing::info;

use crate::{service::local::LocalService, Result};

use self::{qwen3_tts::QWEN3_TTS_BASE_MODEL, vox_cpm2::VOX_CPM2_BASE_MODEL};

#[derive(Debug, Clone)]
pub(crate) struct TrainingPipelineRequest {
    pub task_id: i64,
    pub speaker_id: i64,
    pub speaker_name: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct TtsPipelineRequest {
    pub task_id: i64,
    pub speaker_id: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VoiceClonePipelineRequest {
    pub task_id: i64,
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
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()>;

    async fn run_tts_pipeline(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()>;

    async fn run_voice_clone_pipeline(
        &self,
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

pub(crate) fn validate_downloaded_paths(
    base_model: &str,
    model_scale: &str,
    paths: &[PathBuf],
) -> Result<()> {
    let missing_paths = paths
        .iter()
        .filter(|path| !path.exists())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    if missing_paths.is_empty() {
        return Ok(());
    }

    bail!(
        "模型 {}:{} 已在 model_info.downloaded 中标记为已下载，但以下权重路径缺失: {}。请先在模型管理页卸载该模型，再重新安装或重试任务。",
        base_model,
        model_scale,
        missing_paths.join(", ")
    )
}

fn ensure_required_path_exists(path: &Path, label: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }

    bail!("缺少{}: {}", label, path.display())
}

pub(crate) fn resolve_model_task_pipeline(
    base_model: &str,
) -> Result<&'static dyn ModelTaskPipeline> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => Ok(&qwen3_tts::QWEN3_TTS_MODEL_TASK_PIPELINE),
        VOX_CPM2_BASE_MODEL => Ok(&vox_cpm2::VOX_CPM2_MODEL_TASK_PIPELINE),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}

pub(crate) fn resolve_inference_model_path(
    base_model: &str,
    model_root_path: &Path,
) -> Result<PathBuf> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => qwen3_tts::resolve_inference_model_path(model_root_path),
        VOX_CPM2_BASE_MODEL => vox_cpm2::resolve_inference_model_path(model_root_path),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}
