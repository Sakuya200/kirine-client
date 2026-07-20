#![allow(dead_code)]

use std::{fs, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonScriptTaskKind {
    Training,
    TextToSpeech,
    VoiceClone,
    VoiceDesign,
    StreamingSpeech,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PythonScriptRuntimeOptions {
    pub device: Option<String>,
    pub logging_dir: Option<String>,
    pub attn_implementation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingArgs {
    pub model_root_path: String,
    #[serde(default)]
    pub speaker_dir_name: Option<String>,
    #[serde(default)]
    pub model_params_json: serde_json::Value,
    pub input_jsonl: String,
    pub output_jsonl: String,
    pub output_model_path: String,
    pub batch_size: i64,
    pub lr: Option<String>,
    pub num_epochs: i64,
    pub speaker_name: String,
    pub gradient_accumulation_steps: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSArgs {
    pub model_root_path: String,
    #[serde(default)]
    pub speaker_dir_name: Option<String>,
    #[serde(default)]
    pub model_params_json: serde_json::Value,
    pub text: String,
    pub language: String,
    pub speaker: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCloneArgs {
    pub model_root_path: String,
    #[serde(default)]
    pub speaker_dir_name: Option<String>,
    #[serde(default)]
    pub model_params_json: serde_json::Value,
    pub ref_audio_path: String,
    #[serde(default)]
    pub ref_text: Option<String>,
    pub language: String,
    pub output_path: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceDesignArgs {
    pub model_root_path: String,
    #[serde(default)]
    pub speaker_dir_name: Option<String>,
    #[serde(default)]
    pub model_params_json: serde_json::Value,
    pub text: String,
    pub language: String,
    pub instruct: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSpeakerArg {
    pub name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingArgs {
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub speakers: Vec<StreamingSpeakerArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PythonScriptTaskArgs {
    Training(TrainingArgs),
    TextToSpeech(TTSArgs),
    VoiceClone(VoiceCloneArgs),
    VoiceDesign(VoiceDesignArgs),
    Streaming(StreamingArgs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonScriptInvocationSpec {
    pub version: String,
    pub base_model: String,
    pub model_version: String,
    pub kind: PythonScriptTaskKind,
    pub runtime: PythonScriptRuntimeOptions,
    pub args: PythonScriptTaskArgs,
}

impl PythonScriptInvocationSpec {
    pub fn write_to_json_file(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create python params directory: {}",
                    parent.display()
                )
            })?;
        }

        let temp_path = path.with_extension("json.tmp");
        let payload = serde_json::to_vec_pretty(self)
            .context("failed to serialize python invocation spec")?;

        fs::write(&temp_path, payload).with_context(|| {
            format!(
                "failed to write temporary python params file: {}",
                temp_path.display()
            )
        })?;

        fs::rename(&temp_path, path).with_context(|| {
            format!(
                "failed to move python params file into place: {} -> {}",
                temp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }
}
