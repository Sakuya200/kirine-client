use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
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
            LocalService,
        },
        models::{
            SpeakerStatus, TaskStatus, UpdateTaskStatusPayload, VoxCpm2TrainingModelParams,
        },
        pipeline::{
            model_paths::{llm_model_display_name, speaker_model_dir},
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_shared_python_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            TrainingPipelineRequest,
        },
    },
    utils::{
        audio::{is_ogg_audio_path, resolve_normalized_wav_sidecar_path},
        process::{run_logged_command, run_logged_python_script},
        time::now_string,
    },
    Result,
};

use super::{vox_cpm2_base_model_path, VoxCpm2ModelTaskPipeline};

#[derive(Debug)]
struct TrainingParams {
    base_model: BaseModel,
    model_scale: String,
    training_mode: String,
    batch_size: i64,
    epoch_count: i64,
    gradient_accumulation_steps: i64,
    enable_gradient_checkpointing: bool,
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
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
    train_python_script_path: PathBuf,
    init_model_path: PathBuf,
    sample_root: PathBuf,
    input_jsonl: PathBuf,
    output_model_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrainingCommandLabel {
    InitTaskRuntime,
    NormalizeAudio,
    RunTraining,
}

impl TrainingCommandLabel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitTaskRuntime => "initialize voxcpm2 training runtime",
            Self::NormalizeAudio => "normalize voxcpm2 training audio",
            Self::RunTraining => "run voxcpm2 training pipeline",
        }
    }
}

impl VoxCpm2ModelTaskPipeline {
    pub(super) async fn run_training_pipeline_impl(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        let started_at = Instant::now();

        let result = async {
            self.mark_training_running(service, request.task_id, request.speaker_id)
                .await?;
            let params = self
                .load_training_params(service, request.task_id, request.speaker_id)
                .await?;
            let runtime = TrainingRuntimeOptions::from_hardware_type(load_configs()?.hardware_type());
            let paths = self.resolve_training_paths(
                service,
                request.task_id,
                request.speaker_id,
                &request.speaker_name,
                &params.base_model,
                &params.model_scale,
            )?;
            let log_dir = resolve_local_log_dir()?;

            self.prepare_model_env(&paths, request.task_id, &log_dir, runtime)
                .await?;
            self.normalize_training_audio_inputs(&paths, request.task_id, &log_dir)
                .await?;
            self.run_training_command(
                &paths,
                request.task_id,
                &log_dir,
                &params,
                runtime,
            )
            .await?;

            self.mark_training_completed(
                service,
                request.task_id,
                request.speaker_id,
                started_at.elapsed().as_secs() as i64,
            )
            .await
        }
        .await;

        if let Err(err) = result {
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
                error!(error = %update_err, task_id = request.task_id, speaker_id = request.speaker_id, "failed to persist voxcpm2 training failure state");
            }
            return Err(err);
        }

        Ok(())
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
            .with_context(|| format!("failed to load voxcpm2 training params for task {}", task_id))?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到训练任务参数"))?;
        let params = serde_json::from_str::<VoxCpm2TrainingModelParams>(&row.model_params_json)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        Ok(TrainingParams {
            base_model: row.base_model,
            model_scale: row.model_scale.trim().to_string(),
            training_mode: params.training_mode.to_string(),
            batch_size: params.batch_size,
            epoch_count: params.epoch_count,
            gradient_accumulation_steps: params.gradient_accumulation_steps,
            enable_gradient_checkpointing: params.enable_gradient_checkpointing,
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
        let venv_python_path = src_model_venv_python_path(&src_model_root);
        let init_task_runtime_script_path = src_model_root.join(platform.init_task_runtime_relative_path());
        let ffmpeg_python_script_path = src_model_shared_python_script_path(&src_model_root, "ffmpeg.py");
        let train_python_script_path = src_model_model_python_script_path(&src_model_root, base_model, "training.py")?;
        let init_model_path = vox_cpm2_base_model_path(&src_model_root, model_scale)?;
        let resource_name = format!("{}_{}", speaker_id, crate::service::local::sanitize_path_segment(speaker_name));
        let sample_root = task_sample_dir(Path::new(service.data_dir()), crate::service::models::HistoryTaskType::ModelTraining, task_id);
        let input_jsonl = training_index_jsonl_path(&sample_root);
        let output_model_dir = speaker_model_dir(Path::new(service.model_dir()), speaker_id, &resource_name);

        for (label, path) in [
            ("init-task-runtime script", &init_task_runtime_script_path),
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
            format!("failed to create voxcpm2 training output directory: {}", output_model_dir.display())
        })?;

        Ok(TrainingPaths {
            base_model: base_model.to_string(),
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            ffmpeg_python_script_path,
            train_python_script_path,
            init_model_path,
            sample_root,
            input_jsonl,
            output_model_dir,
        })
    }

    async fn prepare_model_env(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        runtime: TrainingRuntimeOptions,
    ) -> Result<()> {
        let mut init_script_args = Vec::new();
        if runtime.is_cpu() {
            init_script_args.push("--cpu-mode".to_string());
        }

        self.run_training_script(
            &paths.init_task_runtime_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            init_script_args,
            TrainingCommandLabel::InitTaskRuntime,
        )
        .await
    }

    async fn normalize_training_audio_inputs(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
    ) -> Result<()> {
        let raw = std::fs::read_to_string(&paths.input_jsonl).with_context(|| {
            format!("failed to read training source jsonl: {}", paths.input_jsonl.display())
        })?;
        let mut normalized_lines = Vec::new();
        let mut normalized_paths = HashMap::<String, String>::new();

        for (line_number, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut value = serde_json::from_str::<serde_json::Value>(trimmed).with_context(|| {
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

        std::fs::write(&paths.input_jsonl, format!("{}\n", normalized_lines.join("\n"))).with_context(|| {
            format!("failed to rewrite normalized training index: {}", paths.input_jsonl.display())
        })?;

        info!(
            sample_root = %paths.sample_root.display(),
            input_jsonl = %paths.input_jsonl.display(),
            "normalized voxcpm2 training audio inputs before train stage"
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
        if !is_ogg_audio_path(input_path) {
            return Ok(input_string);
        }
        if let Some(normalized) = normalized_paths.get(&input_string) {
            return Ok(normalized.clone());
        }
        if !input_path.exists() {
            bail!("训练音频文件不存在: {}", input_path.display());
        }

        let output_path = resolve_normalized_wav_sidecar_path(input_path);
        let task_log_path = task_log_file_path(log_dir, crate::service::models::HistoryTaskType::ModelTraining, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::NormalizeAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--input-path".to_string(),
                input_path.to_string_lossy().to_string(),
                "--output-path".to_string(),
                output_path.to_string_lossy().to_string(),
                "--format".to_string(),
                "wav".to_string(),
            ],
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
    ) -> Result<()> {
        info!(
            train_jsonl = %paths.input_jsonl.display(),
            output_model_dir = %paths.output_model_dir.display(),
            batch_size = params.batch_size,
            epoch_count = params.epoch_count,
            script = %paths.train_python_script_path.display(),
            training_mode = %runtime.mode_label(&paths.base_model)?,
            device = runtime.training_device(),
            "starting voxcpm2 training.py through direct python invocation"
        );

        if !paths.venv_python_path.exists() {
            bail!("Training venv python not found: {}", paths.venv_python_path.display());
        }
        if !paths.init_model_path.exists() {
            bail!("Training base model path not found: {}", paths.init_model_path.display());
        }

        let task_log_path = task_log_file_path(log_dir, crate::service::models::HistoryTaskType::ModelTraining, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;
        let gradient_flag = if params.enable_gradient_checkpointing {
            "--enable-gradient-checkpointing"
        } else {
            "--no-enable-gradient-checkpointing"
        };

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.train_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::RunTraining.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--train-jsonl".to_string(),
                paths.input_jsonl.to_string_lossy().to_string(),
                "--output-model-path".to_string(),
                paths.output_model_dir.to_string_lossy().to_string(),
                "--init-model-path".to_string(),
                paths.init_model_path.to_string_lossy().to_string(),
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
                "--training-mode".to_string(),
                params.training_mode.clone(),
                gradient_flag.to_string(),
            ],
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
        let task_log_path = task_log_file_path(log_dir, crate::service::models::HistoryTaskType::ModelTraining, task_id);
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
            "voxcpm2 training command completed successfully",
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
