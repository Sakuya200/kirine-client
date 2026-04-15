use std::{
    io,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_task_path},
        task_paths::{ensure_task_metrics_log_dir, task_log_file_path},
    },
    config::{load_configs, save_configs, BaseModel, HardwareType},
    service::{
        local::{
            entity::{
                task_history as task_history_entity, voice_clone_task as voice_clone_task_entity,
            },
            LocalService,
        },
        models::{HistoryTaskType, TaskStatus, TextToSpeechFormat, UpdateTaskStatusPayload},
        pipeline::{
            model_paths::{llm_model_display_name, llm_model_paths},
            qwen3_tts::Qwen3TTSModelTaskPipeline,
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_shared_python_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            VoiceClonePipelineRequest,
        },
    },
    utils::{
        audio::{is_ogg_audio_path, resolve_normalized_wav_sidecar_path, resolve_temp_wav_path},
        file_ops::{ensure_parent_dir, remove_file_if_exists, replace_output_file},
        process::{run_logged_command, run_logged_python_script},
    },
    Result,
};

#[derive(Debug, Clone, Copy)]
struct VoiceCloneRuntimeOptions {
    hardware_type: HardwareType,
}

impl VoiceCloneRuntimeOptions {
    const fn from_hardware_type(hardware_type: HardwareType) -> Self {
        Self { hardware_type }
    }

    const fn is_cpu(self) -> bool {
        matches!(self.hardware_type, HardwareType::Cpu)
    }

    const fn device(self) -> &'static str {
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
struct VoiceClonePaths {
    base_model: BaseModel,
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    download_models_script_path: PathBuf,
    voice_clone_python_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
    base_model_path: PathBuf,
}

#[derive(Debug)]
struct VoiceCloneTaskExecution {
    base_model: BaseModel,
    language: String,
    format: TextToSpeechFormat,
    ref_audio_name: String,
    ref_audio_path: String,
    ref_text: String,
    text: String,
    output_file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoiceCloneCommandLabel {
    InitTaskRuntime,
    DownloadModels,
    RunInference,
    NormalizeReferenceAudio,
    ConvertAudio,
}

impl VoiceCloneCommandLabel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitTaskRuntime => "initialize voice clone runtime",
            Self::DownloadModels => "download voice clone base models",
            Self::RunInference => "run voice clone inference",
            Self::NormalizeReferenceAudio => "normalize voice clone reference audio",
            Self::ConvertAudio => "convert voice clone audio",
        }
    }
}

impl Qwen3TTSModelTaskPipeline {
    pub(super) async fn run_voice_clone_pipeline_impl(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        let started_at = Instant::now();

        let result = async {
            self.mark_voice_clone_running(service, request.task_id)
                .await?;
            let params = self
                .load_voice_clone_task_execution(service, request.task_id)
                .await?;
            let runtime =
                VoiceCloneRuntimeOptions::from_hardware_type(load_configs()?.hardware_type());
            let paths = self.resolve_voice_clone_paths(service, params.base_model)?;
            let log_dir = resolve_local_log_dir()?;

            self.prepare_voice_clone_env(service, &paths, request.task_id, &log_dir, runtime)
                .await?;
            self.validate_voice_clone_environment(&paths, &params)?;
            self.run_voice_clone_command(&paths, request.task_id, &log_dir, &params, runtime)
                .await?;

            self.mark_voice_clone_completed(
                service,
                request.task_id,
                started_at.elapsed().as_secs() as i64,
            )
            .await
        }
        .await;

        if let Err(err) = result {
            let duration_seconds = started_at.elapsed().as_secs() as i64;
            let error_message = err.to_string();
            if let Err(update_err) = self
                .mark_voice_clone_failed(service, request.task_id, duration_seconds, &error_message)
                .await
            {
                error!(error = %update_err, task_id = request.task_id, "failed to persist voice clone failure state");
            }
            return Err(err);
        }

        Ok(())
    }

    async fn load_voice_clone_task_execution(
        &self,
        service: &LocalService,
        task_id: i64,
    ) -> Result<VoiceCloneTaskExecution> {
        let row = voice_clone_task_entity::Entity::find()
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
            .ok_or_else(|| anyhow::anyhow!("未找到声音克隆任务执行参数: {}", task_id))?;

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
            .ok_or_else(|| anyhow::anyhow!("未找到声音克隆历史任务记录: {}", task_id))?;

        Ok(VoiceCloneTaskExecution {
            base_model: row
                .base_model
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            language: row.language,
            format: row
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            ref_audio_name: row.ref_audio_name,
            ref_audio_path: resolve_task_path(Path::new(service.data_dir()), &row.ref_audio_path)
                .to_string_lossy()
                .to_string(),
            ref_text: row.ref_text,
            text: row.text,
            output_file_path: resolve_task_path(
                Path::new(service.data_dir()),
                &row.output_file_path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("当前声音克隆任务缺少输出文件路径"))?,
            )
            .to_string_lossy()
            .to_string(),
        })
    }

    fn resolve_voice_clone_paths(
        &self,
        service: &LocalService,
        base_model: BaseModel,
    ) -> Result<VoiceClonePaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let voice_clone_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "voice_clone.py");
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, "ffmpeg.py");
        let base_model_path =
            llm_model_paths(base_model).voice_clone_init_model_path(&src_model_root);

        Ok(VoiceClonePaths {
            base_model,
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            download_models_script_path,
            voice_clone_python_script_path,
            ffmpeg_python_script_path,
            base_model_path,
        })
    }

    async fn prepare_voice_clone_env(
        &self,
        service: &LocalService,
        paths: &VoiceClonePaths,
        task_id: i64,
        log_dir: &Path,
        runtime: VoiceCloneRuntimeOptions,
    ) -> Result<()> {
        let mut init_script_args = Vec::new();
        if runtime.is_cpu() {
            init_script_args.push("--cpu-mode".to_string());
        }

        self.run_voice_clone_stage_script(
            &paths.init_task_runtime_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            init_script_args,
            VoiceCloneCommandLabel::InitTaskRuntime,
        )
        .await?;

        if self.voice_clone_base_model_downloaded(paths.base_model)? {
            info!(
                base_model = %paths.base_model,
                training_mode = runtime.mode_label(paths.base_model),
                "base model is already marked as downloaded; skipping download-models stage"
            );
            self.validate_prepared_voice_clone_downloads(paths)?;
            return Ok(());
        }

        let download_script_args =
            llm_model_paths(paths.base_model).download_script_args(&paths.src_model_root);

        self.run_voice_clone_stage_script(
            &paths.download_models_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            download_script_args,
            VoiceCloneCommandLabel::DownloadModels,
        )
        .await?;

        self.validate_prepared_voice_clone_downloads(paths)?;
        self.mark_voice_clone_base_model_downloaded(service, paths.base_model)?;
        Ok(())
    }

    fn voice_clone_base_model_downloaded(&self, base_model: BaseModel) -> Result<bool> {
        let config = load_configs()
            .context("failed to load config.toml before checking voice clone base model marker")?;

        Ok(config
            .prepared_base_models()
            .iter()
            .any(|prepared| *prepared == base_model))
    }

    fn mark_voice_clone_base_model_downloaded(
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

    fn validate_prepared_voice_clone_downloads(&self, paths: &VoiceClonePaths) -> Result<()> {
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

    fn validate_voice_clone_environment(
        &self,
        paths: &VoiceClonePaths,
        params: &VoiceCloneTaskExecution,
    ) -> Result<()> {
        if !paths.init_task_runtime_script_path.exists() {
            bail!(
                "Voice clone init-task-runtime script not found: {}",
                paths.init_task_runtime_script_path.display()
            );
        }
        if !paths.download_models_script_path.exists() {
            bail!(
                "Voice clone download-models script not found: {}",
                paths.download_models_script_path.display()
            );
        }
        if !paths.venv_python_path.exists() {
            bail!(
                "Voice clone venv python not found: {}",
                paths.venv_python_path.display()
            );
        }
        if !paths.voice_clone_python_script_path.exists() {
            bail!(
                "Voice clone python script not found: {}",
                paths.voice_clone_python_script_path.display()
            );
        }
        if !paths.ffmpeg_python_script_path.exists() {
            bail!(
                "Voice clone ffmpeg python script not found: {}",
                paths.ffmpeg_python_script_path.display()
            );
        }
        if !paths.base_model_path.exists() {
            bail!(
                "Voice clone base model path not found: {}",
                paths.base_model_path.display()
            );
        }
        if !Path::new(&params.ref_audio_path).exists() {
            bail!("Reference audio file not found: {}", params.ref_audio_path);
        }
        ensure_parent_dir(Path::new(&params.output_file_path), "voice clone output")?;

        Ok(())
    }

    async fn run_voice_clone_command(
        &self,
        paths: &VoiceClonePaths,
        task_id: i64,
        log_dir: &Path,
        params: &VoiceCloneTaskExecution,
        runtime: VoiceCloneRuntimeOptions,
    ) -> Result<()> {
        let ref_audio_path = self
            .normalize_voice_clone_reference_audio(
                paths,
                task_id,
                log_dir,
                Path::new(&params.ref_audio_path),
            )
            .await?;
        let attn_implementation = load_configs()
            .context(
                "failed to load config.toml before resolving voice clone attention implementation",
            )?
            .attn_implementation();

        info!(
            script = %paths.voice_clone_python_script_path.display(),
            ref_audio_name = %params.ref_audio_name,
            ref_audio_path = %ref_audio_path.display(),
            output_path = %params.output_file_path,
            mode = runtime.mode_label(params.base_model),
            device = runtime.device(),
            "starting local voice clone inference through direct python invocation"
        );

        let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.voice_clone_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::RunInference.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--init-model-path".to_string(),
                paths.base_model_path.to_string_lossy().to_string(),
                "--ref-audio-path".to_string(),
                ref_audio_path.to_string_lossy().to_string(),
                "--ref-text".to_string(),
                params.ref_text.clone(),
                "--text".to_string(),
                params.text.clone(),
                "--language".to_string(),
                params.language.clone(),
                "--output-path".to_string(),
                temp_wav_path.to_string_lossy().to_string(),
                "--logging-dir".to_string(),
                metrics_log_dir.to_string_lossy().to_string(),
                "--device".to_string(),
                runtime.device().to_string(),
                "--attn-implementation".to_string(),
                attn_implementation.as_str().to_string(),
            ],
        )
        .await?;

        self.finalize_voice_clone_output(
            paths,
            task_id,
            log_dir,
            &temp_wav_path,
            &params.output_file_path,
            params.format,
        )
        .await?;

        if !Path::new(&params.output_file_path).exists() {
            bail!(
                "Voice clone output file not found after inference: {}",
                params.output_file_path
            );
        }

        Ok(())
    }

    async fn normalize_voice_clone_reference_audio(
        &self,
        paths: &VoiceClonePaths,
        task_id: i64,
        log_dir: &Path,
        input_path: &Path,
    ) -> Result<PathBuf> {
        if !is_ogg_audio_path(input_path) {
            return Ok(input_path.to_path_buf());
        }
        if !input_path.exists() {
            bail!("Reference audio file not found: {}", input_path.display());
        }

        let output_path = resolve_normalized_wav_sidecar_path(input_path);
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::NormalizeReferenceAudio.as_str(),
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
            bail!(
                "Normalized reference audio not found after conversion: {}",
                output_path.display()
            );
        }

        Ok(output_path)
    }

    async fn finalize_voice_clone_output(
        &self,
        paths: &VoiceClonePaths,
        task_id: i64,
        log_dir: &Path,
        temp_wav_path: &Path,
        final_output_path: &str,
        format: TextToSpeechFormat,
    ) -> Result<()> {
        let final_output_path = Path::new(final_output_path);
        if format == TextToSpeechFormat::Wav {
            replace_output_file(temp_wav_path, final_output_path, "voice clone output")?;
            return Ok(());
        }

        self.transcode_voice_clone_audio(
            paths,
            task_id,
            log_dir,
            temp_wav_path,
            final_output_path,
            format,
        )
        .await
    }

    async fn transcode_voice_clone_audio(
        &self,
        paths: &VoiceClonePaths,
        task_id: i64,
        log_dir: &Path,
        input_wav_path: &Path,
        final_output_path: &Path,
        format: TextToSpeechFormat,
    ) -> Result<()> {
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::ConvertAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--input-path".to_string(),
                input_wav_path.to_string_lossy().to_string(),
                "--output-path".to_string(),
                final_output_path.to_string_lossy().to_string(),
                "--format".to_string(),
                format.as_str().to_string(),
            ],
        )
        .await?;

        remove_file_if_exists(input_wav_path, "temporary voice clone wav file")?;

        Ok(())
    }

    async fn run_voice_clone_stage_script(
        &self,
        script_path: &Path,
        current_dir: &Path,
        task_id: i64,
        log_dir: &Path,
        script_args: Vec<String>,
        label: VoiceCloneCommandLabel,
    ) -> Result<()> {
        let platform = ScriptPlatform::current();
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
        let mut args = platform.shell_args(script_path);
        args.push("--log-path".to_string());
        args.push(log_dir.to_string_lossy().to_string());
        args.push("--task-log-file".to_string());
        args.push(task_log_path.to_string_lossy().to_string());
        args.extend(script_args);

        Self::run_voice_clone_process(
            Path::new(platform.shell_program()),
            &args,
            current_dir,
            label,
            &task_log_path,
        )
        .await
    }

    async fn run_voice_clone_process(
        program: &Path,
        args: &[String],
        current_dir: &Path,
        label: VoiceCloneCommandLabel,
        task_log_path: &Path,
    ) -> Result<()> {
        run_logged_command(
            program,
            args,
            current_dir,
            label.as_str(),
            task_log_path,
            "voice clone command completed successfully",
        )
        .await
    }

    async fn mark_voice_clone_running(&self, service: &LocalService, task_id: i64) -> Result<()> {
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

    async fn mark_voice_clone_completed(
        &self,
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

    async fn mark_voice_clone_failed(
        &self,
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
}
