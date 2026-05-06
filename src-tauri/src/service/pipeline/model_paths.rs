use std::path::{Path, PathBuf};

use super::model_artifacts::MODEL_ARTIFACTS_DIR;
use crate::Result;

pub(crate) fn speaker_model_dir(model_dir: &Path, speaker_id: i64) -> PathBuf {
    model_dir.join(speaker_id.to_string())
}

pub(crate) fn llm_model_python_script_path(
    base_model: &str,
    src_model_root: &Path,
    script_name: &str,
) -> Result<PathBuf> {
    Ok(src_model_root.join(base_model).join(script_name))
}

pub(crate) fn preset_model_root_path(
    src_model_root: &Path,
    _base_model: &str,
    _model_scale: &str,
) -> Result<PathBuf> {
    Ok(src_model_root.join(MODEL_ARTIFACTS_DIR))
}
