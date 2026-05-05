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

pub(crate) const MOSS_TTS_LOCAL_DISPLAY_NAME: &str = "MOSS-TTS Local";
pub(crate) const MOSS_TTS_LOCAL_BASE_MODEL: &str = "moss_tts_local";
pub(crate) const MOSS_TTS_LOCAL_MODEL_SCALE: &str = "1.7B";
const MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR: &str = "moss_tts_local";
const MOSS_TTS_LOCAL_MODEL_ARTIFACTS_DIR: &str = "base-models";
const MOSS_TTS_LOCAL_MODEL_NAME: &str = "MOSS-TTS-Local-Transformer";
const MOSS_TTS_LOCAL_AUDIO_TOKENIZER_NAME: &str = "MOSS-Audio-Tokenizer";

pub(crate) static MOSS_TTS_LOCAL_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: MOSS_TTS_LOCAL_DISPLAY_NAME,
        python_script_dir: MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR,
    });

pub(crate) static MOSS_TTS_LOCAL_MODEL_PATHS: MossTtsLocalModelPaths = MossTtsLocalModelPaths;

pub(crate) static MOSS_TTS_LOCAL_MODEL_TASK_PIPELINE: CommonModelTaskPipeline =
    CommonModelTaskPipeline::new(MOSS_TTS_LOCAL_BASE_MODEL);

pub(crate) struct MossTtsLocalModelPaths;

pub(crate) struct MossTtsLocalModelTaskPipeline;

impl LlmModelPaths for MossTtsLocalModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &MOSS_TTS_LOCAL_MODEL_DEFINITION
    }
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
