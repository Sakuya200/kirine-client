use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::service::models::HistoryTaskType;

const TASK_LOG_DIR_NAME: &str = "task";
const TASK_METRICS_DIR_NAME: &str = "metrics";
const TRAINING_IMPORTS_DIR_NAME: &str = "imports";
const TRAINING_AUDIOS_DIR_NAME: &str = "audios";
const TRAINING_TEMP_DIR_NAME: &str = "_tmp";
const TRAINING_INDEX_JSONL_NAME: &str = "index.jsonl";
const TRAINING_OUTPUT_JSONL_NAME: &str = "train.jsonl";
const TRAINING_REFERENCE_AUDIO_BASENAME: &str = "ref_radio";

pub(crate) fn task_sample_dir(data_dir: &Path, task_type: HistoryTaskType, task_id: i64) -> PathBuf {
    data_dir
        .join("samples")
        .join(format!("{}_{}", task_type.storage_dir(), task_id))
}

pub(crate) fn ensure_task_sample_dir(
    data_dir: &Path,
    task_type: HistoryTaskType,
    task_id: i64,
) -> Result<PathBuf> {
    let dir = task_sample_dir(data_dir, task_type, task_id);
    if !dir.exists() {
        fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create task sample dir: {}", dir.display()))?;
    }
    Ok(dir)
}

pub(crate) fn training_imports_dir(sample_root: &Path) -> PathBuf {
    sample_root.join(TRAINING_IMPORTS_DIR_NAME)
}

pub(crate) fn training_audios_dir(sample_root: &Path) -> PathBuf {
    sample_root.join(TRAINING_AUDIOS_DIR_NAME)
}

pub(crate) fn training_temp_extract_dir(sample_root: &Path, sample_id: i64) -> PathBuf {
    sample_root.join(TRAINING_TEMP_DIR_NAME).join(sample_id.to_string())
}

pub(crate) fn training_index_jsonl_path(sample_root: &Path) -> PathBuf {
    sample_root.join(TRAINING_INDEX_JSONL_NAME)
}

pub(crate) fn training_output_jsonl_path(sample_root: &Path) -> PathBuf {
    sample_root.join(TRAINING_OUTPUT_JSONL_NAME)
}

pub(crate) fn training_reference_audio_path(sample_root: &Path, extension: &str) -> PathBuf {
    sample_root.join(format!("{}{}", TRAINING_REFERENCE_AUDIO_BASENAME, extension))
}

pub(crate) fn task_log_file_path(
    log_dir: &Path,
    task_type: HistoryTaskType,
    task_id: i64,
) -> PathBuf {
    log_dir.join(TASK_LOG_DIR_NAME).join(format!(
        "{}-{}.log",
        task_log_file_prefix(task_type),
        task_id
    ))
}

pub(crate) fn ensure_task_metrics_log_dir(log_dir: &Path) -> Result<PathBuf> {
    let metrics_dir = log_dir.join(TASK_METRICS_DIR_NAME);
    fs::create_dir_all(&metrics_dir).with_context(|| {
        format!(
            "failed to create task metrics log directory: {}",
            metrics_dir.display()
        )
    })?;
    Ok(metrics_dir)
}

const fn task_log_file_prefix(task_type: HistoryTaskType) -> &'static str {
    match task_type {
        HistoryTaskType::ModelTraining => "training",
        HistoryTaskType::TextToSpeech => "tts",
        HistoryTaskType::VoiceClone => "voice-clone",
    }
}