mod tts;
mod voice_clone;

use std::{
    io,
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use tts::resolve_inference_model_path;

use crate::{
    service::pipeline::{
        llm_models::LlmModelDefinition, model_paths::LlmModelPaths,
        pipeline::CommonModelTaskPipeline,
    },
    Result,
};

const QWEN3_TTS_TOKENIZER_NAME: &str = "Qwen3-TTS-Tokenizer-12Hz";
pub(crate) const QWEN3_TTS_DISPLAY_NAME: &str = "Qwen3-TTS";
pub(crate) const QWEN3_TTS_BASE_MODEL: &str = "qwen3_tts";
const QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR: &str = "qwen3_tts";
pub(crate) const QWEN3_TTS_MODEL_ARTIFACTS_DIR: &str = "base-models";

#[derive(Debug, Clone, Copy)]
pub(crate) struct Qwen3TtsVariantDefinition {
    pub model_scale: &'static str,
    pub base_model_name: &'static str,
    pub custom_voice_model_name: &'static str,
}

const QWEN3_TTS_17B_VARIANT: Qwen3TtsVariantDefinition = Qwen3TtsVariantDefinition {
    model_scale: "1.7B",
    base_model_name: "Qwen3-TTS-12Hz-1.7B-Base",
    custom_voice_model_name: "Qwen3-TTS-12Hz-1.7B-CustomVoice",
};

const QWEN3_TTS_06B_VARIANT: Qwen3TtsVariantDefinition = Qwen3TtsVariantDefinition {
    model_scale: "0.6B",
    base_model_name: "Qwen3-TTS-12Hz-0.6B-Base",
    custom_voice_model_name: "Qwen3-TTS-12Hz-0.6B-CustomVoice",
};

const QWEN3_TTS_VARIANTS: &[Qwen3TtsVariantDefinition] =
    &[QWEN3_TTS_17B_VARIANT, QWEN3_TTS_06B_VARIANT];

pub(crate) static QWEN3_TTS_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: QWEN3_TTS_DISPLAY_NAME,
        python_script_dir: QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR,
    });

pub(crate) static QWEN3_TTS_MODEL_PATHS: Qwen3TtsModelPaths = Qwen3TtsModelPaths;

pub(crate) static QWEN3_TTS_MODEL_TASK_PIPELINE: CommonModelTaskPipeline =
    CommonModelTaskPipeline::new(QWEN3_TTS_BASE_MODEL);

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
