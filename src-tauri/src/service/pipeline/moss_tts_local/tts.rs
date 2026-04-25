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
        local_paths::{resolve_local_log_dir, resolve_runtime_model_path, resolve_task_path},
        task_paths::{ensure_task_metrics_log_dir, task_log_file_path},
    },
    config::{load_configs, BaseModel, HardwareType},
    service::{
        local::{
            entity::{task_history as task_history_entity, tts_task as tts_task_entity},
            LocalService,
        },
        models::{
            HistoryTaskType, MossTtsLocalTextToSpeechModelParams, TaskStatus,
            TextToSpeechFormat, UpdateTaskStatusPayload,
        },
        pipeline::{
            model_paths::llm_model_display_name,
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_shared_python_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, PipelineBootstrapPaths,
            TtsPipelineRequest, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
    },
    utils::{
        audio::{build_ffmpeg_transcode_args, resolve_temp_wav_path},
        file_ops::{ensure_parent_dir, remove_file_if_exists, replace_output_file},
        process::{run_logged_command, run_logged_python_script},
    },
    Result,
};

use super::{
    moss_tts_local_download_script_args, moss_tts_local_prepared_model_download_paths,
    MossTtsLocalModelTaskPipeline, MOSS_TTS_LOCAL_MODEL_NAME,
    MOSS_TTS_LOCAL_RECOMMENDED_AUDIO_SAMPLE_RATE,
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

    fn mode_label(self, base_model: &str) -> Result<String> {
        Ok(format!(
            "{} / {}",
            llm_model_display_name(base_model)?,
            if self.is_cpu() { "CPU" } else { "CUDA" }
        ))
    }
}

#[derive(Debug)]
struct TtsPaths {
    base_model: BaseModel,
    model_scale: String,
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
    model_scale: String,
    model_root_path: String,
    language: String,
    format: TextToSpeechFormat,
    text: String,
    n_vq_for_inference: i64,
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
            Self::InitTaskRuntime => INIT_MODEL_RUNTIME_LABEL,
            Self::DownloadModels => DOWNLOAD_MODEL_ARTIFACTS_LABEL,
            Self::RunInference => "run moss tts inference",
            Self::ConvertAudio => "convert moss tts audio",
        }
    }
}

impl MossTtsLocalModelTaskPipeline {
    pub(super) async fn run_tts_pipeline_impl(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        let started_at = Instant::now();

        let result = async {
            self.mark_tts_running(service, request.task_id).await?;
            let params = self.load_tts_task_execution(service, request.task_id).await?;
            let paths = self.resolve_tts_paths(service, &params.base_model, &params.model_scale)?;
            let log_dir = resolve_local_log_dir()?;
            let runtime = TtsRuntimeOptions::from_hardware_type(load_configs()?.hardware_type());

            self.prepare_tts_env(service, &paths, request.task_id, &log_dir, runtime)
                .await?;
            let model_path = resolve_inference_model_path(Path::new(&params.model_root_path))?;
            self.validate_tts_environment(&paths, &model_path, &params.output_file_path)?;

            let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
            self.run_tts_command(
                &paths,
                request.task_id,
                &log_dir,
                &params,
                &model_path,
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
                error!(error = %update_err, task_id = request.task_id, "failed to persist moss tts failure state");
            }
            return Err(err);
        }

        Ok(())
    }

    async fn load_tts_task_execution(
        &self,
        service: &LocalService,
        task_id: i64,
    ) -> Result<TtsTaskExecution> {
        let task_detail = tts_task_entity::Entity::find()
            .filter(tts_task_entity::Column::HistoryId.eq(task_id))
            .filter(tts_task_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| format!("failed to load moss tts params for task {}", task_id))?
            .ok_or_else(|| anyhow::anyhow!("未找到 MOSS-TTS Local TTS 任务执行参数: {}", task_id))?;

        task_history_entity::Entity::find_by_id(task_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| format!("failed to load moss tts task history for task {}", task_id))?
            .ok_or_else(|| anyhow::anyhow!("未找到 MOSS-TTS Local TTS 历史任务记录: {}", task_id))?;

        let params = serde_json::from_str::<MossTtsLocalTextToSpeechModelParams>(&task_detail.model_params_json)?;
        let src_model_root = resolve_src_model_root(service.app_dir())?;

        Ok(TtsTaskExecution {
            base_model: task_detail.base_model,
            model_scale: task_detail.model_scale.trim().to_string(),
            model_root_path: resolve_runtime_model_path(
                Path::new(service.model_dir()),
                &src_model_root,
                &task_detail.model_path.unwrap_or_default(),
            )?
            .to_string_lossy()
            .to_string(),
            language: task_detail.language,
            format: task_detail
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            text: task_detail.text,
            n_vq_for_inference: params.n_vq_for_inference,
            output_file_path: resolve_task_path(
                Path::new(service.data_dir()),
                &task_detail.output_file_path.unwrap_or_default(),
            )
            .to_string_lossy()
            .to_string(),
        })
    }

    fn resolve_tts_paths(
        &self,
        service: &LocalService,
        base_model: &str,
        model_scale: &str,
    ) -> Result<TtsPaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let tts_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "tts.py")?;
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, base_model, "ffmpeg.py");

        Ok(TtsPaths {
            base_model: base_model.to_string(),
            model_scale: model_scale.to_string(),
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
            TtsCommandLabel::InitTaskRuntime,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_tts_stage_script(
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
            TtsCommandLabel::DownloadModels,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_tts_stage_script(
                    &script_path,
                    &working_dir,
                    task_id,
                    &log_dir,
                    script_args,
                    label,
                )
                .await
            },
            || self.validate_prepared_tts_downloads(paths),
        )
        .await?;

        Ok(())
    }

    fn validate_prepared_tts_downloads(&self, paths: &TtsPaths) -> Result<()> {
        crate::service::pipeline::validate_downloaded_paths(
            &paths.base_model,
            &paths.model_scale,
            &moss_tts_local_prepared_model_download_paths(&paths.src_model_root, &paths.model_scale)?,
        )
    }

    fn validate_tts_environment(
        &self,
        paths: &TtsPaths,
        model_path: &Path,
        output_file_path: &str,
    ) -> Result<()> {
        for (label, path) in [
            ("MOSS-TTS Local tts venv python", &paths.venv_python_path),
            (
                "MOSS-TTS Local init-task-runtime script",
                &paths.init_task_runtime_script_path,
            ),
            (
                "MOSS-TTS Local download-models script",
                &paths.download_models_script_path,
            ),
            ("MOSS-TTS Local tts python script", &paths.tts_python_script_path),
            ("MOSS-TTS Local ffmpeg python script", &paths.ffmpeg_python_script_path),
        ] {
            if !path.exists() {
                bail!("{} not found: {}", label, path.display());
            }
        }

        if !model_path.exists() {
            bail!("MOSS-TTS Local model path not found: {}", model_path.display());
        }

        ensure_parent_dir(Path::new(output_file_path), "moss tts output")?;
        Ok(())
    }

    async fn run_tts_command(
        &self,
        paths: &TtsPaths,
        task_id: i64,
        log_dir: &Path,
        params: &TtsTaskExecution,
        model_path: &Path,
        temp_wav_path: &Path,
        runtime: TtsRuntimeOptions,
    ) -> Result<()> {
        let attn_implementation = load_configs()?.attn_implementation();
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;

        info!(
            tts_script = %paths.tts_python_script_path.display(),
            model_path = %model_path.display(),
            output_path = %temp_wav_path.display(),
            device = runtime.device(),
            mode = %runtime.mode_label(&params.base_model)?,
            "starting local moss tts inference through direct python invocation"
        );

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.tts_python_script_path,
            &paths.src_model_root,
            TtsCommandLabel::RunInference.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--init-model-path".to_string(),
                model_path.to_string_lossy().to_string(),
                "--text".to_string(),
                params.text.clone(),
                "--language".to_string(),
                params.language.clone(),
                "--n-vq-for-inference".to_string(),
                params.n_vq_for_inference.to_string(),
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

        if !temp_wav_path.exists() {
            bail!(
                "MOSS-TTS Local output file not found after inference: {}",
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
            replace_output_file(temp_wav_path, final_output_path, "moss tts output")?;
            return Ok(());
        }

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::TextToSpeech, task_id);
        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            TtsCommandLabel::ConvertAudio.as_str(),
            &task_log_path,
            "python command completed successfully",
            build_ffmpeg_transcode_args(
                temp_wav_path,
                final_output_path,
                format.as_str(),
                Some(MOSS_TTS_LOCAL_RECOMMENDED_AUDIO_SAMPLE_RATE),
            ),
        )
        .await?;

        remove_file_if_exists(temp_wav_path, "temporary moss tts wav file")?;
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
            "moss tts command completed successfully",
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
            "MOSS-TTS Local model root path not found: {}",
            model_root_path.display()
        );
    }

    if model_root_path.join("config.json").exists() {
        return Ok(model_root_path.to_path_buf());
    }

    let bundled_model_path = model_root_path.join(MOSS_TTS_LOCAL_MODEL_NAME);
    if bundled_model_path.join("config.json").exists() {
        return Ok(bundled_model_path);
    }

    bail!(
        "未找到可用于 MOSS-TTS Local 推理的模型目录: {}",
        model_root_path.display()
    )
}