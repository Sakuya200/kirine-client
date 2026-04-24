use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::watch;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{
            ensure_task_metrics_log_dir, task_log_file_path, task_sample_dir,
            training_index_jsonl_path,
        },
    },
    config::{load_configs, BaseModel, HardwareType},
    service::{
        local::{
            entity::{speaker as speaker_entity, training_task as training_task_entity},
            sanitize_path_segment, LocalService,
        },
        models::{
            HistoryTaskType, MossTtsLocalTrainingModelParams, SpeakerStatus, TaskStatus,
            UpdateTaskStatusPayload,
        },
        pipeline::{
            model_paths::{llm_model_display_name, speaker_model_dir},
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_shared_python_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, PipelineBootstrapPaths,
            TrainingPipelineRequest, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
    },
    utils::{
        audio::{build_ffmpeg_transcode_args, resolve_normalized_wav_sidecar_path},
        process::{
            run_logged_command, run_logged_python_script, run_logged_python_script_cancellable,
            LoggedCommandResult,
        },
        time::now_string,
    },
    Result,
};

use super::{
    moss_tts_local_download_script_args, moss_tts_local_prepared_model_download_paths,
    MossTtsLocalModelTaskPipeline, MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME, MOSS_TTS_LOCAL_MODEL_NAME,
    MOSS_TTS_LOCAL_RECOMMENDED_AUDIO_SAMPLE_RATE,
};

#[derive(Debug)]
struct TrainingParams {
    base_model: BaseModel,
    model_scale: String,
    batch_size: i64,
    epoch_count: i64,
    gradient_accumulation_steps: i64,
    enable_gradient_checkpointing: bool,
    learning_rate: f64,
    weight_decay: f64,
    warmup_ratio: f64,
    warmup_steps: i64,
    max_grad_norm: f64,
    mixed_precision: String,
    channelwise_loss_weight: String,
    skip_reference_audio_codes: bool,
    prep_batch_size: i64,
    prep_n_vq: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
struct TrainingRuntimeOptions {
    hardware_type: HardwareType,
}

impl TrainingRuntimeOptions {
    const fn from_hardware_type(hardware_type: HardwareType) -> Self {
        Self { hardware_type }
    }

    const fn is_cpu(self) -> bool {
        matches!(self.hardware_type, HardwareType::Cpu)
    }

    const fn training_device(self) -> &'static str {
        if self.is_cpu() {
            "cpu"
        } else {
            "cuda:0"
        }
    }

    fn mode_label(self, base_model: &str) -> Result<String> {
        Ok(format!(
            "{} / {}",
            llm_model_display_name(base_model)?,
            if self.is_cpu() { "CPU" } else { "CUDA" }
        ))
    }
}

#[derive(Debug)]
struct TrainingPaths {
    base_model: BaseModel,
    model_scale: String,
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    download_models_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
    train_python_script_path: PathBuf,
    init_model_path: PathBuf,
    codec_path: PathBuf,
    sample_root: PathBuf,
    input_jsonl: PathBuf,
    output_model_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrainingCommandLabel {
    InitTaskRuntime,
    DownloadModels,
    NormalizeAudio,
    RunTraining,
}

impl TrainingCommandLabel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitTaskRuntime => INIT_MODEL_RUNTIME_LABEL,
            Self::DownloadModels => DOWNLOAD_MODEL_ARTIFACTS_LABEL,
            Self::NormalizeAudio => "normalize moss training audio",
            Self::RunTraining => "run moss training pipeline",
        }
    }
}

impl MossTtsLocalModelTaskPipeline {
    fn cancellation_requested(cancel_rx: &watch::Receiver<bool>) -> bool {
        *cancel_rx.borrow()
    }

    pub(super) async fn run_training_pipeline_impl(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        let started_at = Instant::now();
        let cancel_rx = service.active_training_cancel_receiver(request.task_id)?;

        let result = async {
            self.mark_training_running(service, request.task_id, request.speaker_id)
                .await?;
            let params = self
                .load_training_params(service, request.task_id, request.speaker_id)
                .await?;
            let runtime =
                TrainingRuntimeOptions::from_hardware_type(load_configs()?.hardware_type());
            let paths = self.resolve_training_paths(
                service,
                request.task_id,
                request.speaker_id,
                &request.speaker_name,
                &params.base_model,
                &params.model_scale,
            )?;
            let log_dir = resolve_local_log_dir()?;

            self.prepare_model_env(service, &paths, request.task_id, &log_dir, runtime)
                .await?;
            if Self::cancellation_requested(&cancel_rx) {
                return Ok(LoggedCommandResult::Cancelled);
            }
            self.normalize_training_audio_inputs(&paths, request.task_id, &log_dir)
                .await?;
            if Self::cancellation_requested(&cancel_rx) {
                return Ok(LoggedCommandResult::Cancelled);
            }
            self.run_training_command(
                &paths,
                request.task_id,
                &log_dir,
                &params,
                runtime,
                cancel_rx.clone(),
            )
            .await
        }
        .await;

        match result {
            Ok(LoggedCommandResult::Completed) => {
                self.mark_training_completed(
                    service,
                    request.task_id,
                    request.speaker_id,
                    started_at.elapsed().as_secs() as i64,
                )
                .await?;
                Ok(())
            }
            Ok(LoggedCommandResult::Cancelled) => {
                self.mark_training_cancelled(
                    service,
                    request.task_id,
                    request.speaker_id,
                    started_at.elapsed().as_secs() as i64,
                )
                .await?;
                Ok(())
            }
            Err(err) => {
                let duration_seconds = started_at.elapsed().as_secs() as i64;
                let error_message = err.to_string();
                if let Err(update_err) = self
                    .mark_training_failed(
                        service,
                        request.task_id,
                        request.speaker_id,
                        duration_seconds,
                        &error_message,
                    )
                    .await
                {
                    error!(error = %update_err, task_id = request.task_id, speaker_id = request.speaker_id, "failed to persist moss training failure state");
                }
                Err(err)
            }
        }
    }

    async fn load_training_params(
        &self,
        service: &LocalService,
        task_id: i64,
        speaker_id: i64,
    ) -> Result<TrainingParams> {
        let row = training_task_entity::Entity::find()
            .filter(training_task_entity::Column::HistoryId.eq(task_id))
            .filter(training_task_entity::Column::OutputSpeakerId.eq(speaker_id))
            .filter(training_task_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| format!("failed to load moss training params for task {}", task_id))?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到训练任务参数"))?;
        let params =
            serde_json::from_str::<MossTtsLocalTrainingModelParams>(&row.model_params_json)
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        Ok(TrainingParams {
            base_model: row.base_model,
            model_scale: row.model_scale.trim().to_string(),
            batch_size: params.batch_size.max(1),
            epoch_count: params.epoch_count.max(1),
            gradient_accumulation_steps: params.gradient_accumulation_steps.max(1),
            enable_gradient_checkpointing: params.enable_gradient_checkpointing,
            learning_rate: params.learning_rate.unwrap_or(1e-5),
            weight_decay: params.weight_decay.unwrap_or(0.1),
            warmup_ratio: params.warmup_ratio.unwrap_or(0.03),
            warmup_steps: params.warmup_steps.unwrap_or(0).max(0),
            max_grad_norm: params.max_grad_norm.unwrap_or(1.0),
            mixed_precision: params
                .mixed_precision
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "bf16".to_string()),
            channelwise_loss_weight: params
                .channelwise_loss_weight
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "1,32".to_string()),
            skip_reference_audio_codes: params.skip_reference_audio_codes.unwrap_or(true),
            prep_batch_size: params.prep_batch_size.unwrap_or(16).max(1),
            prep_n_vq: params.prep_n_vq,
        })
    }

    fn resolve_training_paths(
        &self,
        service: &LocalService,
        task_id: i64,
        speaker_id: i64,
        speaker_name: &str,
        base_model: &str,
        model_scale: &str,
    ) -> Result<TrainingPaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, base_model, "ffmpeg.py");
        let train_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "training.py")?;
        let init_model_path = src_model_root
            .join("base-models")
            .join(MOSS_TTS_LOCAL_MODEL_NAME);
        let codec_path = src_model_root
            .join("base-models")
            .join(MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME);
        let sample_root = task_sample_dir(
            Path::new(service.data_dir()),
            HistoryTaskType::ModelTraining,
            task_id,
        );
        let input_jsonl = training_index_jsonl_path(&sample_root);
        let output_model_dir = speaker_model_dir(
            Path::new(service.model_dir()),
            speaker_id,
            &sanitize_path_segment(speaker_name),
        );

        for (label, path) in [
            ("init-task-runtime script", &init_task_runtime_script_path),
            ("download-models script", &download_models_script_path),
            ("ffmpeg python script", &ffmpeg_python_script_path),
            ("train python script", &train_python_script_path),
        ] {
            if !path.exists() {
                bail!("Training {} not found: {}", label, path.display());
            }
        }
        if !input_jsonl.exists() {
            bail!("Training source jsonl not found: {}", input_jsonl.display());
        }
        std::fs::create_dir_all(&output_model_dir).with_context(|| {
            format!(
                "failed to create moss training output directory: {}",
                output_model_dir.display()
            )
        })?;

        Ok(TrainingPaths {
            base_model: base_model.to_string(),
            model_scale: model_scale.to_string(),
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            download_models_script_path,
            ffmpeg_python_script_path,
            train_python_script_path,
            init_model_path,
            codec_path,
            sample_root,
            input_jsonl,
            output_model_dir,
        })
    }

    async fn prepare_model_env(
        &self,
        service: &LocalService,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        runtime: TrainingRuntimeOptions,
    ) -> Result<()> {
        let bootstrap_paths = PipelineBootstrapPaths {
            base_model: &paths.base_model,
            model_scale: &paths.model_scale,
            src_model_root: &paths.src_model_root,
            venv_python_path: &paths.venv_python_path,
            init_task_runtime_script_path: &paths.init_task_runtime_script_path,
            download_models_script_path: &paths.download_models_script_path,
        };

        validate_and_init(
            bootstrap_paths,
            task_id,
            log_dir,
            runtime.is_cpu(),
            TrainingCommandLabel::InitTaskRuntime,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_training_script(
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
            moss_tts_local_download_script_args(&paths.src_model_root, &paths.model_scale)?,
            TrainingCommandLabel::DownloadModels,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_training_script(
                    &script_path,
                    &working_dir,
                    task_id,
                    &log_dir,
                    script_args,
                    label,
                )
                .await
            },
            || self.validate_prepared_model_downloads(paths),
        )
        .await?;

        Ok(())
    }

    fn validate_prepared_model_downloads(&self, paths: &TrainingPaths) -> Result<()> {
        crate::service::pipeline::validate_downloaded_paths(
            &paths.base_model,
            &paths.model_scale,
            &moss_tts_local_prepared_model_download_paths(
                &paths.src_model_root,
                &paths.model_scale,
            )?,
        )
    }

    async fn normalize_training_audio_inputs(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
    ) -> Result<()> {
        let raw = std::fs::read_to_string(&paths.input_jsonl).with_context(|| {
            format!(
                "failed to read training source jsonl: {}",
                paths.input_jsonl.display()
            )
        })?;
        let mut normalized_lines = Vec::new();
        let mut normalized_paths = HashMap::<String, String>::new();

        for (line_number, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut value =
                serde_json::from_str::<serde_json::Value>(trimmed).with_context(|| {
                    format!("training index line {} is not valid json", line_number + 1)
                })?;

            for field in ["audio", "ref_audio"] {
                if let Some(path_value) = value.get(field).and_then(serde_json::Value::as_str) {
                    let normalized = self
                        .normalize_training_audio_path(
                            paths,
                            task_id,
                            log_dir,
                            Path::new(path_value),
                            &mut normalized_paths,
                        )
                        .await?;
                    value[field] = serde_json::Value::String(normalized);
                }
            }

            normalized_lines.push(serde_json::to_string(&value)?);
        }

        std::fs::write(
            &paths.input_jsonl,
            format!("{}\n", normalized_lines.join("\n")),
        )
        .with_context(|| {
            format!(
                "failed to rewrite normalized training index: {}",
                paths.input_jsonl.display()
            )
        })?;

        info!(
            sample_root = %paths.sample_root.display(),
            input_jsonl = %paths.input_jsonl.display(),
            "normalized moss training audio inputs before train stage"
        );

        Ok(())
    }

    async fn normalize_training_audio_path(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        input_path: &Path,
        normalized_paths: &mut HashMap<String, String>,
    ) -> Result<String> {
        let input_string = input_path.to_string_lossy().to_string();
        if let Some(normalized) = normalized_paths.get(&input_string) {
            return Ok(normalized.clone());
        }
        if !input_path.exists() {
            bail!("训练音频文件不存在: {}", input_path.display());
        }

        let output_path = resolve_normalized_wav_sidecar_path(input_path);
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::NormalizeAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            build_ffmpeg_transcode_args(
                input_path,
                &output_path,
                "wav",
                Some(MOSS_TTS_LOCAL_RECOMMENDED_AUDIO_SAMPLE_RATE),
            ),
        )
        .await?;

        let output_string = output_path.to_string_lossy().to_string();
        normalized_paths.insert(input_string, output_string.clone());
        Ok(output_string)
    }

    async fn run_training_command(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        params: &TrainingParams,
        runtime: TrainingRuntimeOptions,
        mut cancel_rx: watch::Receiver<bool>,
    ) -> Result<LoggedCommandResult> {
        info!(
            train_jsonl = %paths.input_jsonl.display(),
            output_model_dir = %paths.output_model_dir.display(),
            batch_size = params.batch_size,
            epoch_count = params.epoch_count,
            learning_rate = params.learning_rate,
            script = %paths.train_python_script_path.display(),
            runtime_mode = %runtime.mode_label(&paths.base_model)?,
            device = runtime.training_device(),
            "starting moss training.py through direct python invocation"
        );

        if !paths.venv_python_path.exists() {
            bail!(
                "Training venv python not found: {}",
                paths.venv_python_path.display()
            );
        }
        if !paths.init_model_path.exists() {
            bail!(
                "Training base model path not found: {}",
                paths.init_model_path.display()
            );
        }
        if !paths.codec_path.exists() {
            bail!(
                "Training codec path not found: {}",
                paths.codec_path.display()
            );
        }

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;
        let skip_reference_audio_codes_flag = if params.skip_reference_audio_codes {
            "--skip-reference-audio-codes"
        } else {
            "--no-skip-reference-audio-codes"
        };
        let gradient_checkpointing_flag = if params.enable_gradient_checkpointing {
            "--enable-gradient-checkpointing"
        } else {
            "--no-enable-gradient-checkpointing"
        };

        let mut script_args = vec![
            "--train-jsonl".to_string(),
            paths.input_jsonl.to_string_lossy().to_string(),
            "--output-model-path".to_string(),
            paths.output_model_dir.to_string_lossy().to_string(),
            "--init-model-path".to_string(),
            paths.init_model_path.to_string_lossy().to_string(),
            "--codec-path".to_string(),
            paths.codec_path.to_string_lossy().to_string(),
            "--logging-dir".to_string(),
            metrics_log_dir.to_string_lossy().to_string(),
            "--batch-size".to_string(),
            params.batch_size.to_string(),
            "--num-epochs".to_string(),
            params.epoch_count.to_string(),
            "--device".to_string(),
            runtime.training_device().to_string(),
            "--gradient-accumulation-steps".to_string(),
            params.gradient_accumulation_steps.to_string(),
            "--learning-rate".to_string(),
            params.learning_rate.to_string(),
            "--weight-decay".to_string(),
            params.weight_decay.to_string(),
            "--warmup-ratio".to_string(),
            params.warmup_ratio.to_string(),
            "--warmup-steps".to_string(),
            params.warmup_steps.to_string(),
            "--max-grad-norm".to_string(),
            params.max_grad_norm.to_string(),
            "--mixed-precision".to_string(),
            params.mixed_precision.clone(),
            "--channelwise-loss-weight".to_string(),
            params.channelwise_loss_weight.clone(),
            "--prep-batch-size".to_string(),
            params.prep_batch_size.to_string(),
            gradient_checkpointing_flag.to_string(),
            skip_reference_audio_codes_flag.to_string(),
        ];
        if let Some(prep_n_vq) = params.prep_n_vq {
            script_args.push("--prep-n-vq".to_string());
            script_args.push(prep_n_vq.to_string());
        }

        run_logged_python_script_cancellable(
            &paths.venv_python_path,
            &paths.train_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::RunTraining.as_str(),
            &task_log_path,
            "python command completed successfully",
            script_args,
            &mut cancel_rx,
        )
        .await
    }

    async fn run_training_script(
        &self,
        script_path: &Path,
        current_dir: &Path,
        task_id: i64,
        log_dir: &Path,
        script_args: Vec<String>,
        label: TrainingCommandLabel,
    ) -> Result<()> {
        let platform = ScriptPlatform::current();
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);
        let mut args = platform.shell_args(script_path);
        args.push("--log-path".to_string());
        args.push(log_dir.to_string_lossy().to_string());
        args.push("--task-log-file".to_string());
        args.push(task_log_path.to_string_lossy().to_string());
        args.extend(script_args);

        run_logged_command(
            Path::new(platform.shell_program()),
            &args,
            current_dir,
            label.as_str(),
            &task_log_path,
            "moss training command completed successfully",
        )
        .await
    }

    async fn mark_training_running(
        &self,
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
        self.update_speaker_status(service, speaker_id, SpeakerStatus::Training)
            .await?;
        Ok(())
    }

    async fn mark_training_completed(
        &self,
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
        self.update_speaker_status(service, speaker_id, SpeakerStatus::Ready)
            .await?;
        Ok(())
    }

    async fn mark_training_cancelled(
        &self,
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
        let _ = self.delete_failed_speaker(service, speaker_id).await?;
        Ok(())
    }

    async fn mark_training_failed(
        &self,
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
        let _ = self.delete_failed_speaker(service, speaker_id).await?;
        Ok(())
    }

    async fn delete_failed_speaker(&self, service: &LocalService, speaker_id: i64) -> Result<bool> {
        service.delete_speaker_info_impl(speaker_id).await
    }

    async fn update_speaker_status(
        &self,
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
}
