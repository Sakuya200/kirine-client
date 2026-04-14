use std::{
    fs, io,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{bail, Context};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_runtime_model_path, resolve_task_path},
        task_paths::{ensure_task_metrics_log_dir, task_log_file_path},
    },
    config::{load_configs, save_configs, BaseModel, HardwareType},
    service::{
        local::{
            entity::{task_history as task_history_entity, tts_task as tts_task_entity},
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
            TtsPipelineRequest,
        },
    },
    utils::{
        audio::resolve_temp_wav_path,
        file_ops::{ensure_parent_dir, remove_file_if_exists, replace_output_file},
        process::{run_logged_command, run_logged_python_script},
    },
    Result,
};

#[derive(Debug, Clone, Copy)]
struct TtsRuntimeOptions {
    hardware_type: HardwareType,
}

impl TtsRuntimeOptions {
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
struct TtsPaths {
    base_model: BaseModel,
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    download_models_script_path: PathBuf,
    tts_python_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
}

#[derive(Debug)]
struct TtsTaskExecution {
    base_model: BaseModel,
    speaker_name: String,
    model_path: String,
    hardware_type: HardwareType,
    language: String,
    format: TextToSpeechFormat,
    text: String,
    voice_prompt: String,
    output_file_path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TtsCommandLabel {
    InitTaskRuntime,
    DownloadModels,
    RunInference,
    ConvertAudio,
}

impl TtsCommandLabel {
    const fn as_str(self) -> &'static str {
        match self {
            Self::InitTaskRuntime => "initialize tts runtime",
            Self::DownloadModels => "download tts base models",
            Self::RunInference => "run local tts inference",
            Self::ConvertAudio => "convert generated audio",
        }
    }
}

impl Qwen3TTSModelTaskPipeline {
    pub(super) async fn run_tts_pipeline_impl(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        let started_at = Instant::now();

        let result = async {
            self.mark_tts_running(service, request.task_id).await?;
            let params = self
                .load_tts_task_execution(service, request.task_id, request.speaker_id)
                .await?;
            let paths = self.resolve_tts_paths(service, params.base_model)?;
            let log_dir = resolve_local_log_dir()?;
            let runtime = TtsRuntimeOptions::from_hardware_type(params.hardware_type);
            self.prepare_tts_env(service, &paths, request.task_id, &log_dir, runtime)
                .await?;
            self.validate_tts_environment(&paths, &params)?;

            let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
            self.run_tts_command(
                &paths,
                request.task_id,
                &log_dir,
                &params,
                &temp_wav_path,
                runtime,
            )
            .await?;
            self.finalize_tts_output(
                &paths,
                request.task_id,
                &log_dir,
                &temp_wav_path,
                &params.output_file_path,
                params.format,
            )
            .await?;

            self.mark_tts_completed(
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
                .mark_tts_failed(service, request.task_id, duration_seconds, &error_message)
                .await
            {
                error!(error = %update_err, task_id = request.task_id, speaker_id = request.speaker_id, "failed to persist tts failure state");
            }
            return Err(err);
        }

        Ok(())
    }

    async fn load_tts_task_execution(
        &self,
        service: &LocalService,
        task_id: i64,
        speaker_id: i64,
    ) -> Result<TtsTaskExecution> {
        let task_detail = tts_task_entity::Entity::find()
            .filter(tts_task_entity::Column::HistoryId.eq(task_id))
            .filter(tts_task_entity::Column::SpeakerId.eq(speaker_id))
            .filter(tts_task_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| format!("failed to load tts execution params for task {}", task_id))?
            .ok_or_else(|| anyhow::anyhow!("未找到 TTS 任务执行参数: {}", task_id))?;

        let task_history = task_history_entity::Entity::find_by_id(task_id)
            .filter(task_history_entity::Column::SpeakerId.eq(speaker_id))
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| format!("failed to load tts task history for task {}", task_id))?
            .ok_or_else(|| anyhow::anyhow!("未找到 TTS 历史任务记录: {}", task_id))?;

        let src_model_root = resolve_src_model_root(service.app_dir())?;

        Ok(TtsTaskExecution {
            base_model: task_detail
                .base_model
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            speaker_name: task_history.speaker_name_snapshot.trim().to_string(),
            model_path: resolve_runtime_model_path(
                Path::new(service.model_dir()),
                &src_model_root,
                &task_detail
                    .model_path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("当前 TTS 任务缺少可用的本地模型路径"))?,
            )?
            .to_string_lossy()
            .to_string(),
            hardware_type: task_detail
                .hardware_type
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            language: task_detail.language,
            format: task_detail
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            text: task_detail.text,
            voice_prompt: task_detail.voice_prompt,
            output_file_path: resolve_task_path(
                Path::new(service.data_dir()),
                &task_detail
                    .output_file_path
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("当前 TTS 任务缺少输出文件路径"))?,
            )
            .to_string_lossy()
            .to_string(),
        })
    }

    fn resolve_tts_paths(&self, service: &LocalService, base_model: BaseModel) -> Result<TtsPaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let tts_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "tts.py");
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, "ffmpeg.py");

        Ok(TtsPaths {
            base_model,
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            download_models_script_path,
            tts_python_script_path,
            ffmpeg_python_script_path,
        })
    }

    async fn prepare_tts_env(
        &self,
        service: &LocalService,
        paths: &TtsPaths,
        task_id: i64,
        log_dir: &Path,
        runtime: TtsRuntimeOptions,
    ) -> Result<()> {
        let mut init_script_args = Vec::new();
        if runtime.is_cpu() {
            init_script_args.push("--cpu-mode".to_string());
        }

        self.run_tts_stage_script(
            &paths.init_task_runtime_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            init_script_args,
            TtsCommandLabel::InitTaskRuntime,
        )
        .await?;

        if self.tts_base_model_downloaded(paths.base_model)? {
            info!(
                base_model = %paths.base_model,
                training_mode = runtime.mode_label(paths.base_model),
                "base model is already marked as downloaded; skipping download-models stage"
            );
            self.validate_prepared_tts_downloads(paths)?;
            return Ok(());
        }

        let download_script_args =
            llm_model_paths(paths.base_model).download_script_args(&paths.src_model_root);

        self.run_tts_stage_script(
            &paths.download_models_script_path,
            &paths.src_model_root,
            task_id,
            log_dir,
            download_script_args,
            TtsCommandLabel::DownloadModels,
        )
        .await?;

        self.validate_prepared_tts_downloads(paths)?;
        self.mark_tts_base_model_downloaded(service, paths.base_model)?;

        Ok(())
    }

    fn tts_base_model_downloaded(&self, base_model: BaseModel) -> Result<bool> {
        let config = load_configs()
            .context("failed to load config.toml before checking tts base model marker")?;

        Ok(config
            .prepared_base_models()
            .iter()
            .any(|prepared| *prepared == base_model))
    }

    fn mark_tts_base_model_downloaded(
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

    fn validate_prepared_tts_downloads(&self, paths: &TtsPaths) -> Result<()> {
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

    fn validate_tts_environment(&self, paths: &TtsPaths, params: &TtsTaskExecution) -> Result<()> {
        if !paths.venv_python_path.exists() {
            bail!(
                "TTS venv python not found: {}",
                paths.venv_python_path.display()
            );
        }
        if !paths.init_task_runtime_script_path.exists() {
            bail!(
                "TTS init-task-runtime script not found: {}",
                paths.init_task_runtime_script_path.display()
            );
        }
        if !paths.download_models_script_path.exists() {
            bail!(
                "TTS download-models script not found: {}",
                paths.download_models_script_path.display()
            );
        }
        if !paths.tts_python_script_path.exists() {
            bail!(
                "TTS python script not found: {}",
                paths.tts_python_script_path.display()
            );
        }
        if !paths.ffmpeg_python_script_path.exists() {
            bail!(
                "TTS ffmpeg python script not found: {}",
                paths.ffmpeg_python_script_path.display()
            );
        }

        if !Path::new(&params.model_path).exists() {
            bail!("TTS model path not found: {}", params.model_path);
        }

        ensure_parent_dir(Path::new(&params.output_file_path), "tts output")?;

        Ok(())
    }

    async fn run_tts_command(
        &self,
        paths: &TtsPaths,
        task_id: i64,
        log_dir: &Path,
        params: &TtsTaskExecution,
        temp_wav_path: &Path,
        runtime: TtsRuntimeOptions,
    ) -> Result<()> {
        let attn_implementation = load_configs()
            .context("failed to load config.toml before resolving tts attention implementation")?
            .attn_implementation();

        info!(
            tts_script = %paths.tts_python_script_path.display(),
            model_path = %params.model_path,
            speaker_name = %params.speaker_name,
            output_path = %temp_wav_path.display(),
            device = runtime.device(),
            mode = runtime.mode_label(params.base_model),
            "starting local tts inference through direct python invocation"
        );

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;

        let mut script_args = vec![
            "--init-model-path".to_string(),
            params.model_path.clone(),
            "--speaker".to_string(),
            params.speaker_name.clone(),
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
        ];
        if !params.voice_prompt.trim().is_empty() {
            script_args.push("--instruct".to_string());
            script_args.push(params.voice_prompt.clone());
        }

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.tts_python_script_path,
            &paths.src_model_root,
            TtsCommandLabel::RunInference.as_str(),
            &task_log_path,
            "python command completed successfully",
            script_args,
        )
        .await?;

        if !temp_wav_path.exists() {
            bail!(
                "TTS output file not found after inference: {}",
                temp_wav_path.display()
            );
        }

        Ok(())
    }

    async fn finalize_tts_output(
        &self,
        paths: &TtsPaths,
        task_id: i64,
        log_dir: &Path,
        temp_wav_path: &Path,
        final_output_path: &str,
        format: TextToSpeechFormat,
    ) -> Result<()> {
        let final_output_path = Path::new(final_output_path);
        if format == TextToSpeechFormat::Wav {
            replace_output_file(temp_wav_path, final_output_path, "tts output")?;
            return Ok(());
        }

        self.transcode_tts_audio(
            paths,
            task_id,
            log_dir,
            temp_wav_path,
            final_output_path,
            format,
        )
        .await
    }

    async fn transcode_tts_audio(
        &self,
        paths: &TtsPaths,
        task_id: i64,
        log_dir: &Path,
        input_wav_path: &Path,
        final_output_path: &Path,
        format: TextToSpeechFormat,
    ) -> Result<()> {
        let script_args = vec![
            "--input-path".to_string(),
            input_wav_path.to_string_lossy().to_string(),
            "--output-path".to_string(),
            final_output_path.to_string_lossy().to_string(),
            "--format".to_string(),
            format.as_str().to_string(),
        ];

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            TtsCommandLabel::ConvertAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            script_args,
        )
        .await?;

        remove_file_if_exists(input_wav_path, "temporary tts wav file")?;

        Ok(())
    }

    async fn run_tts_stage_script(
        &self,
        script_path: &Path,
        current_dir: &Path,
        task_id: i64,
        log_dir: &Path,
        script_args: Vec<String>,
        label: TtsCommandLabel,
    ) -> Result<()> {
        let platform = ScriptPlatform::current();
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
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
            "tts command completed successfully",
        )
        .await
    }

    async fn mark_tts_running(&self, service: &LocalService, task_id: i64) -> Result<()> {
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

    async fn mark_tts_completed(
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

    async fn mark_tts_failed(
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

pub(crate) fn resolve_inference_model_path(model_root_path: &Path) -> Result<PathBuf> {
    if !model_root_path.exists() {
        bail!(
            "TTS model root path not found: {}",
            model_root_path.display()
        );
    }

    if is_model_checkpoint_dir(model_root_path) {
        return Ok(model_root_path.to_path_buf());
    }

    let mut checkpoint_dirs = fs::read_dir(model_root_path)
        .with_context(|| {
            format!(
                "failed to inspect tts model root: {}",
                model_root_path.display()
            )
        })?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }

            let file_name = entry.file_name().to_string_lossy().to_string();
            let epoch = file_name
                .strip_prefix("checkpoint-epoch-")?
                .parse::<i64>()
                .ok()?;
            Some((epoch, entry.path()))
        })
        .collect::<Vec<_>>();
    checkpoint_dirs.sort_by_key(|(epoch, _)| *epoch);

    let checkpoint_path = checkpoint_dirs.pop().map(|(_, path)| path).ok_or_else(|| {
        anyhow::anyhow!(
            "当前说话人的训练模型目录中没有可用 checkpoint: {}",
            model_root_path.display()
        )
    })?;

    if !is_model_checkpoint_dir(&checkpoint_path) {
        bail!(
            "TTS checkpoint is missing required files: {}",
            checkpoint_path.display()
        );
    }

    Ok(checkpoint_path)
}

fn is_model_checkpoint_dir(path: &Path) -> bool {
    path.join("config.json").exists() && path.join("model.safetensors").exists()
}
