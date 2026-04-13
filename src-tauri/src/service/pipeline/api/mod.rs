#![allow(dead_code)]

use serde::{Deserialize, Serialize};

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
    pub tokenizer_model_path: String,
    pub input_jsonl: String,
    pub output_jsonl: String,
    pub output_model_path: String,
    pub batch_size: i64,
    pub lr: f64,
    pub num_epochs: i64,
    pub speaker_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TtsScriptArgs {
    pub init_model_path: String,
    pub text: String,
    pub language: String,
    pub speaker: String,
    pub instruct: String,
    pub output_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct VoiceCloneScriptArgs {
    pub ref_audio_path: String,
    pub ref_text: String,
    pub init_model_path: String,
    pub language: String,
    pub output_path: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PythonScriptTaskArgs {
    Training(TrainingScriptArgs),
    TextToSpeech(TtsScriptArgs),
    VoiceClone(VoiceCloneScriptArgs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PythonScriptInvocationSpec {
    pub kind: PythonScriptTaskKind,
    pub runtime: PythonScriptRuntimeOptions,
    pub target: PythonScriptExecutionTarget,
    pub args: PythonScriptTaskArgs,
}