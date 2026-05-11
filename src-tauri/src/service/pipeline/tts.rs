use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_task_path},
        task_paths::{
            ensure_task_metrics_log_dir, task_log_file_path, task_sample_dir, tts_params_json_path,
        },
    },
    config::BaseModel,
    service::{
        local::{
            entity::{
                speaker as speaker_entity, task_history as task_history_entity,
                tts_task as tts_task_entity,
            },
            LocalService,
        },
        models::{
            HistoryTaskType, SpeakerSource, TaskStatus, TextToSpeechFormat, UpdateTaskStatusPayload,
        },
        pipeline::{
            api::{
                PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
                PythonScriptTaskKind, TTSArgs,
            },
            model_artifacts::{
                resolve_model_download_paths, validate_model_artifact_paths, MODEL_ARTIFACTS_DIR,
            },
            model_paths::speaker_model_dir,
            run_pipeline_stage_shell_script, run_python_params_file_invocation,
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_transcode_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, CommonRuntimeOptions, PipelineBootstrapPaths,
            TtsPipelineRequest, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
    },
    utils::{
        audio::{build_ffmpeg_transcode_script_args, resolve_temp_wav_path},
        file_ops::{remove_file_if_exists, replace_output_file},
        process::run_logged_shell_script,
    },
    Result,
};

const COMMON_TTS_RUN_LABEL: &str = "run tts pipeline";
const COMMON_TTS_START_LOG_MESSAGE: &str = "starting tts.py through params-file python invocation";
const COMMON_TTS_OUTPUT_LABEL: &str = "tts output";
const COMMON_TTS_OUTPUT_MISSING_LABEL: &str = "TTS output file not found after inference";
const COMMON_TTS_TEMP_WAV_LABEL: &str = "temporary tts wav file";
const COMMON_TTS_CONVERT_LABEL: &str = "convert tts audio";

#[derive(Debug, Clone)]
pub(crate) struct CommonTtsModelParams {
    pub model_params_json: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedTtsTaskParams {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub model_root_path: String,
    pub speaker_dir_name: Option<String>,
    pub language: String,
    pub format: TextToSpeechFormat,
    pub text: String,
    pub speaker_name: Option<String>,
    pub output_file_path: String,
    pub model_params_json: Value,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTtsPaths {
    pub base_model: String,
    pub model_scale: String,
    pub src_model_root: PathBuf,
    pub venv_python_path: PathBuf,
    pub init_task_runtime_script_path: PathBuf,
    pub download_models_script_path: PathBuf,
    pub tts_python_script_path: PathBuf,
    pub transcode_script_path: PathBuf,
    pub params_json_path: PathBuf,
}

pub(crate) struct TtsInvocationContext<'a> {
    pub params: &'a LoadedTtsTaskParams,
    pub metrics_log_dir: &'a Path,
    pub runtime: CommonRuntimeOptions,
}

pub(crate) fn build_shared_tts_invocation(
    base_model: &str,
    context: &TtsInvocationContext<'_>,
) -> PythonScriptInvocationSpec {
    PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: base_model.to_string(),
        model_scale: context.params.model_scale.clone(),
        kind: PythonScriptTaskKind::TextToSpeech,
        runtime: PythonScriptRuntimeOptions {
            device: Some(context.runtime.device().to_string()),
            logging_dir: Some(context.metrics_log_dir.to_string_lossy().to_string()),
            attn_implementation: Some(context.runtime.attn_implementation().to_string()),
        },
        args: PythonScriptTaskArgs::TextToSpeech(TTSArgs {
            model_root_path: context.params.model_root_path.clone(),
            speaker_dir_name: context.params.speaker_dir_name.clone(),
            model_params_json: context.params.model_params_json.clone(),
            text: context.params.text.clone(),
            language: context.params.language.clone(),
            speaker: context.params.speaker_name.clone().unwrap_or_default(),
            output_path: resolve_temp_wav_path(
                &context.params.output_file_path,
                context.params.format,
            )
            .to_string_lossy()
            .to_string(),
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

pub(crate) async fn run_common_tts_pipeline(
    service: &LocalService,
    request: TtsPipelineRequest,
    base_model: &str,
) -> Result<()> {
    let task_id = request.task_id;
    let started_at = std::time::Instant::now();

    let result = async {
        mark_tts_running_state(service, task_id).await?;

        let runtime_config = service.runtime_config()?;
        let runtime = CommonRuntimeOptions::from_env_config(&runtime_config);
        let log_dir = resolve_local_log_dir()?;
        let params = load_tts_task_params(service, task_id).await?;
        if params.base_model.trim() != base_model {
            bail!(
                "TTS task base model mismatch: expected {}, got {}",
                base_model,
                params.base_model
            );
        }
        let paths = resolve_tts_paths(service, task_id, base_model, &params.model_scale)?;

        prepare_tts_model_env(
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

        validate_tts_environment(
            &paths,
            Path::new(&params.model_root_path),
            &params.output_file_path,
        )?;

        let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
        let metrics_log_dir = ensure_task_metrics_log_dir(&log_dir)?;
        let invocation_context = TtsInvocationContext {
            params: &params,
            metrics_log_dir: &metrics_log_dir,
            runtime,
        };
        let invocation = build_shared_tts_invocation(&paths.base_model, &invocation_context);

        run_tts_python_command(
            &paths.venv_python_path,
            &paths.tts_python_script_path,
            &paths.src_model_root,
            &paths.params_json_path,
            task_id,
            &log_dir,
            &temp_wav_path,
            &invocation,
            COMMON_TTS_RUN_LABEL,
            COMMON_TTS_START_LOG_MESSAGE,
            COMMON_TTS_OUTPUT_MISSING_LABEL,
        )
        .await?;

        finalize_tts_output(
            &paths.src_model_root,
            &paths.transcode_script_path,
            task_id,
            &log_dir,
            &temp_wav_path,
            &params.output_file_path,
            params.format,
            COMMON_TTS_CONVERT_LABEL,
            COMMON_TTS_OUTPUT_LABEL,
            COMMON_TTS_TEMP_WAV_LABEL,
        )
        .await?;

        mark_tts_completed_state(service, task_id, started_at.elapsed().as_secs() as i64).await
    }
    .await;

    if let Err(err) = result {
        let duration_seconds = started_at.elapsed().as_secs() as i64;
        if let Err(update_err) = mark_tts_failed_state(service, task_id, duration_seconds).await {
            error!(
                error = %update_err,
                task_id,
                model = %base_model,
                "failed to persist tts failure state"
            );
        }
        return Err(err);
    }

    Ok(())
}

pub(crate) async fn load_tts_task_params(
    service: &LocalService,
    task_id: i64,
) -> Result<LoadedTtsTaskParams> {
    let task_query = tts_task_entity::Entity::find()
        .filter(tts_task_entity::Column::HistoryId.eq(task_id))
        .filter(tts_task_entity::Column::Deleted.eq(0));

    let task_detail = task_query
        .one(service.orm())
        .await
        .with_context(|| format!("failed to load tts execution params for task {}", task_id))?
        .ok_or_else(|| anyhow::anyhow!("未找到 TTS 任务执行参数: {}", task_id))?;

    let task_history = task_history_entity::Entity::find_by_id(task_id)
        .filter(task_history_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| format!("failed to load tts task history for task {}", task_id))?
        .ok_or_else(|| anyhow::anyhow!("未找到 TTS 历史任务记录: {}", task_id))?;

    let src_model_root = resolve_src_model_root(service.app_dir())?;
    let model_params = parse_common_tts_model_params(&task_detail.model_params_json)?;
    let selected_speaker_name = task_history.speaker_name_snapshot.trim().to_string();
    let (model_root_path, speaker_dir_name, speaker_name) =
        if let Some(speaker_id) = task_detail.speaker_id {
            let speaker = speaker_entity::Entity::find_by_id(speaker_id)
                .filter(speaker_entity::Column::Deleted.eq(0))
                .one(service.orm())
                .await
                .with_context(|| {
                    format!(
                        "failed to load speaker {} for tts task {}",
                        speaker_id, task_id
                    )
                })?
                .ok_or_else(|| anyhow::anyhow!("未找到 TTS 说话人记录: {}", speaker_id))?;

            if speaker.base_model.trim() != task_detail.base_model.trim() {
                bail!(
                    "TTS speaker base model mismatch: expected {}, got {}",
                    task_detail.base_model,
                    speaker.base_model
                );
            }

            if speaker.source == SpeakerSource::Preset.as_str() {
                (
                    src_model_root
                        .join(MODEL_ARTIFACTS_DIR)
                        .to_string_lossy()
                        .to_string(),
                    None,
                    (!selected_speaker_name.is_empty()).then_some(selected_speaker_name),
                )
            } else {
                (
                    Path::new(service.model_dir()).to_string_lossy().to_string(),
                    speaker_model_dir(Path::new(service.model_dir()), speaker_id)
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string()),
                    (!selected_speaker_name.is_empty()).then_some(selected_speaker_name),
                )
            }
        } else {
            (
                resolve_default_model_root_path(&src_model_root, task_detail.base_model.trim()),
                None,
                None,
            )
        };

    Ok(LoadedTtsTaskParams {
        base_model: task_detail.base_model,
        model_scale: task_detail.model_scale.trim().to_string(),
        model_root_path,
        speaker_dir_name,
        language: task_detail.language,
        format: task_detail
            .format
            .parse()
            .map_err(|err: String| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?,
        text: task_detail.text,
        speaker_name,
        output_file_path: resolve_task_path(
            Path::new(service.data_dir()),
            &task_detail.output_file_path.unwrap_or_default(),
        )
        .to_string_lossy()
        .to_string(),
        model_params_json: model_params.model_params_json,
    })
}

pub(crate) fn parse_common_tts_model_params(
    model_params_json: &str,
) -> Result<CommonTtsModelParams> {
    Ok(CommonTtsModelParams {
        model_params_json: serde_json::from_str(model_params_json)
            .with_context(|| "failed to parse tts model params json")?,
    })
}

pub(crate) async fn mark_tts_running_state(service: &LocalService, task_id: i64) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
        })
        .await?;
    Ok(())
}

pub(crate) async fn mark_tts_completed_state(
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

pub(crate) async fn mark_tts_failed_state(
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

pub(crate) async fn prepare_tts_model_env(
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
                HistoryTaskType::TextToSpeech,
                task_id,
                &log_dir,
                label,
                "tts command completed successfully",
                script_args,
            )
            .await
        },
    )
    .await?;

    let model_info = service
        .get_model_info_by_base_and_scale_impl(base_model, model_scale)
        .await?;
    let download_paths = resolve_model_download_paths(src_model_root, &model_info);

    validate_and_download(
        service,
        bootstrap_paths,
        task_id,
        log_dir,
        &model_info,
        DOWNLOAD_MODEL_ARTIFACTS_LABEL,
        |script_path, working_dir, task_id, log_dir, script_args, label| async move {
            run_pipeline_stage_shell_script(
                &script_path,
                &working_dir,
                HistoryTaskType::TextToSpeech,
                task_id,
                &log_dir,
                label,
                "tts command completed successfully",
                script_args,
            )
            .await
        },
        || validate_model_artifact_paths(base_model, model_scale, &download_paths),
    )
    .await
}

pub(crate) fn validate_tts_environment(
    paths: &ResolvedTtsPaths,
    model_path: &Path,
    output_file_path: &str,
) -> Result<()> {
    for (label, path) in [
        ("TTS venv python", paths.venv_python_path.as_path()),
        (
            "TTS init-task-runtime script",
            paths.init_task_runtime_script_path.as_path(),
        ),
        (
            "TTS download-models script",
            paths.download_models_script_path.as_path(),
        ),
        ("TTS python script", paths.tts_python_script_path.as_path()),
        (
            "TTS transcode script",
            paths.transcode_script_path.as_path(),
        ),
    ] {
        if !path.exists() {
            bail!("{} not found: {}", label, path.display());
        }
    }

    if !model_path.exists() {
        bail!("TTS model path not found: {}", model_path.display());
    }

    let output_path = PathBuf::from(output_file_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
        return Ok(());
    }

    bail!(
        "{} parent directory not found: {}",
        COMMON_TTS_OUTPUT_LABEL,
        output_path.display()
    )
}

pub(crate) fn resolve_tts_paths_base(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_scale: &str,
    src_model_root: PathBuf,
) -> Result<ResolvedTtsPaths> {
    let platform = ScriptPlatform::current();
    let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
    let init_task_runtime_script_path =
        src_model_root.join(platform.init_task_runtime_relative_path());
    let download_models_script_path = src_model_root.join(platform.download_models_relative_path());
    let tts_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "tts.py")?;
    let transcode_script_path = src_model_transcode_script_path(&src_model_root);
    let sample_root = task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::TextToSpeech,
        task_id,
    );
    let params_json_path = tts_params_json_path(&sample_root);

    Ok(ResolvedTtsPaths {
        base_model: base_model.to_string(),
        model_scale: model_scale.to_string(),
        src_model_root,
        venv_python_path,
        init_task_runtime_script_path,
        download_models_script_path,
        tts_python_script_path,
        transcode_script_path,
        params_json_path,
    })
}

pub(crate) fn resolve_tts_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_scale: &str,
) -> Result<ResolvedTtsPaths> {
    let src_model_root = resolve_src_model_root(service.app_dir())?;
    resolve_tts_paths_base(service, task_id, base_model, model_scale, src_model_root)
}

pub(crate) async fn run_tts_python_command(
    venv_python_path: &Path,
    tts_python_script_path: &Path,
    src_model_root: &Path,
    params_json_path: &Path,
    task_id: i64,
    log_dir: &Path,
    temp_wav_path: &Path,
    invocation: &PythonScriptInvocationSpec,
    run_label: &str,
    start_message: &str,
    output_missing_label: &str,
) -> Result<()> {
    info!(
        script = %tts_python_script_path.display(),
        params_file = %params_json_path.display(),
        "{}",
        start_message,
    );

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
    run_python_params_file_invocation(
        venv_python_path,
        tts_python_script_path,
        src_model_root,
        run_label,
        &task_log_path,
        params_json_path,
        invocation,
    )
    .await?;

    if !temp_wav_path.exists() {
        bail!("{}: {}", output_missing_label, temp_wav_path.display());
    }

    Ok(())
}

pub(crate) async fn finalize_tts_output(
    src_model_root: &Path,
    transcode_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    temp_wav_path: &Path,
    final_output_path: &str,
    format: TextToSpeechFormat,
    convert_label: &str,
    output_label: &str,
    temp_wav_label: &str,
) -> Result<()> {
    let final_output_path = Path::new(final_output_path);
    if format == TextToSpeechFormat::Wav {
        replace_output_file(temp_wav_path, final_output_path, output_label)?;
        return Ok(());
    }

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
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

    remove_file_if_exists(temp_wav_path, temp_wav_label)?;
    Ok(())
}
