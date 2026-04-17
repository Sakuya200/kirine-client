use std::io;
use std::path::{Path, PathBuf};

use super::llm_models::LlmModelDefinition;
use super::qwen3_tts::{QWEN3_TTS_BASE_MODEL, QWEN3_TTS_MODEL_PATHS};
use super::vox_cpm2::{VOX_CPM2_BASE_MODEL, VOX_CPM2_MODEL_PATHS};
use crate::Result;

pub(crate) trait LlmModelPaths: Send + Sync {
    fn definition(&self) -> &'static LlmModelDefinition;

    fn display_name(&self) -> &'static str {
        self.definition().display_name
    }

    fn python_script_path(&self, src_model_root: &Path, script_name: &str) -> PathBuf {
        src_model_root
            .join("src")
            .join(self.definition().python_script_dir)
            .join(script_name)
    }
}

pub(crate) fn speaker_model_dir(
    model_dir: &Path,
    speaker_id: i64,
    speaker_name_segment: &str,
) -> PathBuf {
    model_dir.join(format!("{}_{}", speaker_id, speaker_name_segment))
}

pub(crate) fn llm_model_paths(base_model: &str) -> Result<&'static dyn LlmModelPaths> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => Ok(&QWEN3_TTS_MODEL_PATHS),
        VOX_CPM2_BASE_MODEL => Ok(&VOX_CPM2_MODEL_PATHS),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}

pub(crate) fn llm_model_display_name(base_model: &str) -> Result<&'static str> {
    Ok(llm_model_paths(base_model)?.display_name())
}
