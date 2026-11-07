use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Context;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde_json::Value;
use tokio::sync::watch;
use tracing::{error, info, warn};

use crate::error::{codes, AppError};
use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{ensure_task_metrics_log_dir, task_log_file_path, training_output_jsonl_path},
    },
    config::{BaseModel, EnvConfig, HardwareType},
    service::{
        local::{
            entity::{
                speaker as speaker_entity, task_history as task_history_entity,
                training_task as training_task_entity,
            },
            LocalService,
        },
        models::{HistoryTaskType, SpeakerStatus, TaskStatus, UpdateTaskStatusPayload},
        pipeline::TrainingPipelineRequest,
    },
    utils::{
        audio::{build_ffmpeg_transcode_script_args, resolve_normalized_wav_sidecar_path},
        process::{read_task_log_tail, run_logged_shell_script, LoggedCommandResult},
        time::now_string,
    },
    Result,
};

use super::{
    api::{
        PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
        PythonScriptTaskKind, TrainingArgs,
    },
    model_paths::speaker_model_dir,
    run_llm_task_invocation_cancellable, run_pipeline_stage_shell_script,
    script_paths::{
        resolve_src_model_root, src_model_begin_llm_task_script_path,
        src_model_model_python_script_path, src_model_transcode_script_path, ScriptPlatform,
    },
};

#[derive(Debug, Clone)]
pub(crate) struct CommonTrainingModelParams {
    pub model_params_json: Value,
    pub batch_size: i64,
    pub epoch_count: i64,
    pub gradient_accumulation_steps: i64,
    pub learning_rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedTrainingTaskParams {
    pub base_model: BaseModel,
    pub model_version: String,
    pub device: HardwareType,
    pub model_params_json: Value,
    pub batch_size: i64,
    pub epoch_count: i64,
    pub gradient_accumulation_steps: i64,
    pub learning_rate: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct CommonTrainingPaths {
    pub src_model_root: PathBuf,
    pub model_root_path: PathBuf,
    pub begin_llm_task_script_path: PathBuf,
    pub ensure_torch_runtime_script_path: PathBuf,
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
    pub model_version: String,
    pub src_model_root: PathBuf,
    pub model_root_path: PathBuf,
    pub begin_llm_task_script_path: PathBuf,
    pub ensure_torch_runtime_script_path: PathBuf,
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
    device: HardwareType,
    attn_implementation: String,
}

impl TrainingRuntimeOptions {
    pub(crate) fn from_task_device(task_device: HardwareType, config: &EnvConfig) -> Result<Self> {
        Ok(Self {
            device: task_device,
            attn_implementation: config.attn_implementation().as_str().to_string(),
        })
    }

    pub(crate) fn is_cpu(&self) -> bool {
        self.device == HardwareType::Cpu
    }

    pub(crate) fn training_device(&self) -> &str {
        self.device.runtime_arg()
    }

    pub(crate) fn attn_implementation(&self) -> &str {
        &self.attn_implementation
    }

    pub(crate) fn mode_label(&self, base_model: &str) -> Result<String> {
        Ok(format!(
            "{} / {}",
            base_model,
            if self.is_cpu() { "CPU" } else { "CUDA" }
        ))
    }
}

pub(crate) fn build_shared_training_invocation(
    base_model: &str,
    context: &TrainingInvocationContext<'_>,
) -> Result<PythonScriptInvocationSpec> {
    Ok(PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: base_model.to_string(),
        model_version: context.paths.model_version.clone(),
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

        let mut cancel_rx =
            service.active_task_cancel_receiver(task_id, HistoryTaskType::ModelTraining)?;
        let runtime_config = service.runtime_config()?;
        let params = load_training_task_params(service, task_id, speaker_id, base_model).await?;
        let runtime = TrainingRuntimeOptions::from_task_device(params.device, &runtime_config)?;
        let paths = resolve_training_paths_base(
            service,
            task_id,
            speaker_id,
            &speaker_name,
            &params.base_model,
            &params.model_version,
        )?;
        let log_dir = resolve_local_log_dir(&runtime_config)?;

        info!(
            src_model_root = %paths.src_model_root.display(),
            ensure_torch_runtime_script = %paths.ensure_torch_runtime_script_path.display(),
            training_mode = %runtime.mode_label(&paths.base_model)?,
            "preparing local model environment via torch runtime ensure stage"
        );

        prepare_training_model_env_for_paths(
            service,
            &paths.base_model,
            &paths.model_version,
            &paths.src_model_root,
            &paths.ensure_torch_runtime_script_path,
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
            &log_dir,
            COMMON_TRAINING_NORMALIZE_AUDIO_LABEL,
            COMMON_TRAINING_NORMALIZE_LOG_MESSAGE,
            runtime_config
                .training
                .normalization_concurrency
                .unwrap_or_else(default_training_normalization_concurrency),
            runtime_config
                .training
                .normalization_failure_rate_percent
                .unwrap_or(DEFAULT_TRAINING_NORMALIZATION_FAILURE_RATE_PERCENT),
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
            &paths.begin_llm_task_script_path,
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
            super::record_task_failure_log(
                service,
                HistoryTaskType::ModelTraining,
                task_id,
                base_model,
                &err,
            );
            if let Err(update_err) =
                mark_training_failed_state(service, task_id, speaker_id, duration_seconds).await
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
    let task_history = task_history_entity::Entity::find_by_id(task_id)
        .filter(task_history_entity::Column::Deleted.eq(0))
        .one(service.orm())
        .await
        .with_context(|| {
            format!(
                "failed to load {} training task history for task {}",
                model_label, task_id
            )
        })?
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到训练任务历史记录"))?;
    let params = parse_common_training_model_params(&row.model_params_json)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    Ok(LoadedTrainingTaskParams {
        base_model: row.base_model,
        model_version: row.model_version.trim().to_string(),
        device: task_history
            .device
            .parse::<HardwareType>()
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?,
        model_params_json: params.model_params_json,
        batch_size: params.batch_size,
        epoch_count: params.epoch_count,
        gradient_accumulation_steps: params.gradient_accumulation_steps,
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
    model_version: &str,
) -> Result<ResolvedTrainingPaths> {
    let common_paths = resolve_common_training_paths(
        service,
        task_id,
        base_model,
        resolve_training_output_model_dir(Path::new(service.model_dir()), speaker_id, speaker_name),
    )?;

    Ok(ResolvedTrainingPaths {
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        src_model_root: common_paths.src_model_root,
        model_root_path: common_paths.model_root_path,
        begin_llm_task_script_path: common_paths.begin_llm_task_script_path,
        ensure_torch_runtime_script_path: common_paths.ensure_torch_runtime_script_path,
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
    let begin_llm_task_script_path = src_model_begin_llm_task_script_path(&src_model_root);
    let ensure_torch_runtime_script_path =
        src_model_root.join(platform.ensure_torch_runtime_relative_path());
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
        (
            "ensure-torch-runtime script",
            &ensure_torch_runtime_script_path,
        ),
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
        begin_llm_task_script_path,
        ensure_torch_runtime_script_path,
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
    base_model: &str,
    model_version: &str,
    src_model_root: &Path,
    ensure_torch_runtime_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    use_cpu_mode: bool,
) -> Result<()> {
    let model_downloaded = service
        .model_downloaded_impl(base_model, model_version)
        .await?;
    if !model_downloaded {
        return Err(AppError::coded(
            codes::MODEL_NOT_INSTALLED,
            format!(
                "模型 {}:{} 未安装，请先在模型管理页安装后再执行任务",
                base_model, model_version
            ),
        )
        .with_param("model", format!("{}:{}", base_model, model_version))
        .into_anyhow());
    }

    if !ensure_torch_runtime_script_path.exists() {
        anyhow::bail!(
            "训练 Torch 运行时校验脚本不存在: {}",
            ensure_torch_runtime_script_path.display()
        );
    }

    let mut script_args = vec!["--base-model".to_string(), base_model.to_string()];
    if use_cpu_mode {
        script_args.push("--cpu-mode".to_string());
    }

    run_training_stage_shell_script(
        ensure_torch_runtime_script_path,
        src_model_root,
        task_id,
        log_dir,
        script_args,
        "校验并切换 Torch 运行时",
    )
    .await
}

pub(crate) async fn prepare_training_model_env_for_paths(
    service: &LocalService,
    base_model: &str,
    model_version: &str,
    src_model_root: &Path,
    ensure_torch_runtime_script_path: &Path,
    task_id: i64,
    log_dir: &Path,
    runtime: TrainingRuntimeOptions,
) -> Result<()> {
    prepare_training_model_env(
        service,
        base_model,
        model_version,
        src_model_root,
        ensure_torch_runtime_script_path,
        task_id,
        log_dir,
        runtime.is_cpu(),
    )
    .await
}

/// 默认失败率阈值：失败样本占比超过该值（百分比）才整体失败，可经 [training] 配置覆盖。
pub(crate) const DEFAULT_TRAINING_NORMALIZATION_FAILURE_RATE_PERCENT: u8 = 10;

/// 默认规范化并发度：min(4, max(2, 逻辑核数/2))。
pub(crate) fn default_training_normalization_concurrency() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(4);
    (cpus / 2).clamp(2, 4)
}

/// 单个训练音频规范化：经 transcode_audio.ps1 脚本链转出单声道 WAV sidecar。
/// 日志写独立样本文件——并发进程各自持有句柄，避免交错写坏同一任务日志。
async fn transcode_training_sample(
    src_model_root: &Path,
    transcode_script_path: &Path,
    input_path: &Path,
    sample_log_path: &Path,
    normalize_label: &str,
) -> Result<String> {
    if !input_path.exists() {
        anyhow::bail!("训练音频文件不存在: {}", input_path.display());
    }

    let output_path = resolve_normalized_wav_sidecar_path(input_path);
    let platform = ScriptPlatform::current();

    run_logged_shell_script(
        Path::new(platform.shell_program()),
        transcode_script_path,
        src_model_root,
        normalize_label,
        sample_log_path,
        "shell script completed successfully",
        platform.shell_base_args(),
        build_ffmpeg_transcode_script_args(input_path, &output_path, "wav", sample_log_path),
    )
    .await?;

    if !output_path.exists() {
        anyhow::bail!("训练音频转码后未生成 WAV 文件: {}", output_path.display());
    }

    Ok(output_path.to_string_lossy().to_string())
}

/// 规范化失败条目：输入路径 + 该样本独立日志的 tail（统一汇总报错用）。
#[derive(Debug, Clone)]
pub struct NormalizationFailure {
    pub index: usize,
    pub input: String,
    pub log_tail: String,
}

/// 并发执行训练音频规范化：`tokio::sync::Semaphore` 控并发、`JoinSet` 收集乱序结果，
/// 原子完成计数按约 10% 步长打进度日志。每个样本仍走完整脚本链
/// （powershell → transcode_audio.ps1 → ffmpeg.exe），并发只提高吞吐不改变脚本边界。
///
/// 失败样本收集 `(序号, 输入, 日志 tail)`；失败率超过阈值才整体失败（错误信息含全部
/// 失败样本汇总），低于阈值返回失败列表由调用方降级（保留原始路径）。
pub async fn run_training_transcodes<F, Fut>(
    pending: Vec<String>,
    concurrency: usize,
    failure_rate_percent: u8,
    log_dir: &Path,
    spawn_one: F,
) -> Result<(HashMap<String, String>, Vec<NormalizationFailure>)>
where
    F: Fn(String, PathBuf) -> Fut + Clone + Send + 'static,
    Fut: std::future::Future<Output = Result<String>> + Send,
{
    let total = pending.len();
    let concurrency = concurrency.max(1).min(total.max(1));
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let completed = Arc::new(AtomicUsize::new(0));
    let progress_step = (total / 10).max(1);
    let mut join_set = tokio::task::JoinSet::new();

    for (index, input) in pending.into_iter().enumerate() {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("normalization semaphore closed");
        let log_dir = log_dir.to_path_buf();
        let spawn_one = spawn_one.clone();
        let completed = Arc::clone(&completed);
        join_set.spawn(async move {
            // 许可 guard 持有到任务结束，控制同时在飞的转码进程数
            let _permit = permit;
            let sample_log_path = log_dir.join(format!("normalize-sample-{:04}.log", index + 1));
            let result = spawn_one(input.clone(), sample_log_path.clone()).await;
            let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
            if done == total || done % progress_step == 0 {
                info!(done, total, "normalizing training audio progress");
            }
            (index, input, sample_log_path, result)
        });
    }

    let mut normalized_paths = HashMap::<String, String>::new();
    let mut failures = Vec::new();
    while let Some(joined) = join_set.join_next().await {
        let (index, input, sample_log_path, result) =
            joined.context("normalization task panicked")?;
        match result {
            Ok(output) => {
                normalized_paths.insert(input, output);
            }
            Err(err) => {
                warn!(
                    error = %err,
                    input = %input,
                    log = %sample_log_path.display(),
                    "training audio normalization failed"
                );
                failures.push(NormalizationFailure {
                    index,
                    input,
                    log_tail: read_task_log_tail(&sample_log_path),
                });
            }
        }
    }

    if !failures.is_empty() {
        let actual_percent = (failures.len() as f64 / total.max(1) as f64 * 100.0).ceil() as u64;
        if actual_percent > failure_rate_percent as u64 {
            let summary = failures
                .iter()
                .map(|failure| {
                    format!(
                        "#{} {}: {}",
                        failure.index + 1,
                        failure.input,
                        failure.log_tail.lines().next_back().unwrap_or("").trim()
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            anyhow::bail!(
                "训练音频规范化失败率 {}% 超过阈值 {}%（{}/{}）:\n{}",
                actual_percent,
                failure_rate_percent,
                failures.len(),
                total,
                summary
            );
        }
        warn!(
            failures = failures.len(),
            total, "training audio normalization failures below threshold, keeping original paths"
        );
    }

    Ok((normalized_paths, failures))
}

pub(crate) async fn normalize_training_jsonl_inputs(
    src_model_root: &Path,
    transcode_script_path: &Path,
    input_jsonl: &Path,
    sample_root: &Path,
    log_dir: &Path,
    normalize_label: &str,
    log_message: &str,
    concurrency: usize,
    failure_rate_percent: u8,
) -> Result<()> {
    let raw = std::fs::read_to_string(input_jsonl).with_context(|| {
        format!(
            "failed to read training source jsonl: {}",
            input_jsonl.display()
        )
    })?;

    // 第一遍：解析所有行，收集待转码音频路径（同一文件多处引用只转码一次）
    let mut parsed_values = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut seen = HashSet::<String>::new();
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
            if seen.insert(path_value.clone()) {
                pending.push(path_value);
            }
        }

        parsed_values.push(value);
    }

    if sample_count == 0 {
        anyhow::bail!("训练索引文件中没有可用样本: {}", input_jsonl.display());
    }

    // 并发规范化；失败率超阈值时在引擎内整体失败，低于阈值则失败样本保留原始路径
    let src_model_root_owned = src_model_root.to_path_buf();
    let transcode_script_path_owned = transcode_script_path.to_path_buf();
    let normalize_label_owned = normalize_label.to_string();
    let (normalized_paths, _failures) = run_training_transcodes(
        pending,
        concurrency,
        failure_rate_percent,
        log_dir,
        move |input, sample_log_path| {
            let src_model_root = src_model_root_owned.clone();
            let transcode_script_path = transcode_script_path_owned.clone();
            let normalize_label = normalize_label_owned.clone();
            async move {
                transcode_training_sample(
                    &src_model_root,
                    &transcode_script_path,
                    Path::new(&input),
                    &sample_log_path,
                    &normalize_label,
                )
                .await
            }
        },
    )
    .await?;

    // 第二遍：回填规范化路径；失败样本（不在 normalized_paths 中）保留原始值
    let mut changed = false;
    for value in &mut parsed_values {
        let object = value.as_object_mut().expect("parsed training jsonl object");
        for field in ["audio", "ref_audio"] {
            let Some(path_value) = object.get(field).and_then(Value::as_str) else {
                continue;
            };
            let Some(normalized) = normalized_paths.get(path_value) else {
                continue;
            };
            if normalized != path_value {
                object.insert(field.to_string(), Value::String(normalized.clone()));
                changed = true;
            }
        }
    }

    if !changed {
        return Ok(());
    }

    let normalized_lines = parsed_values
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?;
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
        learning_rate: parse_optional_training_string(object, "learningRate"),
        model_params_json,
    })
}

pub(crate) async fn run_training_python_command(
    begin_llm_task_script_path: &Path,
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
    if !begin_llm_task_script_path.exists() {
        anyhow::bail!(
            "任务执行脚本不存在: {}",
            begin_llm_task_script_path.display()
        );
    }
    if !model_root_path.exists() {
        anyhow::bail!("训练模型根路径不存在: {}", model_root_path.display());
    }

    let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);
    run_llm_task_invocation_cancellable(
        begin_llm_task_script_path,
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
) -> Result<()> {
    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Failed,
            duration_seconds: Some(duration_seconds),
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
