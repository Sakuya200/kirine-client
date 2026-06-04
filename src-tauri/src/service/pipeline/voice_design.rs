use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tokio::sync::watch;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_task_path},
        task_paths::{
            task_log_file_path, task_sample_dir, voice_design_params_json_path,
        },
    },
    config::{BaseModel, HardwareType},
    service::{
        local::{
            entity::{
                task_history as task_history_entity,
                voice_design_task as voice_design_task_entity,
            },
            LocalService,
        },
        models::{HistoryTaskType, TaskStatus, TextToSpeechFormat, UpdateTaskStatusPayload},
        pipeline::{
            api::{
                PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
                PythonScriptTaskKind, VoiceDesignArgs,
            },
            model_artifacts::MODEL_ARTIFACTS_DIR,
            run_pipeline_stage_shell_script_cancellable,
            run_python_params_file_invocation_cancellable,
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_transcode_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            VoiceDesignPipelineRequest,
        },
    },
    utils::{
        audio::{build_ffmpeg_transcode_script_args, resolve_temp_wav_path},
        file_ops::{remove_file_if_exists, replace_output_file},
        process::{run_logged_shell_script_cancellable, LoggedCommandResult},
    },
    Result,
};

const COMMON_VOICE_DESIGN_RUN_LABEL: &str = "run voice design pipeline";
const COMMON_VOICE_DESIGN_CONVERT_LABEL: &str = "convert voice design audio";

#[derive(Debug, Clone)]
pub(crate) struct LoadedVoiceDesignTaskParams {
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: String,
    pub format: TextToSpeechFormat,
    pub prompt: String,
    pub text: String,
    pub device: HardwareType,
    pub output_file_path: String,
    pub model_params_json: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedVoiceDesignPaths {
    pub base_model: String,
    pub model_version: String,
    pub src_model_root: PathBuf,
    pub venv_python_path: PathBuf,
    pub ensure_torch_runtime_script_path: PathBuf,
    pub voice_design_python_script_path: PathBuf,
    pub transcode_script_path: PathBuf,
    pub model_root_path: String,
    pub params_json_path: PathBuf,
}

pub(crate) struct VoiceDesignInvocationContext<'a> {
    pub paths: &'a ResolvedVoiceDesignPaths,
    pub params: &'a LoadedVoiceDesignTaskParams,
    pub temp_wav_path: &'a Path,
}

pub(crate) fn build_voice_design_invocation(
    context: &VoiceDesignInvocationContext<'_>,
) -> PythonScriptInvocationSpec {
    PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: context.paths.base_model.clone(),
        model_version: context.paths.model_version.clone(),
        kind: PythonScriptTaskKind::VoiceDesign,
        runtime: PythonScriptRuntimeOptions {
            device: Some(context.params.device.runtime_arg().to_string()),
            logging_dir: None,
            attn_implementation: None,
        },
        args: PythonScriptTaskArgs::VoiceDesign(VoiceDesignArgs {
            model_root_path: context.paths.model_root_path.clone(),
            speaker_dir_name: None,
            model_params_json: context.params.model_params_json.clone(),
            text: context.params.text.clone(),
            language: context.params.language.clone(),
            instruct: context.params.prompt.clone(),
            output_path: context.temp_wav_path.to_string_lossy().to_string(),
        }),
    }
}

fn resolve_default_model_root_path(src_model_root: &Path, base_model: &str) -> String {
    let artifact_root = src_model_root.join(MODEL_ARTIFACTS_DIR);
    let dedicated_root = artifact_root.join(base_model);
    if dedicated_root.exists() {
        return dedicated_root.to_string_lossy().to_string();
    }

    artifact_root.to_string_lossy().to_string()
}

pub(crate) async fn run_common_voice_design_pipeline(
    service: &LocalService,
    request: VoiceDesignPipelineRequest,
    base_model: &str,
) -> Result<()> {
    let task_id = request.task_id;
    let started_at = std::time::Instant::now();

    let result = async {
        mark_voice_design_running_state(service, task_id).await?;
        let mut cancel_rx =
            service.active_task_cancel_receiver(task_id, HistoryTaskType::VoiceDesign)?;

        let runtime_config = service.runtime_config()?;
        let log_dir = resolve_local_log_dir(&runtime_config)?;
        let params = load_voice_design_task_params(service, task_id).await?;
        if params.base_model.trim() != base_model {
            bail!(
                "Voice design task base model mismatch: expected {}, got {}",
                base_model,
                params.base_model
            );
        }

        let paths = resolve_voice_design_paths(service, task_id, base_model, &params.model_version)?;

        let prepare_result = prepare_voice_design_model_env(
            service,
            &paths.base_model,
            &paths.model_version,
            &paths.src_model_root,
            &paths.venv_python_path,
            &paths.ensure_torch_runtime_script_path,
            task_id,
            &log_dir,
            params.device == HardwareType::Cpu,
            &mut cancel_rx,
        )
        .await?;

        if matches!(prepare_result, LoggedCommandResult::Cancelled) {
            mark_voice_design_cancelled_state(service, task_id, started_at.elapsed().as_secs() as i64)
                .await?;
            return Ok(());
        }

        validate_voice_design_environment(&paths, &params.output_file_path)?;

        let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
        let invocation_context = VoiceDesignInvocationContext {
            paths: &paths,
            params: &params,
            temp_wav_path: &temp_wav_path,
        };
        let invocation = build_voice_design_invocation(&invocation_context);

        let command_result = run_voice_design_python_command(
            &paths.venv_python_path,
            &paths.voice_design_python_script_path,
            &paths.src_model_root,
            &paths.params_json_path,
            task_id,
            &log_dir,
            &temp_wav_path,
            &invocation,
            &mut cancel_rx,
        )
        .await?;

        if matches!(command_result, LoggedCommandResult::Cancelled) {
            mark_voice_design_cancelled_state(service, task_id, started_at.elapsed().as_secs() as i64)
                .await?;
            return Ok(());
        }

        if *cancel_rx.borrow() {
            mark_voice_design_cancelled_state(service, task_id, started_at.elapsed().as_secs() as i64)
                .await?;
            return Ok(());
        }

        let finalize_result = finalize_voice_design_output(
            &paths.src_model_root,
            &paths.transcode_script_path,
            task_id,
            &log_dir,
            &temp_wav_path,
            &params.output_file_path,
            params.format,
            &mut cancel_rx,
        )
        .await?;

        if matches!(finalize_result, LoggedCommandResult::Cancelled) {
            mark_voice_design_cancelled_state(service, task_id, started_at.elapsed().as_secs() as i64)
                .await?;
            return Ok(());
        }

        if !Path::new(&params.output_file_path).exists() {
            bail!("Voice design output file not found after inference: {}", params.output_file_path);
        }

        mark_voice_design_completed_state(service, task_id, started_at.elapsed().as_secs() as i64)
            .await
    }
    .await;

    if let Err(err) = result {
        let duration_seconds = started_at.elapsed().as_secs() as i64;
        if let Err(update_err) =
            mark_voice_design_failed_state(service, task_id, duration_seconds).await
        {
            error!(
                error = %update_err,
                task_id,
                model = %base_model,
                "failed to persist voice design failure state"
            );
        }
        return Err(err);
    }

    Ok(())
}

pub(crate) async fn load_voice_design_task_params(
    service: &LocalService,
    task_id: i64,
) -> Result<LoadedVoiceDesignTaskParams> {
    let task_detail = voice_design_task_entity::Entity::find()
        .filter(voice_design_task_entity::Column::HistoryId.eq(task_id))
        .filter(voice_design_task_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load voice design execution params for task {}",
                task_id
            )
        })?
        .ok_or_else(|| anyhow::anyhow!("voice design task params not found: {}", task_id))?;

    let task_history = task_history_entity::Entity::find_by_id(task_id)
        .filter(task_history_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load voice design task history for task {}",
                task_id
            )
        })?
        .ok_or_else(|| anyhow::anyhow!("voice design history record not found: {}", task_id))?;

    Ok(LoadedVoiceDesignTaskParams {
        base_model: task_detail.base_model,
        model_version: task_detail.model_version.trim().to_string(),
        language: task_detail.language,
        format: task_detail
            .format
            .parse()
            .map_err(|err: String| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
        prompt: task_detail.prompt,
        text: task_detail.text,
        device: task_history
            .device
            .parse::<HardwareType>()
            .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
        output_file_path: resolve_task_path(
            Path::new(service.data_dir()),
            &task_detail.output_file_path.unwrap_or_default(),
        )
        .to_string_lossy()
        .to_string(),
        model_params_json: serde_json::from_str(&task_detail.model_params_json)
            .with_context(|| "failed to parse voice design model params json")?,
    })
}

pub(crate) async fn mark_voice_design_running_state(service: &LocalService, task_id: i64) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_voice_design_completed_state(
    service: &LocalService,
    task_id: i64,
    duration_seconds: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Completed,
            duration_seconds: Some(duration_seconds),
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_voice_design_failed_state(
    service: &LocalService,
    task_id: i64,
    duration_seconds: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Failed,
            duration_seconds: Some(duration_seconds),
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_voice_design_cancelled_state(
    service: &LocalService,
    task_id: i64,
    duration_seconds: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Cancelled,
            duration_seconds: Some(duration_seconds),
        })
        .await?;
    Ok(())
}

pub(crate) async fn prepare_voice_design_model_env(
    service: &LocalService,
    base_model: &str,
    model_version: &str,
    src_model_root: &Path,
    venv_python_path: &Path,
    ensure_torch_runtime_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    use_cpu_mode: bool,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    let model_downloaded = service
        .model_downloaded_impl(base_model, model_version)
        .await?;
    if !model_downloaded {
        bail!(
            "模型 {}:{} 未安装，请先在模型管理页安装后再执行任务",
            base_model,
            model_version
        );
    }

    let mut script_args = vec!["--base-model".to_string(), base_model.to_string()];
    if use_cpu_mode {
        script_args.push("--cpu-mode".to_string());
    }

    let result = run_pipeline_stage_shell_script_cancellable(
        ensure_torch_runtime_script_path,
        src_model_root,
        HistoryTaskType::VoiceDesign,
        task_id,
        log_dir,
        "ensure torch runtime",
        "voice-design command completed successfully",
        script_args,
        cancel_rx,
    )
    .await?;

    if matches!(result, LoggedCommandResult::Cancelled) {
        return Ok(LoggedCommandResult::Cancelled);
    }

    if !venv_python_path.exists() {
        bail!(
            "voice design runtime not ready, missing Python virtual environment: {}. Reinstall the model from model management.",
            venv_python_path.display()
        );
    }

    if !ensure_torch_runtime_script_path.exists() {
        bail!(
            "音色设计 Torch 运行时校验脚本不存在: {}",
            ensure_torch_runtime_script_path.display()
        );
    }

    Ok(LoggedCommandResult::Completed)
}

pub(crate) fn resolve_voice_design_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_version: &str,
) -> Result<ResolvedVoiceDesignPaths> {
    let src_model_root = resolve_src_model_root(service.app_dir())?;
    let platform = ScriptPlatform::current();
    let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
    let ensure_torch_runtime_script_path =
        src_model_root.join(platform.ensure_torch_runtime_relative_path());
    let voice_design_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "voice_design.py")?;
    let transcode_script_path = src_model_transcode_script_path(&src_model_root);
    let sample_root = task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::VoiceDesign,
        task_id,
    );
    let params_json_path = voice_design_params_json_path(&sample_root);

    Ok(ResolvedVoiceDesignPaths {
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        model_root_path: resolve_default_model_root_path(&src_model_root, base_model),
        src_model_root,
        venv_python_path,
        ensure_torch_runtime_script_path,
        voice_design_python_script_path,
        transcode_script_path,
        params_json_path,
    })
}

pub(crate) fn validate_voice_design_environment(
    paths: &ResolvedVoiceDesignPaths,
    output_file_path: &str,
) -> Result<()> {
    for (label, path) in [
        ("Voice design venv python", paths.venv_python_path.as_path()),
        (
            "Voice design ensure-torch-runtime script",
            paths.ensure_torch_runtime_script_path.as_path(),
        ),
        (
            "Voice design python script",
            paths.voice_design_python_script_path.as_path(),
        ),
        (
            "Voice design transcode script",
            paths.transcode_script_path.as_path(),
        ),
    ] {
        if !path.exists() {
            bail!("{} not found: {}", label, path.display());
        }
    }

    if !Path::new(&paths.model_root_path).exists() {
        bail!("Voice design model path not found: {}", paths.model_root_path);
    }

    let output_path = PathBuf::from(output_file_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
        return Ok(());
    }

    bail!(
        "Voice design output parent directory not found: {}",
        output_path.display()
    )
}

pub(crate) async fn run_voice_design_python_command(
    venv_python_path: &Path,
    voice_design_python_script_path: &Path,
    src_model_root: &Path,
    params_json_path: &Path,
    task_id: i64,
    log_dir: &Path,
    temp_wav_path: &Path,
    invocation: &PythonScriptInvocationSpec,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    info!(
        script = %voice_design_python_script_path.display(),
        params_file = %params_json_path.display(),
        "starting voice_design.py through params-file python invocation",
    );

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceDesign, task_id);
    let result = run_python_params_file_invocation_cancellable(
        venv_python_path,
        voice_design_python_script_path,
        src_model_root,
        COMMON_VOICE_DESIGN_RUN_LABEL,
        &task_log_path,
        params_json_path,
        invocation,
        cancel_rx,
    )
    .await?;

    if matches!(result, LoggedCommandResult::Cancelled) {
        return Ok(LoggedCommandResult::Cancelled);
    }

    if !temp_wav_path.exists() {
        bail!("Voice design output file not found after inference: {}", temp_wav_path.display());
    }

    Ok(LoggedCommandResult::Completed)
}

pub(crate) async fn finalize_voice_design_output(
    src_model_root: &Path,
    transcode_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    temp_wav_path: &Path,
    final_output_path: &str,
    format: TextToSpeechFormat,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    let final_output_path = Path::new(final_output_path);
    if format == TextToSpeechFormat::Wav {
        if *cancel_rx.borrow() {
            return Ok(LoggedCommandResult::Cancelled);
        }
        replace_output_file(temp_wav_path, final_output_path, "voice design output")?;
        return Ok(LoggedCommandResult::Completed);
    }

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceDesign, task_id);
    let platform = ScriptPlatform::current();

    let result = run_logged_shell_script_cancellable(
        Path::new(platform.shell_program()),
        transcode_script_path,
        src_model_root,
        COMMON_VOICE_DESIGN_CONVERT_LABEL,
        &task_log_path,
        "shell script completed successfully",
        platform.shell_base_args(),
        build_ffmpeg_transcode_script_args(
            temp_wav_path,
            final_output_path,
            format.as_str(),
            &task_log_path,
        ),
        cancel_rx,
    )
    .await?;

    if matches!(result, LoggedCommandResult::Cancelled) {
        return Ok(LoggedCommandResult::Cancelled);
    }

    remove_file_if_exists(temp_wav_path, "temporary voice design wav file")?;
    Ok(LoggedCommandResult::Completed)
}