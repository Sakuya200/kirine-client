use std::path::{Path, PathBuf};

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
