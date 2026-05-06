use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tokio::sync::watch;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{ensure_task_metrics_log_dir, task_log_file_path, training_output_jsonl_path},
    },
    config::{BaseModel, EnvConfig, HardwareType},
    service::{
        local::{
            entity::{speaker as speaker_entity, training_task as training_task_entity},
            LocalService,
        },
        models::{HistoryTaskType, SpeakerStatus, TaskStatus, UpdateTaskStatusPayload},
        pipeline::TrainingPipelineRequest,
    },
    utils::{
        audio::{build_ffmpeg_transcode_script_args, resolve_normalized_wav_sidecar_path},
        process::{run_logged_shell_script, LoggedCommandResult},
        time::now_string,
    },
    Result,
};

use super::{
    api::{
        PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
        PythonScriptTaskKind, TrainingArgs,
    },
    model_artifacts::{build_model_download_script_args, resolve_model_download_paths},
    model_paths::llm_model_display_name,
    model_paths::speaker_model_dir,
    run_pipeline_stage_shell_script, run_python_params_file_invocation_cancellable,
    script_paths::{
        resolve_src_model_root, src_model_model_python_script_path,
        src_model_transcode_script_path, src_model_venv_python_path, ScriptPlatform,
    },
    validate_and_download, validate_and_init, PipelineBootstrapPaths,
    DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
};

#[derive(Debug, Clone)]
pub(crate) struct CommonTrainingModelParams {
    pub model_params_json: Value,
    pub batch_size: i64,
    pub epoch_count: i64,
    pub gradient_accumulation_steps: i64,
    pub enable_gradient_checkpointing: bool,
    pub learning_rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedTrainingTaskParams {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub model_params_json: Value,
    pub batch_size: i64,
    pub epoch_count: i64,
    pub gradient_accumulation_steps: i64,
    pub enable_gradient_checkpointing: bool,
    pub learning_rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CommonTrainingPaths {
    pub src_model_root: PathBuf,
    pub model_root_path: PathBuf,
    pub venv_python_path: PathBuf,
    pub init_task_runtime_script_path: PathBuf,
    pub download_models_script_path: PathBuf,
    pub transcode_script_path: PathBuf,
    pub train_python_script_path: PathBuf,
    pub sample_root: PathBuf,
    pub input_jsonl: PathBuf,
    pub output_jsonl: PathBuf,
    pub params_json_path: PathBuf,
    pub output_model_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedTrainingPaths {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub src_model_root: PathBuf,
    pub model_root_path: PathBuf,
    pub venv_python_path: PathBuf,
    pub init_task_runtime_script_path: PathBuf,
    pub download_models_script_path: PathBuf,
    pub transcode_script_path: PathBuf,
    pub train_python_script_path: PathBuf,
    pub sample_root: PathBuf,
    pub input_jsonl: PathBuf,
    pub output_jsonl: PathBuf,
    pub params_json_path: PathBuf,
    pub output_model_dir: PathBuf,
}

const COMMON_TRAINING_NORMALIZE_AUDIO_LABEL: &str = "normalize training audio";
const COMMON_TRAINING_NORMALIZE_LOG_MESSAGE: &str =
    "normalized training audio inputs before train stage";
const COMMON_TRAINING_RUN_LABEL: &str = "run training pipeline";

pub(crate) struct TrainingInvocationContext<'a> {
    pub paths: &'a ResolvedTrainingPaths,
    pub params: &'a LoadedTrainingTaskParams,
    pub speaker_name: &'a str,
    pub metrics_log_dir: &'a Path,
    pub runtime: TrainingRuntimeOptions,
}

#[derive(Debug, Clone)]
pub(crate) struct TrainingRuntimeOptions {
    hardware_type: HardwareType,
    attn_implementation: String,
}

impl TrainingRuntimeOptions {
    pub(crate) fn from_env_config(config: &EnvConfig) -> Self {
        Self {
            hardware_type: config.hardware_type(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
        }
    }

    pub(crate) const fn is_cpu(&self) -> bool {
        matches!(self.hardware_type, HardwareType::Cpu)
    }

    pub(crate) const fn training_device(&self) -> &'static str {
        if self.is_cpu() {
            "cpu"
        } else {
            "cuda:0"
        }
    }

    pub(crate) fn attn_implementation(&self) -> &str {
        &self.attn_implementation
    }

    pub(crate) fn mode_label(&self, base_model: &str) -> Result<String> {
        Ok(format!(
            "{} / {}",
            llm_model_display_name(base_model)?,
            if self.is_cpu() { "CPU" } else { "CUDA" }
        ))
    }
}

pub(crate) fn build_shared_training_invocation(
    base_model: &str,
    context: &TrainingInvocationContext<'_>,
) -> Result<PythonScriptInvocationSpec> {
    Ok(PythonScriptInvocationSpec {
        version: 1,
        base_model: base_model.to_string(),
        kind: PythonScriptTaskKind::Training,
        runtime: PythonScriptRuntimeOptions {
            device: Some(context.runtime.training_device().to_string()),
            logging_dir: Some(context.metrics_log_dir.to_string_lossy().to_string()),
            attn_implementation: Some(context.runtime.attn_implementation().to_string()),
        },
        args: PythonScriptTaskArgs::Training(TrainingArgs {
            model_root_path: context.paths.model_root_path.to_string_lossy().to_string(),
            speaker_dir_name: None,
            model_params_json: context.params.model_params_json.clone(),
            input_jsonl: context.paths.input_jsonl.to_string_lossy().to_string(),
            output_jsonl: context.paths.output_jsonl.to_string_lossy().to_string(),
            output_model_path: context.paths.output_model_dir.to_string_lossy().to_string(),
            batch_size: context.params.batch_size,
            lr: context.params.learning_rate.clone(),
            num_epochs: context.params.epoch_count,
            speaker_name: context.speaker_name.to_string(),
            gradient_accumulation_steps: context.params.gradient_accumulation_steps,
            enable_gradient_checkpointing: context.params.enable_gradient_checkpointing,
        }),
    })
}

fn cancellation_requested(cancel_rx: &watch::Receiver<bool>) -> bool {
    *cancel_rx.borrow()
}

pub(crate) async fn run_common_training_pipeline(
    service: &LocalService,
    request: TrainingPipelineRequest,
    base_model: &str,
) -> Result<()> {
    let task_id = request.task_id;
    let speaker_id = request.speaker_id;
    let speaker_name = request.speaker_name.clone();
    let started_at = Instant::now();

    let result = async {
        mark_training_running_state(service, task_id, speaker_id).await?;

        let mut cancel_rx = service.active_training_cancel_receiver(task_id)?;
        let runtime_config = service.runtime_config()?;
        let runtime = TrainingRuntimeOptions::from_env_config(&runtime_config);
        let params = load_training_task_params(service, task_id, speaker_id, base_model).await?;
        let paths = resolve_training_paths_base(
            service,
            task_id,
            speaker_id,
            &speaker_name,
            &params.base_model,
            &params.model_scale,
        )?;
        let log_dir = resolve_local_log_dir()?;

        info!(
            src_model_root = %paths.src_model_root.display(),
            init_script = %paths.init_task_runtime_script_path.display(),
            download_script = %paths.download_models_script_path.display(),
            training_mode = %runtime.mode_label(&paths.base_model)?,
            "preparing local model environment via required init-task-runtime and optional download-models stages"
        );

        prepare_training_model_env_for_paths(
            service,
            &paths.base_model,
            &paths.model_scale,
            &paths.src_model_root,
            &paths.venv_python_path,
            &paths.init_task_runtime_script_path,
            &paths.download_models_script_path,
            task_id,
            &log_dir,
            runtime.clone(),
        )
        .await?;

        if cancellation_requested(&cancel_rx) {
            return Ok(LoggedCommandResult::Cancelled);
        }

        normalize_training_jsonl_inputs(
            &paths.src_model_root,
            &paths.transcode_script_path,
            &paths.input_jsonl,
            &paths.sample_root,
            task_id,
            &log_dir,
            COMMON_TRAINING_NORMALIZE_AUDIO_LABEL,
            COMMON_TRAINING_NORMALIZE_LOG_MESSAGE,
        )
        .await?;

        if cancellation_requested(&cancel_rx) {
            return Ok(LoggedCommandResult::Cancelled);
        }

        info!(
            script = %paths.train_python_script_path.display(),
            params_file = %paths.params_json_path.display(),
            "starting training.py through params-file python invocation"
        );

        let metrics_log_dir = ensure_task_metrics_log_dir(&log_dir)?;
        let invocation_context = TrainingInvocationContext {
            paths: &paths,
            params: &params,
            speaker_name: &speaker_name,
            metrics_log_dir: &metrics_log_dir,
            runtime,
        };
        let invocation = build_shared_training_invocation(base_model, &invocation_context)?;

        run_training_python_command(
            &paths.venv_python_path,
            &paths.train_python_script_path,
            &paths.src_model_root,
            &paths.model_root_path,
            task_id,
            &log_dir,
            &paths.params_json_path,
            &invocation,
            COMMON_TRAINING_RUN_LABEL,
            &mut cancel_rx,
        )
        .await
    }
    .await;

    match result {
        Ok(LoggedCommandResult::Completed) => {
            mark_training_completed_state(
                service,
                task_id,
                speaker_id,
                started_at.elapsed().as_secs() as i64,
            )
            .await?;
            Ok(())
        }
        Ok(LoggedCommandResult::Cancelled) => {
            mark_training_cancelled_state(
                service,
                task_id,
                speaker_id,
                started_at.elapsed().as_secs() as i64,
            )
            .await?;
            Ok(())
        }
        Err(err) => {
            let duration_seconds = started_at.elapsed().as_secs() as i64;
            let error_message = err.to_string();
            if let Err(update_err) = mark_training_failed_state(
                service,
                task_id,
                speaker_id,
                duration_seconds,
                &error_message,
            )
            .await
            {
                error!(
                    error = %update_err,
                    task_id,
                    speaker_id,
                    model = base_model,
                    "failed to persist training failure state"
                );
            }
            Err(err)
        }
    }
}

pub(crate) async fn load_training_task_params(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
    model_label: &str,
) -> Result<LoadedTrainingTaskParams> {
    let row = training_task_entity::Entity::find()
        .filter(training_task_entity::Column::HistoryId.eq(task_id))
        .filter(training_task_entity::Column::OutputSpeakerId.eq(speaker_id))
        .filter(training_task_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load {} training params for task {}",
                model_label, task_id
            )
        })?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到训练任务参数"))?;
    let params = parse_common_training_model_params(&row.model_params_json)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    Ok(LoadedTrainingTaskParams {
        base_model: row.base_model,
        model_scale: row.model_scale.trim().to_string(),
        model_params_json: params.model_params_json,
        batch_size: params.batch_size,
        epoch_count: params.epoch_count,
        gradient_accumulation_steps: params.gradient_accumulation_steps,
        enable_gradient_checkpointing: params.enable_gradient_checkpointing,
        learning_rate: params.learning_rate,
    })
}

pub(crate) fn resolve_training_output_model_dir(
    model_dir: &Path,
    speaker_id: i64,
    _speaker_name: &str,
) -> PathBuf {
    speaker_model_dir(model_dir, speaker_id)
}

pub(crate) fn resolve_training_paths_base(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
    speaker_name: &str,
    base_model: &str,
    model_scale: &str,
) -> Result<ResolvedTrainingPaths> {
    let common_paths = resolve_common_training_paths(
        service,
        task_id,
        base_model,
        resolve_training_output_model_dir(Path::new(service.model_dir()), speaker_id, speaker_name),
    )?;

    Ok(ResolvedTrainingPaths {
        base_model: base_model.to_string(),
        model_scale: model_scale.to_string(),
        src_model_root: common_paths.src_model_root,
        model_root_path: common_paths.model_root_path,
        venv_python_path: common_paths.venv_python_path,
        init_task_runtime_script_path: common_paths.init_task_runtime_script_path,
        download_models_script_path: common_paths.download_models_script_path,
        transcode_script_path: common_paths.transcode_script_path,
        train_python_script_path: common_paths.train_python_script_path,
        sample_root: common_paths.sample_root,
        input_jsonl: common_paths.input_jsonl,
        output_jsonl: common_paths.output_jsonl,
        params_json_path: common_paths.params_json_path,
        output_model_dir: common_paths.output_model_dir,
    })
}

pub(crate) fn resolve_common_training_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    output_model_dir: PathBuf,
) -> Result<CommonTrainingPaths> {
    let platform = ScriptPlatform::current();
    let src_model_root = resolve_src_model_root(service.app_dir())?;
    let model_root_path = src_model_root.join("base-models");
    let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
    let init_task_runtime_script_path =
        src_model_root.join(platform.init_task_runtime_relative_path());
    let download_models_script_path = src_model_root.join(platform.download_models_relative_path());
    let transcode_script_path = src_model_transcode_script_path(&src_model_root);
    let train_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "training.py")?;
    let sample_root = crate::common::task_paths::task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::ModelTraining,
        task_id,
    );
    let input_jsonl = crate::common::task_paths::training_index_jsonl_path(&sample_root);
    let output_jsonl = training_output_jsonl_path(&sample_root);
    let params_json_path = crate::common::task_paths::training_params_json_path(&sample_root);

    for (label, path) in [
        ("init-task-runtime script", &init_task_runtime_script_path),
        ("download-models script", &download_models_script_path),
        ("transcode script", &transcode_script_path),
        ("train python script", &train_python_script_path),
    ] {
        if !path.exists() {
            anyhow::bail!("Training {} not found: {}", label, path.display());
        }
    }
    if !input_jsonl.exists() {
        anyhow::bail!("Training source jsonl not found: {}", input_jsonl.display());
    }
    std::fs::create_dir_all(&output_model_dir).with_context(|| {
        format!(
            "failed to create training output directory: {}",
            output_model_dir.display()
        )
    })?;

    Ok(CommonTrainingPaths {
        src_model_root,
        model_root_path,
        venv_python_path,
        init_task_runtime_script_path,
        download_models_script_path,
        transcode_script_path,
        train_python_script_path,
        sample_root,
        input_jsonl,
        output_jsonl,
        params_json_path,
        output_model_dir,
    })
}

pub(crate) async fn run_training_stage_shell_script(
    script_path: &Path,
    current_dir: &Path,
    task_id: i64,
    log_dir: &Path,
    script_args: Vec<String>,
    label: &str,
) -> Result<()> {
    run_pipeline_stage_shell_script(
        script_path,
        current_dir,
        HistoryTaskType::ModelTraining,
        task_id,
        log_dir,
        label,
        "training command completed successfully",
        script_args,
    )
    .await
}

pub(crate) async fn prepare_training_model_env(
    service: &LocalService,
    bootstrap_paths: PipelineBootstrapPaths<'_>,
    task_id: i64,
    log_dir: &Path,
    use_cpu_mode: bool,
    download_script_args: Vec<String>,
    download_paths: &[PathBuf],
) -> Result<()> {
    validate_and_init(
        bootstrap_paths,
        task_id,
        log_dir,
        use_cpu_mode,
        INIT_MODEL_RUNTIME_LABEL,
        |script_path, working_dir, task_id, log_dir, script_args, label| async move {
            run_training_stage_shell_script(
                &script_path,
                &working_dir,
                task_id,
                &log_dir,
                script_args,
                label,
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
        download_script_args,
        DOWNLOAD_MODEL_ARTIFACTS_LABEL,
        |script_path, working_dir, task_id, log_dir, script_args, label| async move {
            run_training_stage_shell_script(
                &script_path,
                &working_dir,
                task_id,
                &log_dir,
                script_args,
                label,
            )
            .await
        },
        || {
            super::model_artifacts::validate_model_artifact_paths(
                bootstrap_paths.base_model,
                bootstrap_paths.model_scale,
                download_paths,
            )
        },
    )
    .await
}

pub(crate) async fn prepare_training_model_env_for_paths(
    service: &LocalService,
    base_model: &str,
    model_scale: &str,
    src_model_root: &Path,
    venv_python_path: &Path,
    init_task_runtime_script_path: &Path,
    download_models_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    runtime: TrainingRuntimeOptions,
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

    prepare_training_model_env(
        service,
        bootstrap_paths,
        task_id,
        log_dir,
        runtime.is_cpu(),
        build_model_download_script_args(src_model_root, &model_info)?,
        &download_paths,
    )
    .await
}

pub(crate) async fn normalize_training_audio_path(
    src_model_root: &Path,
    transcode_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    input_path: &Path,
    normalized_paths: &mut HashMap<String, String>,
    label: &str,
) -> Result<String> {
    let input_string = input_path.to_string_lossy().to_string();
    if let Some(normalized) = normalized_paths.get(&input_string) {
        return Ok(normalized.clone());
    }
    if !input_path.exists() {
        anyhow::bail!("训练音频文件不存在: {}", input_path.display());
    }

    let output_path = resolve_normalized_wav_sidecar_path(input_path);
    let task_log_path = crate::common::task_paths::task_log_file_path(
        log_dir,
        HistoryTaskType::ModelTraining,
        task_id,
    );
    let platform = ScriptPlatform::current();

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        transcode_script_path,
        src_model_root,
        label,
        &task_log_path,
        "shell script completed successfully",
        platform.shell_base_args(),
        build_ffmpeg_transcode_script_args(input_path, &output_path, "wav", &task_log_path),
    )
    .await?;

    if !output_path.exists() {
        anyhow::bail!("训练音频转码后未生成 WAV 文件: {}", output_path.display());
    }

    let output_string = output_path.to_string_lossy().to_string();
    normalized_paths.insert(input_string, output_string.clone());
    Ok(output_string)
}

pub(crate) async fn normalize_training_jsonl_inputs(
    src_model_root: &Path,
    transcode_script_path: &Path,
    input_jsonl: &Path,
    sample_root: &Path,
    task_id: i64,
    log_dir: &Path,
    normalize_label: &str,
    log_message: &str,
) -> Result<()> {
    let raw = std::fs::read_to_string(input_jsonl).with_context(|| {
        format!(
            "failed to read training source jsonl: {}",
            input_jsonl.display()
        )
    })?;

    let mut normalized_paths = HashMap::<String, String>::new();
    let mut normalized_lines = Vec::new();
    let mut changed = false;
    let mut sample_count = 0usize;

    for (line_number, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        sample_count += 1;

        let mut value = serde_json::from_str::<Value>(trimmed).with_context(|| {
            format!("training index line {} is not valid json", line_number + 1)
        })?;
        let object = value.as_object_mut().ok_or_else(|| {
            anyhow::anyhow!(
                "training index line {} must be a json object",
                line_number + 1
            )
        })?;

        for field in ["audio", "ref_audio"] {
            let Some(path_value) = object
                .get(field)
                .and_then(Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };

            let normalized = normalize_training_audio_path(
                src_model_root,
                transcode_script_path,
                task_id,
                log_dir,
                Path::new(&path_value),
                &mut normalized_paths,
                normalize_label,
            )
            .await?;

            if normalized != path_value {
                object.insert(field.to_string(), Value::String(normalized));
                changed = true;
            }
        }

        normalized_lines.push(serde_json::to_string(&value)?);
    }

    if sample_count == 0 {
        anyhow::bail!("训练索引文件中没有可用样本: {}", input_jsonl.display());
    }

    if !changed {
        return Ok(());
    }

    std::fs::write(input_jsonl, format!("{}\n", normalized_lines.join("\n"))).with_context(
        || {
            format!(
                "failed to rewrite normalized training index: {}",
                input_jsonl.display()
            )
        },
    )?;

    info!(
        sample_root = %sample_root.display(),
        input_jsonl = %input_jsonl.display(),
        "{}",
        log_message,
    );

    Ok(())
}

pub(crate) fn parse_common_training_model_params(
    raw_model_params_json: &str,
) -> Result<CommonTrainingModelParams> {
    let model_params_json = serde_json::from_str::<Value>(raw_model_params_json)
        .context("failed to parse training model params json")?;
    let object = model_params_json
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("training model params payload must be a json object"))?;

    Ok(CommonTrainingModelParams {
        batch_size: parse_required_training_i64(object, "batchSize")?.max(1),
        epoch_count: parse_required_training_i64(object, "epochCount")?.max(1),
        gradient_accumulation_steps: parse_required_training_i64(
            object,
            "gradientAccumulationSteps",
        )?
        .max(1),
        enable_gradient_checkpointing: parse_required_training_bool(
            object,
            "enableGradientCheckpointing",
        )?,
        learning_rate: parse_optional_training_string(object, "learningRate"),
        model_params_json,
    })
}

pub(crate) async fn run_training_python_command(
    venv_python_path: &Path,
    train_python_script_path: &Path,
    src_model_root: &Path,
    model_root_path: &Path,
    task_id: i64,
    log_dir: &Path,
    params_json_path: &Path,
    invocation: &PythonScriptInvocationSpec,
    label: &str,
    cancel_rx: &mut watch::Receiver<bool>,
) -> Result<LoggedCommandResult> {
    if !venv_python_path.exists() {
        anyhow::bail!(
            "Training venv python not found: {}",
            venv_python_path.display()
        );
    }
    if !model_root_path.exists() {
        anyhow::bail!(
            "Training model root path not found: {}",
            model_root_path.display()
        );
    }

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);
    run_python_params_file_invocation_cancellable(
        venv_python_path,
        train_python_script_path,
        src_model_root,
        label,
        &task_log_path,
        params_json_path,
        invocation,
        cancel_rx,
    )
    .await
}

fn parse_required_training_i64(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<i64> {
    let value = object.get(field).ok_or_else(|| {
        anyhow::anyhow!("training model params missing required field: {}", field)
    })?;
    match value {
        Value::Number(number) => number.as_i64().ok_or_else(|| {
            anyhow::anyhow!(
                "training model params field {} is not a valid integer",
                field
            )
        }),
        Value::String(text) => text.trim().parse::<i64>().with_context(|| {
            format!(
                "training model params field {} is not a valid integer",
                field
            )
        }),
        _ => Err(anyhow::anyhow!(
            "training model params field {} must be an integer",
            field
        )),
    }
}

fn parse_required_training_bool(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<bool> {
    let value = object.get(field).ok_or_else(|| {
        anyhow::anyhow!("training model params missing required field: {}", field)
    })?;
    match value {
        Value::Bool(flag) => Ok(*flag),
        Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(anyhow::anyhow!(
                "training model params field {} must be a boolean",
                field
            )),
        },
        _ => Err(anyhow::anyhow!(
            "training model params field {} must be a boolean",
            field
        )),
    }
}

fn parse_optional_training_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Option<String> {
    object.get(field).and_then(|value| match value {
        Value::Null => None,
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        other => {
            let text = other.to_string();
            let trimmed = text.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
    })
}

pub(crate) async fn mark_training_running_state(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
            error_message: None,
        })
        .await?;
    update_training_speaker_status(service, speaker_id, SpeakerStatus::Training).await
}

pub(crate) async fn mark_training_completed_state(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
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
    update_training_speaker_status(service, speaker_id, SpeakerStatus::Ready).await
}

pub(crate) async fn mark_training_cancelled_state(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
    duration_seconds: i64,
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Cancelled,
            duration_seconds: Some(duration_seconds),
            error_message: Some("模型训练任务已由用户终止。".to_string()),
        })
        .await?;
    let _ = delete_failed_training_speaker(service, speaker_id).await?;
    Ok(())
}

pub(crate) async fn mark_training_failed_state(
    service: &LocalService,
    task_id: i64,
    speaker_id: i64,
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
    let _ = delete_failed_training_speaker(service, speaker_id).await?;
    Ok(())
}

pub(crate) async fn delete_failed_training_speaker(
    service: &LocalService,
    speaker_id: i64,
) -> Result<bool> {
    service.delete_speaker_info_impl(speaker_id).await
}

pub(crate) async fn update_training_speaker_status(
    service: &LocalService,
    speaker_id: i64,
    status: SpeakerStatus,
) -> Result<()> {
    let modify_time = now_string()?;
    let speaker = speaker_entity::Entity::find_by_id(speaker_id)
        .filter(speaker_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| format!("failed to load speaker {} before status update", speaker_id))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到目标说话人"))?;

    let mut active_model: speaker_entity::ActiveModel = speaker.into();
    active_model.status = sea_orm::ActiveValue::Set(status.as_str().to_string());
    active_model.modify_time = sea_orm::ActiveValue::Set(modify_time);
    active_model
        .update(service.orm())
        .await
        .with_context(|| format!("failed to update speaker status for {}", speaker_id))?;
    Ok(())
}
