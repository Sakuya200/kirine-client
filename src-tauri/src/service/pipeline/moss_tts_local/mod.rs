mod training;
mod tts;
mod voice_clone;

use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use tts::resolve_inference_model_path;

use async_trait::async_trait;
use serde_json::to_string;

use crate::{
    service::{
        local::LocalService,
        pipeline::{
            llm_models::LlmModelDefinition, model_paths::LlmModelPaths, ModelTaskPipeline,
            TrainingPipelineRequest, TtsPipelineRequest, VoiceClonePipelineRequest,
        },
    },
    Result,
};

pub(crate) const MOSS_TTS_LOCAL_DISPLAY_NAME: &str = "MOSS-TTS Local";
pub(crate) const MOSS_TTS_LOCAL_BASE_MODEL: &str = "moss_tts_local";
pub(crate) const MOSS_TTS_LOCAL_MODEL_SCALE: &str = "1.7B";
pub(crate) const MOSS_TTS_LOCAL_RECOMMENDED_AUDIO_SAMPLE_RATE: u32 = 24_000;
const MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR: &str = "moss_tts_local";
const MOSS_TTS_LOCAL_MODEL_ARTIFACTS_DIR: &str = "base-models";
const MOSS_TTS_LOCAL_MODEL_NAME: &str = "MOSS-TTS-Local-Transformer";
const MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME: &str = "MOSS-Audio-Tokenizer";
const MOSS_TTS_LOCAL_MODEL_REPO_ID: &str = "OpenMOSS-Team/MOSS-TTS-Local-Transformer";
const MOSS_TTS_LOCAL_AUDIO_TOKENIZER_REPO_ID: &str = "OpenMOSS-Team/MOSS-Audio-Tokenizer";

pub(crate) static MOSS_TTS_LOCAL_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: MOSS_TTS_LOCAL_DISPLAY_NAME,
        python_script_dir: MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR,
    });

pub(crate) static MOSS_TTS_LOCAL_MODEL_PATHS: MossTtsLocalModelPaths = MossTtsLocalModelPaths;

pub(crate) static MOSS_TTS_LOCAL_MODEL_TASK_PIPELINE: MossTtsLocalModelTaskPipeline =
    MossTtsLocalModelTaskPipeline;

pub(crate) struct MossTtsLocalModelPaths;

pub(crate) struct MossTtsLocalModelTaskPipeline;

impl LlmModelPaths for MossTtsLocalModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &MOSS_TTS_LOCAL_MODEL_DEFINITION
    }
}

pub(crate) fn moss_tts_local_download_script_args(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<Vec<String>> {
    validate_moss_tts_local_model_scale(model_scale)?;
    let target_root_dir = src_model_root.join(MOSS_TTS_LOCAL_MODEL_ARTIFACTS_DIR);

    Ok(vec![
        "--model-id-list".to_string(),
        to_string(&vec![
            MOSS_TTS_LOCAL_MODEL_REPO_ID,
            MOSS_TTS_LOCAL_AUDIO_TOKENIZER_REPO_ID,
        ])
        .expect("serialize moss model repo ids"),
        "--model-name-list".to_string(),
        to_string(&vec![
            MOSS_TTS_LOCAL_MODEL_NAME,
            MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME,
        ])
        .expect("serialize moss model names"),
        "--target-root-dir".to_string(),
        target_root_dir.to_string_lossy().to_string(),
    ])
}

pub(crate) fn moss_tts_local_prepared_model_download_paths(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<Vec<PathBuf>> {
    validate_moss_tts_local_model_scale(model_scale)?;
    Ok(vec![
        src_model_root
            .join(MOSS_TTS_LOCAL_MODEL_ARTIFACTS_DIR)
            .join(MOSS_TTS_LOCAL_MODEL_NAME),
        src_model_root
            .join(MOSS_TTS_LOCAL_MODEL_ARTIFACTS_DIR)
            .join(MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME),
    ])
}

fn validate_moss_tts_local_model_scale(model_scale: &str) -> Result<()> {
    let normalized_scale = model_scale.trim();
    if normalized_scale != MOSS_TTS_LOCAL_MODEL_SCALE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的 MOSS-TTS Local 模型规模: {}", model_scale),
        )
        .into());
    }

    Ok(())
}

#[async_trait]
impl ModelTaskPipeline for MossTtsLocalModelTaskPipeline {
    async fn run_training_pipeline(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        self.run_training_pipeline_impl(service, request).await
    }

    async fn run_tts_pipeline(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        self.run_tts_pipeline_impl(service, request).await
    }

    async fn run_voice_clone_pipeline(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        self.run_voice_clone_pipeline_impl(service, request).await
    }
}