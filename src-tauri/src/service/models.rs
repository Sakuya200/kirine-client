use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{BaseModel, HardwareType};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum AppLanguage {
    #[serde(rename = "chinese", alias = "zh-CN")]
    Chinese,
    #[serde(rename = "english", alias = "en-US")]
    English,
    #[serde(rename = "japanese", alias = "ja-JP")]
    Japanese,
    #[serde(rename = "korean", alias = "ko-KR")]
    Korean,
}

impl AppLanguage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chinese => "chinese",
            Self::English => "english",
            Self::Japanese => "japanese",
            Self::Korean => "korean",
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
            "korean" | "ko-KR" => Ok(Self::Korean),
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
    VoiceDesign,
    StreamingSpeech,
}

impl HistoryTaskType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ModelTraining => "model-training",
            Self::TextToSpeech => "text-to-speech",
            Self::VoiceClone => "voice-clone",
            Self::VoiceDesign => "voice-design",
            Self::StreamingSpeech => "streaming-speech",
        }
    }

    pub const fn storage_dir(self) -> &'static str {
        match self {
            Self::ModelTraining => "model_training",
            Self::TextToSpeech => "tts",
            Self::VoiceClone => "voice_clone",
            Self::VoiceDesign => "voice_design",
            Self::StreamingSpeech => "streaming",
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
            "voice-design" => Ok(Self::VoiceDesign),
            "streaming-speech" => Ok(Self::StreamingSpeech),
            other => Err(format!("不支持的历史任务类型: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SpeakerSource {
    Local,
    Preset,
    Remote,
}

impl SpeakerSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Preset => "preset",
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
            "preset" => Ok(Self::Preset),
            "remote" => Ok(Self::Remote),
            other => Err(format!("不支持的说话人来源: {}", other)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ModelDownloadType {
    #[serde(rename = "HF-Like")]
    HfLike,
    #[serde(rename = "Custom")]
    Custom,
}

impl ModelDownloadType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HfLike => "HF-Like",
            Self::Custom => "Custom",
        }
    }
}

impl fmt::Display for ModelDownloadType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ModelDownloadType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "HF-Like" | "hf-like" | "hflike" | "hf_like" => Ok(Self::HfLike),
            "Custom" | "custom" => Ok(Self::Custom),
            other => Err(format!("不支持的模型下载类型: {}", other)),
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
    pub speaker_name: String,
    pub samples: u32,
    pub base_model: BaseModel,
    pub create_time: String,
    pub modify_time: String,
    pub description: String,
    pub status: SpeakerStatus,
    pub source: SpeakerSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSpeakerPayload {
    pub speaker_name: String,
    pub samples: u32,
    pub base_model: BaseModel,
    pub description: String,
    pub status: SpeakerStatus,
    pub source: SpeakerSource,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSpeakerPayload {
    pub id: i64,
    pub speaker_name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportModelAsSpeakerPayload {
    pub base_model: BaseModel,
    pub model_version: String,
    pub source_model_dir_path: String,
    pub speaker_name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskStatusPayload {
    pub task_id: i64,
    pub status: TaskStatus,
    pub duration_seconds: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: i64,
    pub base_model: BaseModel,
    pub model_name: String,
    pub model_version: String,
    pub download_type: ModelDownloadType,
    pub required_model_name_list: Vec<String>,
    pub required_model_repo_id_list: Vec<String>,
    pub supported_feature_list: Vec<String>,
    pub supported_devices: Vec<HardwareType>,
    pub supported_languages: Vec<AppLanguage>,
    pub downloaded: bool,
    pub create_time: String,
    pub modify_time: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelMutationResult {
    pub model: ModelInfo,
    pub removed_paths: Vec<String>,
    pub preserved_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TextToSpeechTaskDetail {
    pub speaker_id: Option<i64>,
    pub base_model: BaseModel,
    pub model_version: String,
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
    pub model_version: String,
    pub speaker_name: String,
    pub description: String,
    pub model_params: Value,
    pub sample_count: i64,
    pub samples: Vec<ModelTrainingSampleInput>,
    pub notes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceCloneTaskDetail {
    pub base_model: BaseModel,
    pub model_version: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VoiceDesignTaskDetail {
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub prompt: String,
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
    pub device: HardwareType,
    pub create_time: String,
    pub modify_time: String,
    pub task_log: Option<String>,
    pub detail: serde_json::Value,
}

/// 历史任务列表摘要（不含 detail/task_log，用于分页列表查询，消除 N+1 全量抓取）
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecordSummary {
    pub id: i64,
    pub task_type: HistoryTaskType,
    pub title: String,
    pub speaker: String,
    pub status: TaskStatus,
    pub duration_seconds: i64,
    pub device: HardwareType,
    pub create_time: String,
    pub modify_time: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTextToSpeechTaskPayload {
    pub speaker_id: Option<i64>,
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub device: HardwareType,
    pub text: String,
    pub model_params: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextToSpeechTaskResult {
    pub task_id: i64,
    pub file_name: String,
    pub speaker_id: Option<i64>,
    pub speaker_label: String,
    pub base_model: BaseModel,
    pub model_version: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceDesignAudioAsset {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateModelTrainingTaskPayload {
    pub language: AppLanguage,
    pub base_model: BaseModel,
    pub model_version: String,
    pub speaker_name: String,
    pub description: String,
    pub device: HardwareType,
    pub model_params: Value,
    pub samples: Vec<ModelTrainingSampleInput>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVoiceCloneTaskPayload {
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub device: HardwareType,
    pub ref_audio_name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
    pub text: String,
    pub model_params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVoiceDesignTaskPayload {
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub device: HardwareType,
    pub prompt: String,
    pub text: String,
    pub model_params: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelTrainingTaskResult {
    pub task_id: i64,
    pub base_model: BaseModel,
    pub model_version: String,
    pub speaker_name: String,
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
    pub model_version: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceDesignTaskResult {
    pub task_id: i64,
    pub file_name: String,
    pub base_model: BaseModel,
    pub model_version: String,
    pub language: AppLanguage,
    pub format: TextToSpeechFormat,
    pub export_audio_name: String,
    pub duration_seconds: i64,
    pub prompt: String,
    pub text: String,
    pub model_params: Value,
    pub created_at: String,
    pub status: TaskStatus,
    pub output_file_path: String,
}

/// 通用分页查询请求结构体（统一前后端分页请求对象）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest<T> {
    pub page: u32,
    pub page_size: u32,
    pub filter: Option<T>,
}

impl<T> Default for PageRequest<T> {
    fn default() -> Self {
        PageRequest {
            page: 1,
            page_size: 10,
            filter: None,
        }
    }
}

/// 通用分页响应结构体
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        let total_pages = if page_size == 0 {
            0
        } else {
            ((total as u32) + page_size - 1) / page_size
        };
        Page {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerFilter {
    pub keyword: Option<String>,
    pub status: Option<SpeakerStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelFilter {
    pub keyword: Option<String>,
    pub downloaded: Option<bool>,
    pub feature: Option<HistoryTaskType>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct HistoryFilter {
    pub keyword: Option<String>,
    pub task_type: Option<HistoryTaskType>,
    pub status: Option<TaskStatus>,
}

/// 说话人分页响应（附带统计，供页面统计卡使用）
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerPageResult {
    pub items: Vec<SpeakerInfo>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
    pub ready_count: u64,
    pub training_count: u64,
    pub disabled_count: u64,
    pub total_samples: u64,
}
