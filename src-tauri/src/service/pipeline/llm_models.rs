use std::io;

use crate::Result;

#[derive(Debug, Clone, Copy)]
pub(crate) struct LlmModelDefinition {
    pub display_name: &'static str,
    pub python_script_dir: &'static str,
}

pub(crate) const QWEN3_TTS_DISPLAY_NAME: &str = "Qwen3-TTS";
pub(crate) const QWEN3_TTS_BASE_MODEL: &str = "qwen3_tts";
const QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR: &str = "qwen3_tts";

pub(crate) const MOSS_TTS_LOCAL_DISPLAY_NAME: &str = "MOSS-TTS Local";
pub(crate) const MOSS_TTS_LOCAL_BASE_MODEL: &str = "moss_tts_local";
const MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR: &str = "moss_tts_local";

pub(crate) const VOX_CPM2_DISPLAY_NAME: &str = "VoxCPM2";
pub(crate) const VOX_CPM2_BASE_MODEL: &str = "vox_cpm2";
pub(crate) const VOX_CPM2_MODEL_SCALE: &str = "2B";
const VOX_CPM2_MODEL_PYTHON_SCRIPT_DIR: &str = "vox_cpm2";

pub(crate) const QWEN3_TTS_MODEL_DEFINITION: LlmModelDefinition = LlmModelDefinition {
    display_name: QWEN3_TTS_DISPLAY_NAME,
    python_script_dir: QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR,
};

pub(crate) const MOSS_TTS_LOCAL_MODEL_DEFINITION: LlmModelDefinition = LlmModelDefinition {
    display_name: MOSS_TTS_LOCAL_DISPLAY_NAME,
    python_script_dir: MOSS_TTS_LOCAL_MODEL_PYTHON_SCRIPT_DIR,
};

pub(crate) const VOX_CPM2_MODEL_DEFINITION: LlmModelDefinition = LlmModelDefinition {
    display_name: VOX_CPM2_DISPLAY_NAME,
    python_script_dir: VOX_CPM2_MODEL_PYTHON_SCRIPT_DIR,
};

pub(crate) fn llm_model_definition(base_model: &str) -> Result<&'static LlmModelDefinition> {
    match base_model.trim() {
        QWEN3_TTS_BASE_MODEL => Ok(&QWEN3_TTS_MODEL_DEFINITION),
        MOSS_TTS_LOCAL_BASE_MODEL => Ok(&MOSS_TTS_LOCAL_MODEL_DEFINITION),
        VOX_CPM2_BASE_MODEL => Ok(&VOX_CPM2_MODEL_DEFINITION),
        other => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("不支持的基础模型类型: {}", other),
        )
        .into()),
    }
}
