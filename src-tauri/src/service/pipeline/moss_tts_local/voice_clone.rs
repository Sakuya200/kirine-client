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
        task_paths::{
            ensure_task_metrics_log_dir, task_log_file_path, task_sample_dir,
            voice_clone_params_json_path,
        },
    },
    config::{load_configs, BaseModel, HardwareType},
    service::{
        local::{
            entity::{
                task_history as task_history_entity, voice_clone_task as voice_clone_task_entity,
            },
            LocalService,
        },
        models::{
            HistoryTaskType, MossTtsLocalVoiceCloneModelParams, TaskStatus, TextToSpeechFormat,
            UpdateTaskStatusPayload,
        },
        pipeline::{
            api::{
                PythonScriptExecutionTarget, PythonScriptInvocationSpec,
                PythonScriptRuntimeOptions, PythonScriptTaskArgs, PythonScriptTaskKind,
                VoiceCloneScriptArgs,
            },
            script_paths::{
                resolve_src_model_root, src_model_model_python_script_path,
                src_model_shared_python_script_path, src_model_venv_python_path, ScriptPlatform,
            },
            validate_and_download, validate_and_init, PipelineBootstrapPaths,
            VoiceClonePipelineRequest, DOWNLOAD_MODEL_ARTIFACTS_LABEL, INIT_MODEL_RUNTIME_LABEL,
        },
    },
    utils::{
        audio::{
            build_ffmpeg_transcode_args, resolve_normalized_wav_sidecar_path, resolve_temp_wav_path,
        },
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
}

#[derive(Debug)]
struct VoiceClonePaths {
    base_model: BaseModel,
    model_scale: String,
    src_model_root: PathBuf,
    venv_python_path: PathBuf,
    init_task_runtime_script_path: PathBuf,
    download_models_script_path: PathBuf,
    voice_clone_python_script_path: PathBuf,
    ffmpeg_python_script_path: PathBuf,
    base_model_path: PathBuf,
    params_json_path: PathBuf,
}

#[derive(Debug)]
struct VoiceCloneTaskExecution {
    base_model: BaseModel,
    model_scale: String,
    language: String,
    format: TextToSpeechFormat,
    ref_audio_path: String,
    ref_text: String,
    text: String,
    n_vq_for_inference: i64,
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
            Self::InitTaskRuntime => INIT_MODEL_RUNTIME_LABEL,
            Self::DownloadModels => DOWNLOAD_MODEL_ARTIFACTS_LABEL,
            Self::RunInference => "run moss voice clone inference",
            Self::NormalizeReferenceAudio => "normalize moss voice clone reference audio",
            Self::ConvertAudio => "convert moss voice clone audio",
        }
    }
}

impl MossTtsLocalModelTaskPipeline {
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
            let paths = self.resolve_voice_clone_paths(
                service,
                request.task_id,
                &params.base_model,
                &params.model_scale,
            )?;
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
                error!(error = %update_err, task_id = request.task_id, "failed to persist moss voice clone failure state");
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
                    "failed to load moss voice clone params for task {}",
                    task_id
                )
            })?
            .ok_or_else(|| {
                anyhow::anyhow!("未找到 MOSS-TTS Local 声音克隆任务执行参数: {}", task_id)
            })?;

        task_history_entity::Entity::find_by_id(task_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(service.orm())
            .await
            .with_context(|| {
                format!(
                    "failed to load moss voice clone task history for task {}",
                    task_id
                )
            })?
            .ok_or_else(|| {
                anyhow::anyhow!("未找到 MOSS-TTS Local 声音克隆历史任务记录: {}", task_id)
            })?;

        let params =
            serde_json::from_str::<MossTtsLocalVoiceCloneModelParams>(&row.model_params_json)?;

        Ok(VoiceCloneTaskExecution {
            base_model: row.base_model,
            model_scale: row.model_scale.trim().to_string(),
            language: row.language,
            format: row
                .format
                .parse()
                .map_err(|err: String| io::Error::new(io::ErrorKind::InvalidData, err))?,
            ref_audio_path: resolve_task_path(Path::new(service.data_dir()), &row.ref_audio_path)
                .to_string_lossy()
                .to_string(),
            ref_text: row.ref_text,
            text: row.text,
            n_vq_for_inference: params.n_vq_for_inference,
            output_file_path: resolve_task_path(
                Path::new(service.data_dir()),
                &row.output_file_path.clone().unwrap_or_default(),
            )
            .to_string_lossy()
            .to_string(),
        })
    }

    fn resolve_voice_clone_paths(
        &self,
        service: &LocalService,
        task_id: i64,
        base_model: &str,
        model_scale: &str,
    ) -> Result<VoiceClonePaths> {
        let platform = ScriptPlatform::current();
        let src_model_root = resolve_src_model_root(service.app_dir())?;
        let venv_python_path = src_model_venv_python_path(&src_model_root, base_model);
        let init_task_runtime_script_path =
            src_model_root.join(platform.init_task_runtime_relative_path());
        let download_models_script_path =
            src_model_root.join(platform.download_models_relative_path());
        let voice_clone_python_script_path =
            src_model_model_python_script_path(&src_model_root, base_model, "voice_clone.py")?;
        let ffmpeg_python_script_path =
            src_model_shared_python_script_path(&src_model_root, base_model, "ffmpeg.py");
        let base_model_path = src_model_root
            .join("base-models")
            .join(MOSS_TTS_LOCAL_MODEL_NAME);
        let sample_root = task_sample_dir(
            Path::new(service.data_dir()),
            HistoryTaskType::VoiceClone,
            task_id,
        );
        let params_json_path = voice_clone_params_json_path(&sample_root);

        Ok(VoiceClonePaths {
            base_model: base_model.to_string(),
            model_scale: model_scale.to_string(),
            src_model_root,
            venv_python_path,
            init_task_runtime_script_path,
            download_models_script_path,
            voice_clone_python_script_path,
            ffmpeg_python_script_path,
            base_model_path,
            params_json_path,
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
            VoiceCloneCommandLabel::InitTaskRuntime,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_voice_clone_stage_script(
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
            VoiceCloneCommandLabel::DownloadModels,
            |script_path, working_dir, task_id, log_dir, script_args, label| async move {
                self.run_voice_clone_stage_script(
                    &script_path,
                    &working_dir,
                    task_id,
                    &log_dir,
                    script_args,
                    label,
                )
                .await
            },
            || self.validate_prepared_voice_clone_downloads(paths),
        )
        .await?;

        Ok(())
    }

    fn validate_prepared_voice_clone_downloads(&self, paths: &VoiceClonePaths) -> Result<()> {
        crate::service::pipeline::validate_downloaded_paths(
            &paths.base_model,
            &paths.model_scale,
            &moss_tts_local_prepared_model_download_paths(
                &paths.src_model_root,
                &paths.model_scale,
            )?,
        )
    }

    fn validate_voice_clone_environment(
        &self,
        paths: &VoiceClonePaths,
        params: &VoiceCloneTaskExecution,
    ) -> Result<()> {
        for (label, path) in [
            (
                "MOSS-TTS Local voice clone venv python",
                &paths.venv_python_path,
            ),
            (
                "MOSS-TTS Local init-task-runtime script",
                &paths.init_task_runtime_script_path,
            ),
            (
                "MOSS-TTS Local download-models script",
                &paths.download_models_script_path,
            ),
            (
                "MOSS-TTS Local voice clone python script",
                &paths.voice_clone_python_script_path,
            ),
            (
                "MOSS-TTS Local ffmpeg python script",
                &paths.ffmpeg_python_script_path,
            ),
            ("MOSS-TTS Local base model path", &paths.base_model_path),
        ] {
            if !path.exists() {
                bail!("{} not found: {}", label, path.display());
            }
        }

        if !Path::new(&params.ref_audio_path).exists() {
            bail!("Reference audio file not found: {}", params.ref_audio_path);
        }

        ensure_parent_dir(
            Path::new(&params.output_file_path),
            "moss voice clone output",
        )?;
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
        let attn_implementation = load_configs()?.attn_implementation();

        info!(
            script = %paths.voice_clone_python_script_path.display(),
            params_file = %paths.params_json_path.display(),
            "starting local moss voice clone inference through params-file python invocation"
        );

        let temp_wav_path = resolve_temp_wav_path(&params.output_file_path, params.format);
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
        let metrics_log_dir = ensure_task_metrics_log_dir(log_dir)?;

        let invocation = PythonScriptInvocationSpec {
            version: 1,
            base_model: paths.base_model.clone(),
            kind: PythonScriptTaskKind::VoiceClone,
            runtime: PythonScriptRuntimeOptions {
                device: Some(runtime.device().to_string()),
                logging_dir: Some(metrics_log_dir.to_string_lossy().to_string()),
                attn_implementation: Some(attn_implementation.as_str().to_string()),
            },
            target: PythonScriptExecutionTarget {
                model_script_name: "voice_clone.py".to_string(),
                uses_shared_helpers: vec!["common.py".to_string()],
            },
            args: PythonScriptTaskArgs::VoiceClone(VoiceCloneScriptArgs {
                ref_audio_path: ref_audio_path.to_string_lossy().to_string(),
                ref_text: params.ref_text.clone(),
                init_model_path: paths.base_model_path.to_string_lossy().to_string(),
                language: params.language.clone(),
                output_path: temp_wav_path.to_string_lossy().to_string(),
                text: params.text.clone(),
                mode: None,
                style_prompt: None,
                cfg_value: None,
                inference_timesteps: None,
                n_vq_for_inference: Some(params.n_vq_for_inference),
            }),
        };

        invocation.write_to_json_file(&paths.params_json_path)?;

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.voice_clone_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::RunInference.as_str(),
            &task_log_path,
            "python command completed successfully",
            vec![
                "--params-file".to_string(),
                paths.params_json_path.to_string_lossy().to_string(),
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
                "MOSS-TTS Local voice clone output file not found after inference: {}",
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
        let output_path = resolve_normalized_wav_sidecar_path(input_path);
        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);

        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::NormalizeReferenceAudio.as_str(),
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

        if !output_path.exists() {
            bail!(
                "Normalized MOSS reference audio not found after conversion: {}",
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
            replace_output_file(temp_wav_path, final_output_path, "moss voice clone output")?;
            return Ok(());
        }

        let task_log_path = task_log_file_path(log_dir, HistoryTaskType::VoiceClone, task_id);
        run_logged_python_script(
            &paths.venv_python_path,
            &paths.ffmpeg_python_script_path,
            &paths.src_model_root,
            VoiceCloneCommandLabel::ConvertAudio.as_str(),
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

        remove_file_if_exists(temp_wav_path, "temporary moss voice clone wav file")?;
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

        run_logged_command(
            Path::new(platform.shell_program()),
            &args,
            current_dir,
            label.as_str(),
            &task_log_path,
            "moss voice clone command completed successfully",
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
