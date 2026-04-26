#![allow(dead_code)]

use std::{fs, path::Path};

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PythonScriptTaskKind {
    Training,
    TextToSpeech,
    VoiceClone,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PythonScriptRuntimeOptions {
    pub device: Option<String>,
    pub logging_dir: Option<String>,
    pub attn_implementation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PythonScriptExecutionTarget {
    pub model_script_name: String,
    pub uses_shared_helpers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrainingScriptArgs {
    pub init_model_path: String,
    pub codec_path: Option<String>,
    pub tokenizer_model_path: Option<String>,
    pub input_jsonl: String,
    pub output_jsonl: Option<String>,
    pub output_model_path: String,
    pub batch_size: i64,
    pub lr: Option<String>,
    pub num_epochs: i64,
    pub speaker_name: Option<String>,
    pub gradient_accumulation_steps: i64,
    pub enable_gradient_checkpointing: bool,
    pub use_lora: bool,
    pub training_mode: Option<String>,
    pub lora_rank: Option<i64>,
    pub lora_alpha: Option<i64>,
    pub lora_dropout: Option<String>,
    pub weight_decay: Option<String>,
    pub warmup_steps: Option<i64>,
    pub warmup_ratio: Option<String>,
    pub max_grad_norm: Option<String>,
    pub mixed_precision: Option<String>,
    pub channelwise_loss_weight: Option<String>,
    pub skip_reference_audio_codes: Option<bool>,
    pub prep_batch_size: Option<i64>,
    pub prep_n_vq: Option<i64>,
    pub lr_scheduler_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TtsScriptArgs {
    pub init_model_path: String,
    pub text: String,
    pub language: String,
    pub speaker: String,
    pub instruct: String,
    pub output_path: String,
    pub cfg_value: Option<String>,
    pub inference_timesteps: Option<i64>,
    pub n_vq_for_inference: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VoiceCloneScriptArgs {
    pub ref_audio_path: String,
    pub ref_text: String,
    pub init_model_path: String,
    pub language: String,
    pub output_path: String,
    pub text: String,
    pub mode: Option<String>,
    pub style_prompt: Option<String>,
    pub cfg_value: Option<String>,
    pub inference_timesteps: Option<i64>,
    pub n_vq_for_inference: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PythonScriptTaskArgs {
    Training(TrainingScriptArgs),
    TextToSpeech(TtsScriptArgs),
    VoiceClone(VoiceCloneScriptArgs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PythonScriptInvocationSpec {
    pub version: u32,
    pub base_model: String,
    pub kind: PythonScriptTaskKind,
    pub runtime: PythonScriptRuntimeOptions,
    pub target: PythonScriptExecutionTarget,
    pub args: PythonScriptTaskArgs,
}

impl PythonScriptInvocationSpec {
    pub(crate) fn write_to_json_file(&self, path: &Path) -> Result<()> {
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
