use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_task_path},
        task_paths::{
            ensure_task_metrics_log_dir, task_log_file_path, task_sample_dir,
            voice_clone_params_json_path,
        },
    },
    config::BaseModel,
    service::{
        local::{
            entity::{
                task_history as task_history_entity, voice_clone_task as voice_clone_task_entity,
            },
            LocalService,
        },
        models::{HistoryTaskType, TaskStatus, TextToSpeechFormat, UpdateTaskStatusPayload},
        pipeline::{
            api::{
                PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
                PythonScriptTaskKind, VoiceCloneArgs,
            },
            model_artifacts::{
                build_model_download_script_args, resolve_model_download_paths,
                validate_model_artifact_paths,
                MODEL_ARTIFACTS_DIR,
            },
            run_pipeline_stage_shell_script, run_python_params_file_invocation,
            script_paths::{
                src_model_model_python_script_path, src_model_transcode_script_path,
                src_model_venv_python_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, CommonRuntimeOptions, PipelineBootstrapPaths,
            VoiceClonePipelineRequest, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
    },
    utils::{
        audio::{
            build_ffmpeg_transcode_script_args, resolve_normalized_wav_sidecar_path,
            resolve_temp_wav_path,
        },
        file_ops::{remove_file_if_exists, replace_output_file},
        process::run_logged_shell_script,
    },
    Result,
};

const COMMON_VOICE_CLONE_RUN_LABEL: &str = "run voice clone pipeline";
const COMMON_VOICE_CLONE_START_LOG_MESSAGE: &str =
    "starting voice_clone.py through params-file python invocation";
const COMMON_VOICE_CLONE_OUTPUT_LABEL: &str = "voice clone output";
const COMMON_VOICE_CLONE_OUTPUT_MISSING_LABEL: &str =
    "Voice clone output file not found after inference";
const COMMON_VOICE_CLONE_TEMP_WAV_LABEL: &str = "temporary voice clone wav file";
const COMMON_VOICE_CLONE_CONVERT_LABEL: &str = "convert voice clone audio";
const COMMON_VOICE_CLONE_NORMALIZE_LABEL: &str = "normalize voice clone reference audio";

#[derive(Debug, Clone)]
pub(crate) struct CommonVoiceCloneModelParams {
    pub model_params_json: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedVoiceCloneTaskParams {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: String,
    pub format: TextToSpeechFormat,
    pub ref_audio_path: String,
    pub ref_text: Option<String>,
    pub text: String,
    pub output_file_path: String,
    pub model_params_json: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedVoiceClonePaths {
    pub base_model: String,
    pub model_scale: String,
    pub src_model_root: PathBuf,
    pub venv_python_path: PathBuf,
    pub init_task_runtime_script_path: PathBuf,
    pub download_models_script_path: PathBuf,
    pub voice_clone_python_script_path: PathBuf,
    pub transcode_script_path: PathBuf,
    pub model_root_path: String,
    pub speaker_dir_name: Option<String>,
    pub params_json_path: PathBuf,
}

pub(crate) struct VoiceCloneInvocationContext<'a> {
    pub paths: &'a ResolvedVoiceClonePaths,
    pub params: &'a LoadedVoiceCloneTaskParams,
    pub normalized_ref_audio_path: &'a Path,
    pub temp_wav_path: &'a Path,
    pub metrics_log_dir: &'a Path,
    pub runtime: CommonRuntimeOptions,
}

pub(crate) fn build_shared_voice_clone_invocation(
    base_model: &str,
    context: &VoiceCloneInvocationContext<'_>,
) -> PythonScriptInvocationSpec {
    PythonScriptInvocationSpec {
        version: 1,
        base_model: base_model.to_string(),
        kind: PythonScriptTaskKind::VoiceClone,
        runtime: PythonScriptRuntimeOptions {
            device: Some(context.runtime.device().to_string()),
            logging_dir: Some(context.metrics_log_dir.to_string_lossy().to_string()),
            attn_implementation: Some(context.runtime.attn_implementation().to_string()),
        },
        args: PythonScriptTaskArgs::VoiceClone(VoiceCloneArgs {
            model_root_path: context.paths.model_root_path.clone(),
            speaker_dir_name: context.paths.speaker_dir_name.clone(),
            model_params_json: context.params.model_params_json.clone(),
            ref_audio_path: context
                .normalized_ref_audio_path
                .to_string_lossy()
                .to_string(),
            ref_text: context.params.ref_text.clone(),
            language: context.params.language.clone(),
            output_path: context.temp_wav_path.to_string_lossy().to_string(),
            text: context.params.text.clone(),
        }),
    }
}

pub(crate) async fn run_common_voice_clone_pipeline(
    service: &LocalService,
    request: VoiceClonePipelineRequest,
    base_model: &str,
) -> Result<()> {
    let task_id = request.task_id;
    let started_at = std::time::Instant::now();

    let result = async {
        mark_voice_clone_running_state(service, task_id).await?;

        let runtime_config = service.runtime_config()?;
        let runtime = CommonRuntimeOptions::from_env_config(&runtime_config);
        let log_dir = resolve_local_log_dir()?;
        let params = load_voice_clone_task_params(service, task_id).await?;
        if params.base_model.trim() != base_model {
            bail!(
                "Voice clone task base model mismatch: expected {}, got {}",
                base_model,
                params.base_model
            );
        }
        let paths = resolve_voice_clone_paths(service, task_id, base_model, &params.model_scale)?;

        prepare_voice_clone_model_env(
            service,
            &paths.base_model,
            &paths.model_scale,
            &paths.src_model_root,
            &paths.venv_python_path,
            &paths.init_task_runtime_script_path,
            &paths.download_models_script_path,
            task_id,
            &log_dir,
            runtime.is_cpu(),
        )
        .await?;

        validate_voice_clone_environment(
            &paths,
            Path::new(&params.ref_audio_path),
            &params.output_file_path,
        )?;

        let normalized_ref_audio_path = normalize_voice_clone_reference_audio(
            &paths.src_model_root,
            &paths.transcode_script_path,
            task_id,
            &log_dir,
            Path::new(&params.ref_audio_path),
            COMMON_VOICE_CLONE_NORMALIZE_LABEL,
        )
        .await?;

        let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
        let metrics_log_dir = ensure_task_metrics_log_dir(&log_dir)?;
        let invocation_context = VoiceCloneInvocationContext {
            paths: &paths,
            params: &params,
            normalized_ref_audio_path: &normalized_ref_audio_path,
            temp_wav_path: &temp_wav_path,
            metrics_log_dir: &metrics_log_dir,
            runtime,
        };
        let invocation =
            build_shared_voice_clone_invocation(&paths.base_model, &invocation_context);

        run_voice_clone_python_command(
            &paths.venv_python_path,
            &paths.voice_clone_python_script_path,
            &paths.src_model_root,
            &paths.params_json_path,
            task_id,
            &log_dir,
            &invocation,
            COMMON_VOICE_CLONE_RUN_LABEL,
            COMMON_VOICE_CLONE_START_LOG_MESSAGE,
        )
        .await?;

        finalize_voice_clone_output(
            &paths.src_model_root,
            &paths.transcode_script_path,
            task_id,
            &log_dir,
            &temp_wav_path,
            &params.output_file_path,
            params.format,
            COMMON_VOICE_CLONE_CONVERT_LABEL,
        )
        .await?;

        if !Path::new(&params.output_file_path).exists() {
            bail!(
                "{}: {}",
                COMMON_VOICE_CLONE_OUTPUT_MISSING_LABEL,
                params.output_file_path
            );
        }

        mark_voice_clone_completed_state(service, task_id, started_at.elapsed().as_secs() as i64)
            .await
    }
    .await;

    if let Err(err) = result {
        let duration_seconds = started_at.elapsed().as_secs() as i64;
        let error_message = err.to_string();
        if let Err(update_err) =
            mark_voice_clone_failed_state(service, task_id, duration_seconds, &error_message).await
        {
            error!(
                error = %update_err,
                task_id,
                model = %base_model,
                "failed to persist voice clone failure state"
            );
        }
        return Err(err);
    }

    Ok(())
}

pub(crate) async fn load_voice_clone_task_params(
    service: &LocalService,
    task_id: i64,
) -> Result<LoadedVoiceCloneTaskParams> {
    let task_detail = voice_clone_task_entity::Entity::find()
        .filter(voice_clone_task_entity::Column::HistoryId.eq(task_id))
        .filter(voice_clone_task_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load voice clone execution params for task {}",
                task_id
            )
        })?
        .ok_or_else(|| anyhow::anyhow!("未找到 Voice Clone 任务执行参数: {}", task_id))?;

    task_history_entity::Entity::find_by_id(task_id)
        .filter(task_history_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load voice clone task history for task {}",
                task_id
            )
        })?
        .ok_or_else(|| anyhow::anyhow!("未找到 Voice Clone 历史任务记录: {}", task_id))?;

    let ref_text = task_detail.ref_text.trim().to_string();
    let model_params = parse_common_voice_clone_model_params(&task_detail.model_params_json)?;

    Ok(LoadedVoiceCloneTaskParams {
        base_model: task_detail.base_model,
        model_scale: task_detail.model_scale.trim().to_string(),
        language: task_detail.language,
        format: task_detail
            .format
            .parse()
            .map_err(|err: String| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
        ref_audio_path: resolve_task_path(
            Path::new(service.data_dir()),
            &task_detail.ref_audio_path,
        )
        .to_string_lossy()
        .to_string(),
        ref_text: (!ref_text.is_empty()).then_some(ref_text),
        text: task_detail.text,
        output_file_path: resolve_task_path(
            Path::new(service.data_dir()),
            &task_detail.output_file_path.unwrap_or_default(),
        )
        .to_string_lossy()
        .to_string(),
        model_params_json: model_params.model_params_json,
    })
}

pub(crate) fn parse_common_voice_clone_model_params(
    model_params_json: &str,
) -> Result<CommonVoiceCloneModelParams> {
    Ok(CommonVoiceCloneModelParams {
        model_params_json: serde_json::from_str(model_params_json)
            .with_context(|| "failed to parse voice clone model params json")?,
    })
}

pub(crate) async fn mark_voice_clone_running_state(
    service: &LocalService,
    task_id: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
            error_message: None,
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_voice_clone_completed_state(
    service: &LocalService,
    task_id: i64,
    duration_seconds: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Completed,
            duration_seconds: Some(duration_seconds),
            error_message: None,
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_voice_clone_failed_state(
    service: &LocalService,
    task_id: i64,
    duration_seconds: i64,
    error_message: &str,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Failed,
            duration_seconds: Some(duration_seconds),
            error_message: Some(error_message.trim().to_string()),
        })
        .await?;
    Ok(())
}

pub(crate) async fn prepare_voice_clone_model_env(
    service: &LocalService,
    base_model: &str,
    model_scale: &str,
    src_model_root: &Path,
    venv_python_path: &Path,
    init_task_runtime_script_path: &Path,
    download_models_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    use_cpu_mode: bool,
) -> Result<()> {
    let bootstrap_paths = PipelineBootstrapPaths {
        base_model,
        model_scale,
        src_model_root,
        venv_python_path,
        init_task_runtime_script_path,
        download_models_script_path,
    };
    let model_info = service
        .get_model_info_by_base_and_scale_impl(base_model, model_scale)
        .await?;
    let download_paths = resolve_model_download_paths(src_model_root, &model_info);

    validate_and_init(
        bootstrap_paths,
        task_id,
        log_dir,
        use_cpu_mode,
        INIT_MODEL_RUNTIME_LABEL,
        |script_path, working_dir, task_id, log_dir, script_args, label| async move {
            run_pipeline_stage_shell_script(
                &script_path,
                &working_dir,
                HistoryTaskType::VoiceClone,
                task_id,
                &log_dir,
                label,
                "voice clone command completed successfully",
                script_args,
            )
            .await
        },
    )
    .await?;

    validate_and_download(
        service,
        bootstrap_paths,
        task_id,
        log_dir,
        build_model_download_script_args(src_model_root, &model_info)?,
        DOWNLOAD_MODEL_ARTIFACTS_LABEL,
        |script_path, working_dir, task_id, log_dir, script_args, label| async move {
            run_pipeline_stage_shell_script(
                &script_path,
                &working_dir,
                HistoryTaskType::VoiceClone,
                task_id,
                &log_dir,
                label,
                "voice clone command completed successfully",
                script_args,
            )
            .await
        },
        || validate_model_artifact_paths(base_model, model_scale, &download_paths),
    )
    .await
}

pub(crate) async fn normalize_voice_clone_reference_audio(
    src_model_root: &Path,
    transcode_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    input_path: &Path,
    normalize_label: &str,
) -> Result<PathBuf> {
    if !input_path.exists() {
        anyhow::bail!("Reference audio file not found: {}", input_path.display());
    }

    let output_path = resolve_normalized_wav_sidecar_path(input_path);
    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
    let platform = ScriptPlatform::current();

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        transcode_script_path,
        src_model_root,
        normalize_label,
        &task_log_path,
        "shell script completed successfully",
        platform.shell_base_args(),
        build_ffmpeg_transcode_script_args(input_path, &output_path, "wav", &task_log_path),
    )
    .await?;

    if !output_path.exists() {
        anyhow::bail!(
            "Normalized reference audio not found after conversion: {}",
            output_path.display()
        );
    }

    Ok(output_path)
}

pub(crate) async fn finalize_voice_clone_output(
    src_model_root: &Path,
    transcode_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    temp_wav_path: &Path,
    final_output_path: &str,
    format: TextToSpeechFormat,
    convert_label: &str,
) -> Result<()> {
    let final_output_path = Path::new(final_output_path);
    if format == TextToSpeechFormat::Wav {
        replace_output_file(
            temp_wav_path,
            final_output_path,
            COMMON_VOICE_CLONE_OUTPUT_LABEL,
        )?;
        return Ok(());
    }

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
    let platform = ScriptPlatform::current();

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        transcode_script_path,
        src_model_root,
        convert_label,
        &task_log_path,
        "shell script completed successfully",
        platform.shell_base_args(),
        build_ffmpeg_transcode_script_args(
            temp_wav_path,
            final_output_path,
            format.as_str(),
            &task_log_path,
        ),
    )
    .await?;

    remove_file_if_exists(temp_wav_path, COMMON_VOICE_CLONE_TEMP_WAV_LABEL)?;
    Ok(())
}

pub(crate) fn resolve_voice_clone_paths_base(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_scale: &str,
    src_model_root: PathBuf,
) -> Result<ResolvedVoiceClonePaths> {
    let platform = ScriptPlatform::current();
    let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
    let init_task_runtime_script_path =
        src_model_root.join(platform.init_task_runtime_relative_path());
    let download_models_script_path = src_model_root.join(platform.download_models_relative_path());
    let voice_clone_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "voice_clone.py")?;
    let transcode_script_path = src_model_transcode_script_path(&src_model_root);
    let sample_root = task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::VoiceClone,
        task_id,
    );
    let params_json_path = voice_clone_params_json_path(&sample_root);
    let model_root_path = src_model_root
        .join(MODEL_ARTIFACTS_DIR)
        .to_string_lossy()
        .to_string();

    Ok(ResolvedVoiceClonePaths {
        base_model: base_model.to_string(),
        model_scale: model_scale.to_string(),
        src_model_root,
        venv_python_path,
        init_task_runtime_script_path,
        download_models_script_path,
        voice_clone_python_script_path,
        transcode_script_path,
        model_root_path,
        speaker_dir_name: None,
        params_json_path,
    })
}

pub(crate) fn resolve_voice_clone_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_scale: &str,
) -> Result<ResolvedVoiceClonePaths> {
    let src_model_root = crate::service::pipeline::script_paths::resolve_src_model_root(service.app_dir())?;

    resolve_voice_clone_paths_base(
        service,
        task_id,
        base_model,
        model_scale,
        src_model_root,
    )
}

pub(crate) fn validate_voice_clone_environment(
    paths: &ResolvedVoiceClonePaths,
    ref_audio_path: &Path,
    output_file_path: &str,
) -> Result<()> {
    for (label, path) in [
        (
            "Voice clone init-task-runtime script",
            paths.init_task_runtime_script_path.as_path(),
        ),
        (
            "Voice clone download-models script",
            paths.download_models_script_path.as_path(),
        ),
        ("Voice clone venv python", paths.venv_python_path.as_path()),
        (
            "Voice clone python script",
            paths.voice_clone_python_script_path.as_path(),
        ),
        (
            "Voice clone transcode script",
            paths.transcode_script_path.as_path(),
        ),
        (
            "Voice clone model root path",
            Path::new(&paths.model_root_path),
        ),
    ] {
        if !path.exists() {
            bail!("{} not found: {}", label, path.display());
        }
    }

    if !ref_audio_path.exists() {
        bail!(
            "Reference audio file not found: {}",
            ref_audio_path.display()
        );
    }

    let output_path = PathBuf::from(output_file_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
        return Ok(());
    }

    bail!(
        "{} parent directory not found: {}",
        COMMON_VOICE_CLONE_OUTPUT_LABEL,
        output_path.display()
    )
}

pub(crate) async fn run_voice_clone_python_command(
    venv_python_path: &Path,
    voice_clone_python_script_path: &Path,
    src_model_root: &Path,
    params_json_path: &Path,
    task_id: i64,
    log_dir: &Path,
    invocation: &PythonScriptInvocationSpec,
    run_label: &str,
    start_message: &str,
) -> Result<()> {
    info!(
        script = %voice_clone_python_script_path.display(),
        params_file = %params_json_path.display(),
        "{}",
        start_message,
    );

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
    run_python_params_file_invocation(
        venv_python_path,
        voice_clone_python_script_path,
        src_model_root,
        run_label,
        &task_log_path,
        params_json_path,
        invocation,
    )
    .await
}
