use std::{fmt, str::FromStr};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::config::BaseModel;

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AppLanguage {
    #[serde(rename = "chinese", alias = "zh-CN")]
    Chinese,
    #[serde(rename = "english", alias = "en-US")]
    English,
    #[serde(rename = "japanese", alias = "ja-JP")]
    Japanese,
}

impl AppLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chinese => "chinese",
            Self::English => "english",
            Self::Japanese => "japanese",
        }
    }
}

impl fmt::Display for AppLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AppLanguage {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "chinese" | "zh-CN" => Ok(Self::Chinese),
            "english" | "en-US" => Ok(Self::English),
            "japanese" | "ja-JP" => Ok(Self::Japanese),
            other => Err(format!("不支持的语言类型: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TextToSpeechFormat {
    Wav,
    Mp3,
    Flac,
}

impl TextToSpeechFormat {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Mp3 => "mp3",
            Self::Flac => "flac",
        }
    }
}

impl fmt::Display for TextToSpeechFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TextToSpeechFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "wav" => Ok(Self::Wav),
            "mp3" => Ok(Self::Mp3),
            "flac" => Ok(Self::Flac),
            other => Err(format!("不支持的音频格式: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum HistoryTaskType {
    ModelTraining,
    TextToSpeech,
    VoiceClone,
}

impl HistoryTaskType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelTraining => "model-training",
            Self::TextToSpeech => "text-to-speech",
            Self::VoiceClone => "voice-clone",
        }
    }

    pub const fn storage_dir(self) -> &'static str {
        match self {
            Self::ModelTraining => "model_training",
            Self::TextToSpeech => "tts",
            Self::VoiceClone => "voice_clone",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Cancelled,
    Failed,
}

impl TaskStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub const fn is_finished(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Failed)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeakerStatus {
    Ready,
    Training,
    Disabled,
}

impl SpeakerStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Training => "training",
            Self::Disabled => "disabled",
        }
    }
}

impl fmt::Display for SpeakerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SpeakerStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "ready" => Ok(Self::Ready),
            "training" => Ok(Self::Training),
            "disabled" => Ok(Self::Disabled),
            other => Err(format!("不支持的说话人状态: {}", other)),
        }
    }
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            other => Err(format!("不支持的任务状态: {}", other)),
        }
    }
}

impl fmt::Display for HistoryTaskType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HistoryTaskType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "model-training" => Ok(Self::ModelTraining),
            "text-to-speech" => Ok(Self::TextToSpeech),
            "voice-clone" => Ok(Self::VoiceClone),
            other => Err(format!("不支持的历史任务类型: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeakerSource {
    Local,
    Remote,
}

impl SpeakerSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

impl fmt::Display for SpeakerSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SpeakerSource {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "local" => Ok(Self::Local),
            "remote" => Ok(Self::Remote),
            other => Err(format!("不支持的说话人来源: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelTrainingSampleType {
    Single,
    Dataset,
}

impl ModelTrainingSampleType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Dataset => "dataset",
        }
    }
}

impl fmt::Display for ModelTrainingSampleType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModelTrainingSampleType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "single" => Ok(Self::Single),
            "dataset" => Ok(Self::Dataset),
            other => Err(format!("不支持的模型训练样本类型: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelTrainingFileKind {
    Audio,
    Archive,
    Annotation,
}

impl ModelTrainingFileKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Archive => "archive",
            Self::Annotation => "annotation",
        }
    }
}

impl fmt::Display for ModelTrainingFileKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModelTrainingFileKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "audio" => Ok(Self::Audio),
            "archive" => Ok(Self::Archive),
            "annotation" => Ok(Self::Annotation),
            other => Err(format!("不支持的文件类型: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerInfo {
    pub id: i64,
    pub name: String,
    pub languages: Vec<AppLanguage>,
    pub samples: u32,
    pub base_model: BaseModel,
    pub create_time: String,
    pub modify_time: String,
    pub description: String,
    pub status: SpeakerStatus,
    pub source: SpeakerSource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSpeakerPayload {
    pub name: String,
    pub languages: Vec<AppLanguage>,
    pub samples: u32,
    pub base_model: BaseModel,
    pub description: String,
    pub status: SpeakerStatus,
    pub source: SpeakerSource,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSpeakerPayload {
    pub id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStatusPayload {
    pub task_id: i64,
    pub status: TaskStatus,
    pub duration_seconds: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: i64,
    pub base_model: BaseModel,
    pub model_name: String,
    pub model_scale: String,
    pub required_model_name_list: Vec<String>,
    pub required_model_repo_id_list: Vec<String>,
    pub supported_feature_list: Vec<String>,
    pub create_time: String,
    pub modify_time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Qwen3TtsTextToSpeechModelParams {
    pub voice_prompt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Qwen3TtsTrainingModelParams {
    pub epoch_count: i64,
    pub batch_size: i64,
    pub gradient_accumulation_steps: i64,
    pub enable_gradient_checkpointing: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VoxCpm2VoiceCloneMode {
    Reference,
    Ultimate,
}

impl VoxCpm2VoiceCloneMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reference => "reference",
            Self::Ultimate => "ultimate",
        }
    }
}

impl fmt::Display for VoxCpm2VoiceCloneMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VoxCpm2VoiceCloneMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "reference" => Ok(Self::Reference),
            "ultimate" => Ok(Self::Ultimate),
            other => Err(format!("不支持的 VoxCPM2 克隆模式: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum VoxCpm2TrainingMode {
    Full,
    Lora,
}

impl VoxCpm2TrainingMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Lora => "lora",
        }
    }
}

impl fmt::Display for VoxCpm2TrainingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VoxCpm2TrainingMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "full" => Ok(Self::Full),
            "lora" => Ok(Self::Lora),
            other => Err(format!("不支持的 VoxCPM2 训练模式: {}", other)),
        }
    }
}

const VOX_CPM2_DEFAULT_LORA_RANK: i64 = 32;
const VOX_CPM2_DEFAULT_LORA_ALPHA: i64 = 32;
const VOX_CPM2_DEFAULT_LORA_DROPOUT: &str = "0.0";

fn deserialize_optional_stringified_value<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.trim().to_string())),
        Some(Value::Number(value)) => Ok(Some(value.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "LoRA dropout 仅支持字符串或数字，当前为: {}",
            other
        ))),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoxCpm2TextToSpeechModelParams {
    pub cfg_value: f64,
    pub inference_timesteps: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoxCpm2VoiceCloneModelParams {
    pub mode: VoxCpm2VoiceCloneMode,
    pub style_prompt: String,
    pub cfg_value: f64,
    pub inference_timesteps: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoxCpm2TrainingModelParams {
    #[serde(default)]
    pub training_mode: Option<VoxCpm2TrainingMode>,
    #[serde(default)]
    pub use_lora: Option<bool>,
    #[serde(default)]
    pub lora_rank: Option<i64>,
    #[serde(default)]
    pub lora_alpha: Option<i64>,
    #[serde(default, deserialize_with = "deserialize_optional_stringified_value")]
    pub lora_dropout: Option<String>,
    pub epoch_count: i64,
    pub batch_size: i64,
    pub gradient_accumulation_steps: i64,
    pub enable_gradient_checkpointing: bool,
}

impl VoxCpm2TrainingModelParams {
    pub fn normalized(mut self) -> Self {
        let use_lora = self.use_lora();
        self.use_lora = Some(use_lora);
        self.training_mode = Some(if use_lora {
            VoxCpm2TrainingMode::Lora
        } else {
            VoxCpm2TrainingMode::Full
        });
        self.lora_rank = Some(self.lora_rank());
        self.lora_alpha = Some(self.lora_alpha());
        self.lora_dropout = Some(self.lora_dropout());
        self
    }

    pub fn use_lora(&self) -> bool {
        self.use_lora.unwrap_or(matches!(
            self.training_mode,
            Some(VoxCpm2TrainingMode::Lora)
        ))
    }

    pub fn training_mode_value(&self) -> VoxCpm2TrainingMode {
        if self.use_lora() {
            VoxCpm2TrainingMode::Lora
        } else {
            VoxCpm2TrainingMode::Full
        }
    }

    pub fn lora_rank(&self) -> i64 {
        self.lora_rank.unwrap_or(VOX_CPM2_DEFAULT_LORA_RANK).max(1)
    }

    pub fn lora_alpha(&self) -> i64 {
        self.lora_alpha
            .unwrap_or(VOX_CPM2_DEFAULT_LORA_ALPHA)
            .max(1)
    }

    pub fn lora_dropout(&self) -> String {
        self.lora_dropout
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(VOX_CPM2_DEFAULT_LORA_DROPOUT)
            .to_string()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextToSpeechTaskDetail {
    pub speaker_id: i64,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub text: String,
    pub model_params: Value,
    pub char_count: usize,
    pub file_name: String,
    pub output_file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrainingTaskDetail {
    pub language: AppLanguage,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub model_name: String,
    pub model_params: Value,
    pub sample_count: i64,
    pub samples: Vec<ModelTrainingSampleInput>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCloneTaskDetail {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub ref_audio_name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
    pub text: String,
    pub model_params: Value,
    pub char_count: usize,
    pub file_name: String,
    pub output_file_path: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub id: i64,
    pub task_type: HistoryTaskType,
    pub title: String,
    pub speaker: String,
    pub status: TaskStatus,
    pub duration_seconds: i64,
    pub create_time: String,
    pub modify_time: String,
    pub error_message: Option<String>,
    pub detail: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTextToSpeechTaskPayload {
    pub speaker_id: i64,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub text: String,
    pub model_params: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextToSpeechTaskResult {
    pub task_id: i64,
    pub file_name: String,
    pub speaker_id: i64,
    pub speaker_label: String,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub duration_seconds: i64,
    pub text: String,
    pub model_params: Value,
    pub created_at: String,
    pub status: TaskStatus,
    pub output_file_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextToSpeechAudioAsset {
    pub task_id: i64,
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCloneAudioAsset {
    pub task_id: i64,
    pub file_name: String,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrainingFileInput {
    pub file_name: String,
    pub file_kind: ModelTrainingFileKind,
    pub file_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrainingSampleInput {
    pub id: i64,
    pub sample_type: ModelTrainingSampleType,
    pub title: String,
    pub detail: String,
    pub transcript_preview: Option<String>,
    pub primary_file: ModelTrainingFileInput,
    pub secondary_file: Option<ModelTrainingFileInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelTrainingTaskPayload {
    pub language: AppLanguage,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub model_name: String,
    pub model_params: Value,
    pub samples: Vec<ModelTrainingSampleInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVoiceCloneTaskPayload {
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub ref_audio_name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
    pub text: String,
    pub model_params: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrainingTaskResult {
    pub task_id: i64,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub model_name: String,
    pub model_params: Value,
    pub sample_count: i64,
    pub create_time: String,
    pub status: TaskStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCloneTaskResult {
    pub task_id: i64,
    pub file_name: String,
    pub ref_audio_name: String,
    pub base_model: BaseModel,
    pub model_scale: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub duration_seconds: i64,
    pub ref_text: String,
    pub text: String,
    pub model_params: Value,
    pub created_at: String,
    pub status: TaskStatus,
    pub output_file_path: String,
}
