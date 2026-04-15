use std::{
    collections::HashMap,
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::{
    common::{
        local_paths::resolve_local_log_dir,
        task_paths::{
            ensure_task_metrics_log_dir, task_log_file_path, task_sample_dir,
            training_index_jsonl_path, training_output_jsonl_path,
        },
    },
    config::{load_configs, save_configs, BaseModel, HardwareType, QloraMode},
    service::{
        local::{
            entity::{speaker as speaker_entity, training_task as training_task_entity},
            sanitize_path_segment, LocalService,
        },
        models::{HistoryTaskType, SpeakerStatus, TaskStatus, UpdateTaskStatusPayload},
        pipeline::{
            model_paths::{llm_model_display_name, llm_model_paths},
            qwen3_tts::Qwen3TTSModelTaskPipeline,
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

#[derive(Debug, Serialize, Deserialize)]
struct TrainingIndexEntry {
    audio: String,
    text: String,
    ref_audio: String,
}

#[derive(Debug)]
struct TrainingParams {
    base_model: BaseModel,
    batch_size: i64,
    epoch_count: i64,
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

    const fn encode_device(self) -> &'static str {
        if self.is_cpu() {
            "cpu"
        } else {
            "cuda:0"
        }
    }

    const fn training_device(self) -> &'static str {
        if self.is_cpu() {
            "cpu"
        } else {
            "cuda:0"
        }
    }

    fn mode_label(self, base_model: BaseModel) -> String {
        format!(
            "{} / {}",
            llm_model_display_name(base_model),
            if self.is_cpu() { "CPU" } else { "CUDA" }
        )
    }
}

#[derive(Debug)]
struct TrainingPaths {
    base_model: BaseModel,
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    download_models_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
    encode_python_script_path: PathBuf,
    train_python_script_path: PathBuf,
    tokenizer_model_path: PathBuf,
    init_model_path: PathBuf,
    sample_root: PathBuf,
    input_jsonl: PathBuf,
    train_jsonl: PathBuf,
    output_model_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrainingCommandLabel {
    InitTaskRuntime,
    DownloadModels,
    NormalizeAudio,
    EncodeAudio,
    RunTraining,
}

impl TrainingCommandLabel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitTaskRuntime => "initialize local task runtime",
            Self::DownloadModels => "download local base models",
            Self::NormalizeAudio => "normalize training audio",
            Self::EncodeAudio => "encode audio data",
            Self::RunTraining => "run training pipeline",
        }
    }
}

impl Qwen3TTSModelTaskPipeline {
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
            let runtime =
                TrainingRuntimeOptions::from_hardware_type(load_configs()?.hardware_type());
            let paths = self.resolve_training_paths(
                service,
                request.task_id,
                request.speaker_id,
                &request.speaker_name,
                params.base_model,
            )?;
            let log_dir = resolve_local_log_dir()?;

            self.prepare_model_env(service, &paths, request.task_id, &log_dir, runtime)
                .await?;
            self.normalize_training_audio_inputs(&paths, request.task_id, &log_dir)
                .await?;
            self.run_encode_audio(&paths, request.task_id, &log_dir, runtime)
                .await?;
            self.run_training_command(
                &paths,
                request.task_id,
                &log_dir,
                &request.speaker_name,
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
                error!(error = %update_err, task_id = request.task_id, speaker_id = request.speaker_id, "failed to persist training failure state");
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
            .with_context(|| format!("failed to load training params for task {}", task_id))?
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "未找到训练任务参数"))?;

        Ok(TrainingParams {
            base_model: row
                .base_model
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            batch_size: row.batch_size,
            epoch_count: row.epoch_count,
        })
    }

    fn resolve_training_paths(
        &self,
        service: &LocalService,
        task_id: i64,
        speaker_id: i64,
        speaker_name: &str,
        base_model: BaseModel,
    ) -> Result<TrainingPaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, "ffmpeg.py");
        let encode_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "encode_audio.py");
        let train_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "training.py");
        let model_paths = llm_model_paths(base_model);
        let tokenizer_model_path = model_paths.training_tokenizer_model_path(&src_model_root);
        let init_model_path = model_paths.training_init_model_path(&src_model_root);
        let resource_name = format!("{}_{}", speaker_id, sanitize_path_segment(speaker_name));
        let sample_root = task_sample_dir(
            Path::new(service.data_dir()),
            HistoryTaskType::ModelTraining,
            task_id,
        );
        let input_jsonl = training_index_jsonl_path(&sample_root);
        let train_jsonl = training_output_jsonl_path(&sample_root);
        let output_model_dir = Path::new(service.model_dir()).join(&resource_name);

        for (label, path) in [
            ("init-task-runtime script", &init_task_runtime_script_path),
            ("download-models script", &download_models_script_path),
            ("ffmpeg python script", &ffmpeg_python_script_path),
            ("encode python script", &encode_python_script_path),
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
                "failed to create training output directory: {}",
                output_model_dir.display()
            )
        })?;

        Ok(TrainingPaths {
            base_model,
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            download_models_script_path,
            ffmpeg_python_script_path,
            encode_python_script_path,
            train_python_script_path,
            tokenizer_model_path,
            init_model_path,
            sample_root,
            input_jsonl,
            train_jsonl,
            output_model_dir,
        })
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
        let mut entries = Vec::new();

        for (line_number, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let entry = serde_json::from_str::<TrainingIndexEntry>(trimmed).with_context(|| {
                format!("training index line {} is not valid json", line_number + 1)
            })?;
            entries.push(entry);
        }

        if entries.is_empty() {
            bail!(
                "训练索引文件中没有可用样本: {}",
                paths.input_jsonl.display()
            );
        }

        let mut normalized_paths = HashMap::<String, String>::new();
        let mut changed = false;

        for entry in &mut entries {
            let normalized_audio = self
                .normalize_training_audio_path(
                    paths,
                    task_id,
                    log_dir,
                    Path::new(&entry.audio),
                    &mut normalized_paths,
                )
                .await?;
            if normalized_audio != entry.audio {
                entry.audio = normalized_audio;
                changed = true;
            }

            let normalized_ref_audio = self
                .normalize_training_audio_path(
                    paths,
                    task_id,
                    log_dir,
                    Path::new(&entry.ref_audio),
                    &mut normalized_paths,
                )
                .await?;
            if normalized_ref_audio != entry.ref_audio {
                entry.ref_audio = normalized_ref_audio;
                changed = true;
            }
        }

        if !changed {
            return Ok(());
        }

        let jsonl = entries
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()?
            .join("\n");
        std::fs::write(&paths.input_jsonl, format!("{}\n", jsonl)).with_context(|| {
            format!(
                "failed to rewrite normalized training index: {}",
                paths.input_jsonl.display()
            )
        })?;

        info!(
            sample_root = %paths.sample_root.display(),
            input_jsonl = %paths.input_jsonl.display(),
            "normalized ogg training audio inputs to wav before encode stage"
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
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);

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

        if !output_path.exists() {
            bail!("训练音频转码后未生成 WAV 文件: {}", output_path.display());
        }

        let output_string = output_path.to_string_lossy().to_string();
        normalized_paths.insert(input_string, output_string.clone());
        Ok(output_string)
    }

    async fn prepare_model_env(
        &self,
        service: &LocalService,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        runtime: TrainingRuntimeOptions,
    ) -> Result<()> {
        info!(
            src_model_root = %paths.src_model_root.display(),
            init_script = %paths.init_task_runtime_script_path.display(),
            download_script = %paths.download_models_script_path.display(),
            training_mode = runtime.mode_label(paths.base_model),
            "preparing local model environment via required init-task-runtime and optional download-models stages"
        );

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
        .await?;

        if self.training_base_model_downloaded(paths.base_model)? {
            info!(
                base_model = %paths.base_model,
                training_mode = runtime.mode_label(paths.base_model),
                "base model is already marked as downloaded; skipping download-models stage"
            );
            self.validate_prepared_model_downloads(paths)?;
            return Ok(());
        }

        let download_script_args =
            llm_model_paths(paths.base_model).download_script_args(&paths.src_model_root);

        self.run_training_script(
            &paths.download_models_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            download_script_args,
            TrainingCommandLabel::DownloadModels,
        )
        .await?;

        self.validate_prepared_model_downloads(paths)?;
        self.mark_training_base_model_downloaded(service, paths.base_model)?;

        Ok(())
    }

    fn training_base_model_downloaded(&self, base_model: BaseModel) -> Result<bool> {
        let config = load_configs()
            .context("failed to load config.toml before checking base model marker")?;

        Ok(config
            .prepared_base_models()
            .iter()
            .any(|prepared| *prepared == base_model))
    }

    fn mark_training_base_model_downloaded(
        &self,
        _service: &LocalService,
        base_model: BaseModel,
    ) -> Result<()> {
        let mut config = load_configs()
            .context("failed to load config.toml before updating prepared base model marker")?;
        if config
            .training
            .prepared_base_models
            .iter()
            .any(|prepared| *prepared == base_model)
        {
            return Ok(());
        }

        config.training.prepared_base_models.push(base_model);
        config
            .training
            .prepared_base_models
            .sort_by_key(|value| value.as_str());
        config.training.prepared_base_models.dedup();

        save_configs(&config)
            .context("failed to persist training.prepared_base_models to config.toml")
    }

    fn validate_prepared_model_downloads(&self, paths: &TrainingPaths) -> Result<()> {
        let mut missing_paths = Vec::new();

        for path in
            llm_model_paths(paths.base_model).prepared_model_download_paths(&paths.src_model_root)
        {
            if !path.exists() {
                missing_paths.push(path.display().to_string());
            }
        }

        if missing_paths.is_empty() {
            return Ok(());
        }

        bail!(
            "基础模型已在 config.toml 的 prepared_base_models 中标记为完成，但以下路径缺失: {}。如需重新下载，请手动清理 training.prepared_base_models 后重试。",
            missing_paths.join(", ")
        )
    }
    async fn run_encode_audio(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        runtime: TrainingRuntimeOptions,
    ) -> Result<()> {
        let attn_implementation = load_configs()
            .context("failed to load config.toml before resolving encode attention implementation")?
            .attn_implementation();

        info!(
            input = %paths.input_jsonl.display(),
            output = %paths.train_jsonl.display(),
            script = %paths.encode_python_script_path.display(),
            training_mode = runtime.mode_label(paths.base_model),
            device = runtime.encode_device(),
            "starting encode_audio.py through direct python invocation"
        );

        if !paths.venv_python_path.exists() {
            bail!(
                "Training venv python not found: {}",
                paths.venv_python_path.display()
            );
        }
        if !paths.tokenizer_model_path.exists() {
            bail!(
                "Training tokenizer model path not found: {}",
                paths.tokenizer_model_path.display()
            );
        }

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.encode_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::EncodeAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--input-jsonl".to_string(),
                paths.input_jsonl.to_string_lossy().to_string(),
                "--output-jsonl".to_string(),
                paths.train_jsonl.to_string_lossy().to_string(),
                "--tokenizer-model-path".to_string(),
                paths.tokenizer_model_path.to_string_lossy().to_string(),
                "--device".to_string(),
                runtime.encode_device().to_string(),
                "--attn-implementation".to_string(),
                attn_implementation.as_str().to_string(),
            ],
        )
        .await?;

        if !paths.train_jsonl.exists() {
            bail!(
                "Encoded training jsonl not found after stage execution: {}",
                paths.train_jsonl.display()
            );
        }

        Ok(())
    }

    async fn run_training_command(
        &self,
        paths: &TrainingPaths,
        task_id: i64,
        log_dir: &Path,
        speaker_name: &str,
        params: &TrainingParams,
        runtime: TrainingRuntimeOptions,
    ) -> Result<()> {
        let training_config = load_configs().context(
            "failed to load config.toml before resolving training attention implementation",
        )?;
        let attn_implementation = training_config.attn_implementation();

        info!(
            train_jsonl = %paths.train_jsonl.display(),
            output_model_dir = %paths.output_model_dir.display(),
            batch_size = params.batch_size,
            epoch_count = params.epoch_count,
            script = %paths.train_python_script_path.display(),
            training_mode = runtime.mode_label(paths.base_model),
            device = runtime.training_device(),
            "starting training.py through direct python invocation"
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

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::ModelTraining, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;

        let mut script_args = vec![
            "--train-jsonl".to_string(),
            paths.train_jsonl.to_string_lossy().to_string(),
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
            "--speaker-name".to_string(),
            speaker_name.to_string(),
            "--device".to_string(),
            runtime.training_device().to_string(),
            "--attn-implementation".to_string(),
            attn_implementation.as_str().to_string(),
            "--qlora-r".to_string(),
            training_config.qlora_rank().to_string(),
            "--qlora-alpha".to_string(),
            training_config.qlora_alpha().to_string(),
            "--qlora-dropout".to_string(),
            training_config.qlora_dropout().to_string(),
            "--qlora-quant-type".to_string(),
            training_config.qlora_quant_type().as_str().to_string(),
        ];

        match training_config.qlora_mode() {
            QloraMode::Enabled => script_args.push("--use-qlora".to_string()),
            QloraMode::Disabled => script_args.push("--no-use-qlora".to_string()),
        }

        script_args.push(if training_config.qlora_double_quant() {
            "--qlora-double-quant".to_string()
        } else {
            "--no-qlora-double-quant".to_string()
        });

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.train_python_script_path,
            &paths.src_model_root,
            TrainingCommandLabel::RunTraining.as_str(),
            &task_log_path,
            "python command completed successfully",
            script_args,
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

        Self::run_command(
            Path::new(platform.shell_program()),
            &args,
            current_dir,
            label,
            &task_log_path,
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

    async fn run_command(
        program: &Path,
        args: &[String],
        current_dir: &Path,
        label: TrainingCommandLabel,
        task_log_path: &Path,
    ) -> Result<()> {
        run_logged_command(
            program,
            args,
            current_dir,
            label.as_str(),
            task_log_path,
            "command completed successfully",
        )
        .await
    }
}
