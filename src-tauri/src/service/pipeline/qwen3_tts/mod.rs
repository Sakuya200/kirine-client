mod preset_speakers;
mod training;
mod tts;
mod voice_clone;

use std::{
    path::{Path, PathBuf},
    sync::LazyLock,
};

pub(crate) use preset_speakers::qwen3_tts_preset_speakers;
pub(crate) use tts::resolve_inference_model_path;

use async_trait::async_trait;
use serde_json::to_string;

use crate::{
    service::{
        local::LocalService,
        pipeline::{
            llm_models::LlmModelDefinition, model_paths::LlmModelPaths, ModelTaskPipeline,
            TrainingPipelineRequest, TtsPipelineRequest, VoiceClonePipelineRequest,
        },
    },
    Result,
};

const QWEN3_TTS_BASE_MODEL_NAME: &str = "Qwen3-TTS-12Hz-1.7B-Base";
const QWEN3_TTS_TOKENIZER_NAME: &str = "Qwen3-TTS-Tokenizer-12Hz";
const QWEN3_TTS_BASE_MODEL_REPO_ID: &str = "Qwen/Qwen3-TTS-12Hz-1.7B-Base";
const QWEN3_TTS_TOKENIZER_REPO_ID: &str = "Qwen/Qwen3-TTS-Tokenizer-12Hz";
pub(crate) const QWEN3_TTS_CUSTOM_VOICE_MODEL_NAME: &str = "Qwen3-TTS-12Hz-1.7B-CustomVoice";
const QWEN3_TTS_CUSTOM_VOICE_MODEL_REPO_ID: &str = "Qwen/Qwen3-TTS-12Hz-1.7B-CustomVoice";
pub(crate) const QWEN3_TTS_DISPLAY_NAME: &str = "Qwen3-TTS";
const QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR: &str = "qwen3_tts";
const QWEN3_TTS_MODEL_ARTIFACTS_DIR: &str = "base-models";

pub(crate) static QWEN3_TTS_MODEL_DEFINITION: LazyLock<LlmModelDefinition> =
    LazyLock::new(|| LlmModelDefinition {
        display_name: QWEN3_TTS_DISPLAY_NAME,
        artifacts_dir: QWEN3_TTS_MODEL_ARTIFACTS_DIR,
        python_script_dir: QWEN3_TTS_MODEL_PYTHON_SCRIPT_DIR,
        required_model_name_list: vec![
            QWEN3_TTS_BASE_MODEL_NAME,
            QWEN3_TTS_TOKENIZER_NAME,
            QWEN3_TTS_CUSTOM_VOICE_MODEL_NAME,
        ],
        required_model_repo_id_list: vec![
            QWEN3_TTS_BASE_MODEL_REPO_ID,
            QWEN3_TTS_TOKENIZER_REPO_ID,
            QWEN3_TTS_CUSTOM_VOICE_MODEL_REPO_ID,
        ],
    });

pub(crate) static QWEN3_TTS_MODEL_PATHS: Qwen3TtsModelPaths = Qwen3TtsModelPaths;

pub(crate) static QWEN3_TTS_MODEL_TASK_PIPELINE: Qwen3TTSModelTaskPipeline =
    Qwen3TTSModelTaskPipeline;

pub(crate) struct Qwen3TtsModelPaths;

pub(crate) struct Qwen3TTSModelTaskPipeline;

impl LlmModelPaths for Qwen3TtsModelPaths {
    fn definition(&self) -> &'static LlmModelDefinition {
        &QWEN3_TTS_MODEL_DEFINITION
    }

    fn download_script_args(&self, src_model_root: &Path) -> Vec<String> {
        let definition = self.definition();
        let target_root_dir = src_model_root.join(definition.artifacts_dir);

        vec![
            "--model-id-list".to_string(),
            to_string(&definition.required_model_repo_id_list)
                .expect("serialize required model repo ids"),
            "--model-name-list".to_string(),
            to_string(&definition.required_model_name_list)
                .expect("serialize required model names"),
            "--target-root-dir".to_string(),
            target_root_dir.to_string_lossy().to_string(),
        ]
    }

    fn training_tokenizer_model_path(&self, src_model_root: &Path) -> PathBuf {
        self.artifact_path(
            src_model_root,
            &self.definition().required_model_name_list[1],
        )
    }

    fn training_init_model_path(&self, src_model_root: &Path) -> PathBuf {
        self.artifact_path(
            src_model_root,
            &self.definition().required_model_name_list[0],
        )
    }

    fn prepared_model_download_paths(&self, src_model_root: &Path) -> Vec<PathBuf> {
        self.definition()
            .required_model_name_list
            .iter()
            .map(|model_name| self.artifact_path(src_model_root, model_name))
            .collect()
    }

    fn voice_clone_init_model_path(&self, src_model_root: &Path) -> PathBuf {
        self.training_init_model_path(src_model_root)
    }
}

#[async_trait]
impl ModelTaskPipeline for Qwen3TTSModelTaskPipeline {
    async fn run_training_pipeline(
        &self,
        service: &LocalService,
        request: TrainingPipelineRequest,
    ) -> Result<()> {
        self.run_training_pipeline_impl(service, request).await
    }

    async fn run_tts_pipeline(
        &self,
        service: &LocalService,
        request: TtsPipelineRequest,
    ) -> Result<()> {
        self.run_tts_pipeline_impl(service, request).await
    }

    async fn run_voice_clone_pipeline(
        &self,
        service: &LocalService,
        request: VoiceClonePipelineRequest,
    ) -> Result<()> {
        self.run_voice_clone_pipeline_impl(service, request).await
    }
}
