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
    VoiceDesign,
    StreamingSpeech,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct PythonScriptRuntimeOptions {
    pub device: Option<String>,
    pub logging_dir: Option<String>,
    pub attn_implementation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrainingArgs {
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
pub(crate) struct TTSArgs {
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
pub(crate) struct VoiceCloneArgs {
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
pub(crate) struct VoiceDesignArgs {
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
pub(crate) struct StreamingSpeakerArg {
    pub name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StreamingArgs {
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub speakers: Vec<StreamingSpeakerArg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum PythonScriptTaskArgs {
    Training(TrainingArgs),
    TextToSpeech(TTSArgs),
    VoiceClone(VoiceCloneArgs),
    VoiceDesign(VoiceDesignArgs),
    Streaming(StreamingArgs),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PythonScriptInvocationSpec {
    pub version: String,
    pub base_model: String,
    pub model_version: String,
    pub kind: PythonScriptTaskKind,
    pub runtime: PythonScriptRuntimeOptions,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// 规则4：`PythonScriptInvocationSpec::write_to_json_file` 写入的 params 文件即
    /// 「调用模型层 python 脚本时的参数」。此处对 4 种任务类型分别构造 spec、写入临时
    /// 文件、读回 JSON，断言 kind 标签与各 args 的关键字段被正确序列化。
    fn unique_path(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "kirine-params-{}-{}-{}.json",
            std::process::id(),
            label,
            n
        ))
    }

    fn runtime() -> PythonScriptRuntimeOptions {
        PythonScriptRuntimeOptions {
            device: Some("cpu".to_string()),
            logging_dir: None,
            attn_implementation: Some("sdpa".to_string()),
        }
    }

    fn write_and_read(spec: &PythonScriptInvocationSpec, label: &str) -> Value {
        let path = unique_path(label);
        spec.write_to_json_file(&path)
            .expect("write spec to json file");
        let bytes = fs::read(&path).expect("read back params file");
        let _ = fs::remove_file(&path);
        serde_json::from_slice(&bytes).expect("parse params json")
    }

    #[test]
    fn writes_tts_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "qwen3_tts".to_string(),
            model_version: "1.7B".to_string(),
            kind: PythonScriptTaskKind::TextToSpeech,
            runtime: runtime(),
            args: PythonScriptTaskArgs::TextToSpeech(TTSArgs {
                model_root_path: "/models/root".to_string(),
                speaker_dir_name: None,
                model_params_json: serde_json::json!({}),
                text: "你好".to_string(),
                language: "chinese".to_string(),
                speaker: "Alice".to_string(),
                output_path: "/out/tts.wav".to_string(),
            }),
        };

        let json = write_and_read(&spec, "tts");
        assert_eq!(json["base_model"], "qwen3_tts");
        assert_eq!(json["model_version"], "1.7B");
        assert_eq!(json["kind"], "TextToSpeech");
        assert_eq!(json["args"]["TextToSpeech"]["text"], "你好");
        assert_eq!(json["args"]["TextToSpeech"]["language"], "chinese");
        assert_eq!(json["args"]["TextToSpeech"]["output_path"], "/out/tts.wav");
        assert_eq!(json["runtime"]["device"], "cpu");
    }

    #[test]
    fn writes_voice_clone_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "qwen3_tts".to_string(),
            model_version: "1.7B".to_string(),
            kind: PythonScriptTaskKind::VoiceClone,
            runtime: runtime(),
            args: PythonScriptTaskArgs::VoiceClone(VoiceCloneArgs {
                model_root_path: "/models/root".to_string(),
                speaker_dir_name: None,
                model_params_json: serde_json::json!({}),
                ref_audio_path: "/ref.wav".to_string(),
                ref_text: Some("参考".to_string()),
                language: "chinese".to_string(),
                output_path: "/out/vc.wav".to_string(),
                text: "生成".to_string(),
            }),
        };

        let json = write_and_read(&spec, "vc");
        assert_eq!(json["kind"], "VoiceClone");
        assert_eq!(json["args"]["VoiceClone"]["ref_audio_path"], "/ref.wav");
        assert_eq!(json["args"]["VoiceClone"]["ref_text"], "参考");
    }

    #[test]
    fn writes_voice_design_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "vox_cpm2".to_string(),
            model_version: "2B".to_string(),
            kind: PythonScriptTaskKind::VoiceDesign,
            runtime: runtime(),
            args: PythonScriptTaskArgs::VoiceDesign(VoiceDesignArgs {
                model_root_path: "/models/root".to_string(),
                speaker_dir_name: None,
                model_params_json: serde_json::json!({}),
                text: "生成".to_string(),
                language: "chinese".to_string(),
                instruct: "温柔".to_string(),
                output_path: "/out/vd.wav".to_string(),
            }),
        };

        let json = write_and_read(&spec, "vd");
        assert_eq!(json["kind"], "VoiceDesign");
        assert_eq!(json["args"]["VoiceDesign"]["instruct"], "温柔");
        assert_eq!(json["base_model"], "vox_cpm2");
    }

    #[test]
    fn writes_training_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "qwen3_tts".to_string(),
            model_version: "1.7B".to_string(),
            kind: PythonScriptTaskKind::Training,
            runtime: runtime(),
            args: PythonScriptTaskArgs::Training(TrainingArgs {
                model_root_path: "/models/root".to_string(),
                speaker_dir_name: None,
                model_params_json: serde_json::json!({}),
                input_jsonl: "/in/manifest.jsonl".to_string(),
                output_jsonl: "/out/manifest.jsonl".to_string(),
                output_model_path: "/out/model".to_string(),
                batch_size: 4,
                lr: Some("2e-5".to_string()),
                num_epochs: 12,
                speaker_name: "Alice".to_string(),
                gradient_accumulation_steps: 2,
            }),
        };

        let json = write_and_read(&spec, "train");
        assert_eq!(json["kind"], "Training");
        assert_eq!(json["args"]["Training"]["input_jsonl"], "/in/manifest.jsonl");
        assert_eq!(json["args"]["Training"]["batch_size"], 4);
        assert_eq!(json["args"]["Training"]["num_epochs"], 12);
        assert_eq!(json["args"]["Training"]["lr"], "2e-5");
    }

    #[test]
    fn writes_streaming_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "gpt_sovits_cpufast".to_string(),
            model_version: "v1".to_string(),
            kind: PythonScriptTaskKind::StreamingSpeech,
            runtime: runtime(),
            args: PythonScriptTaskArgs::Streaming(StreamingArgs {
                context_file_path: "/ctx/context.json".to_string(),
                input_cache_file_path: "/ctx/input.jsonl".to_string(),
                output_audio_dir: "/ctx/audio".to_string(),
                speakers: vec![StreamingSpeakerArg {
                    name: "A".to_string(),
                    ref_audio_path: "/ref.wav".to_string(),
                    ref_text: "参考".to_string(),
                }],
            }),
        };

        let json = write_and_read(&spec, "streaming");
        assert_eq!(json["kind"], "StreamingSpeech");
        assert_eq!(json["args"]["Streaming"]["context_file_path"], "/ctx/context.json");
        assert_eq!(json["args"]["Streaming"]["input_cache_file_path"], "/ctx/input.jsonl");
        assert_eq!(json["args"]["Streaming"]["speakers"][0]["name"], "A");
        assert_eq!(json["args"]["Streaming"]["speakers"][0]["ref_text"], "参考");
    }
}
