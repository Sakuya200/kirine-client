mod preset_speakers;
mod training;
mod tts;
mod voice_clone;

use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use preset_speakers::qwen3_tts_preset_speakers;
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

const QWEN3_TTS_TOKENIZER_NAME: &str = "Qwen3-TTS-Tokenizer-12Hz";
const QWEN3_TTS_TOKENIZER_REPO_ID: &str = "Qwen/Qwen3-TTS-Tokenizer-12Hz";
pub(crate) const QWEN3_TTS_DISPLAY_NAME: &str = "Qwen3-TTS";
pub(crate) const QWEN3_TTS_BASE_MODEL: &str = "qwen3_tts";
const QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR: &str = "qwen3_tts";
pub(crate) const QWEN3_TTS_MODEL_ARTIFACTS_DIR: &str = "base-models";

#[derive(Debug, Clone, Copy)]
pub(crate) struct Qwen3TtsVariantDefinition {
    pub model_scale: &'static str,
    pub base_model_name: &'static str,
    pub custom_voice_model_name: &'static str,
    pub base_model_repo_id: &'static str,
    pub custom_voice_model_repo_id: &'static str,
}

const QWEN3_TTS_17B_VARIANT: Qwen3TtsVariantDefinition = Qwen3TtsVariantDefinition {
    model_scale: "1.7B",
    base_model_name: "Qwen3-TTS-12Hz-1.7B-Base",
    custom_voice_model_name: "Qwen3-TTS-12Hz-1.7B-CustomVoice",
    base_model_repo_id: "Qwen/Qwen3-TTS-12Hz-1.7B-Base",
    custom_voice_model_repo_id: "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice",
};

const QWEN3_TTS_06B_VARIANT: Qwen3TtsVariantDefinition = Qwen3TtsVariantDefinition {
    model_scale: "0.6B",
    base_model_name: "Qwen3-TTS-12Hz-0.6B-Base",
    custom_voice_model_name: "Qwen3-TTS-12Hz-0.6B-CustomVoice",
    base_model_repo_id: "Qwen/Qwen3-TTS-12Hz-0.6B-Base",
    custom_voice_model_repo_id: "Qwen/Qwen3-TTS-12Hz-0.6B-CustomVoice",
};

const QWEN3_TTS_VARIANTS: &[Qwen3TtsVariantDefinition] =
    &[QWEN3_TTS_17B_VARIANT, QWEN3_TTS_06B_VARIANT];

pub(crate) const QWEN3_TTS_DEFAULT_CUSTOM_VOICE_MODEL_NAME: &str =
    QWEN3_TTS_17B_VARIANT.custom_voice_model_name;

pub(crate) static QWEN3_TTS_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: QWEN3_TTS_DISPLAY_NAME,
        python_script_dir: QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR,
    });

pub(crate) static QWEN3_TTS_MODEL_PATHS: Qwen3TtsModelPaths = Qwen3TtsModelPaths;

pub(crate) static QWEN3_TTS_MODEL_TASK_PIPELINE: Qwen3TTSModelTaskPipeline =
    Qwen3TTSModelTaskPipeline;

pub(crate) struct Qwen3TtsModelPaths;

pub(crate) struct Qwen3TTSModelTaskPipeline;

fn qwen3_tts_artifact_path(src_model_root: &Path, artifact_name: &str) -> PathBuf {
    src_model_root
        .join(QWEN3_TTS_MODEL_ARTIFACTS_DIR)
        .join(artifact_name)
}

pub(crate) fn qwen3_tts_variant_definition(
    model_scale: &str,
) -> Result<&'static Qwen3TtsVariantDefinition> {
    let normalized = model_scale.trim();

    QWEN3_TTS_VARIANTS
        .iter()
        .find(|variant| variant.model_scale == normalized)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("不支持的 Qwen3-TTS 模型规模: {}", model_scale),
            )
            .into()
        })
}

pub(crate) fn qwen3_tts_prepared_variant_key(model_scale: &str) -> Result<String> {
    let variant = qwen3_tts_variant_definition(model_scale)?;
    Ok(format!("{}:{}", QWEN3_TTS_BASE_MODEL, variant.model_scale))
}

pub(crate) fn qwen3_tts_download_script_args(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<Vec<String>> {
    let variant = qwen3_tts_variant_definition(model_scale)?;
    let target_root_dir = src_model_root.join(QWEN3_TTS_MODEL_ARTIFACTS_DIR);

    Ok(vec![
        "--model-id-list".to_string(),
        to_string(&vec![
            variant.base_model_repo_id,
            QWEN3_TTS_TOKENIZER_REPO_ID,
            variant.custom_voice_model_repo_id,
        ])
        .expect("serialize required model repo ids"),
        "--model-name-list".to_string(),
        to_string(&vec![
            variant.base_model_name,
            QWEN3_TTS_TOKENIZER_NAME,
            variant.custom_voice_model_name,
        ])
        .expect("serialize required model names"),
        "--target-root-dir".to_string(),
        target_root_dir.to_string_lossy().to_string(),
    ])
}

pub(crate) fn qwen3_tts_training_tokenizer_model_path(
    src_model_root: &Path,
    _model_scale: &str,
) -> Result<PathBuf> {
    Ok(qwen3_tts_artifact_path(
        src_model_root,
        QWEN3_TTS_TOKENIZER_NAME,
    ))
}

pub(crate) fn qwen3_tts_training_init_model_path(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<PathBuf> {
    let variant = qwen3_tts_variant_definition(model_scale)?;
    Ok(qwen3_tts_artifact_path(
        src_model_root,
        variant.base_model_name,
    ))
}

pub(crate) fn qwen3_tts_voice_clone_init_model_path(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<PathBuf> {
    qwen3_tts_training_init_model_path(src_model_root, model_scale)
}

pub(crate) fn qwen3_tts_preset_custom_voice_model_path(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<PathBuf> {
    let variant = qwen3_tts_variant_definition(model_scale)?;
    Ok(qwen3_tts_artifact_path(
        src_model_root,
        variant.custom_voice_model_name,
    ))
}

pub(crate) fn qwen3_tts_prepared_model_download_paths(
    src_model_root: &Path,
    model_scale: &str,
) -> Result<Vec<PathBuf>> {
    let variant = qwen3_tts_variant_definition(model_scale)?;
    Ok(vec![
        qwen3_tts_artifact_path(src_model_root, variant.base_model_name),
        qwen3_tts_artifact_path(src_model_root, QWEN3_TTS_TOKENIZER_NAME),
        qwen3_tts_artifact_path(src_model_root, variant.custom_voice_model_name),
    ])
}

impl LlmModelPaths for Qwen3TtsModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &QWEN3_TTS_MODEL_DEFINITION
    }
}

#[async_trait]
impl ModelTaskPipeline for Qwen3TTSModelTaskPipeline {
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
