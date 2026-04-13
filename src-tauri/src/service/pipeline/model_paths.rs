use std::path::{Path, PathBuf};

use crate::config::BaseModel;

use super::llm_models::LlmModelDefinition;
use super::qwen3_tts::QWEN3_TTS_MODEL_PATHS;

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

    fn download_script_args(&self, src_model_root: &Path) -> Vec<String>;

    fn training_tokenizer_model_path(&self, src_model_root: &Path) -> PathBuf;

    fn training_init_model_path(&self, src_model_root: &Path) -> PathBuf;

    fn prepared_model_download_paths(&self, src_model_root: &Path) -> Vec<PathBuf>;

    fn voice_clone_init_model_path(&self, src_model_root: &Path) -> PathBuf;

    fn artifact_path(&self, src_model_root: &Path, artifact_name: &str) -> PathBuf {
        src_model_root
            .join(self.definition().artifacts_dir)
            .join(artifact_name)
    }
}

pub(crate) fn speaker_model_dir(
    model_dir: &Path,
    speaker_id: i64,
    speaker_name_segment: &str,
) -> PathBuf {
    model_dir.join(format!("{}_{}", speaker_id, speaker_name_segment))
}

pub(crate) fn llm_model_paths(base_model: BaseModel) -> &'static dyn LlmModelPaths {
    match base_model {
        BaseModel::Qwen3Tts => &QWEN3_TTS_MODEL_PATHS,
    }
}

pub(crate) fn llm_model_display_name(base_model: BaseModel) -> &'static str {
    llm_model_paths(base_model).display_name()
}
