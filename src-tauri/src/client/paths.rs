//! 远端 HTTP 接口路径常量（以 `/api` 为前缀，与 `api_url` 拼接组成完整地址）。
//!
//! 含路径参数的接口使用 `{id}` 占位，运行时由 [`with_id`] 替换。

// ---- Speaker ----
pub const SPEAKERS: &str = "/api/speakers";
pub const SPEAKERS_IMPORT: &str = "/api/speakers/import";
pub const SPEAKER_BY_ID: &str = "/api/speakers/{id}";

// ---- Model ----
pub const MODELS: &str = "/api/models";
pub const MODELS_DEVICE_TYPE: &str = "/api/models/device-type";
pub const MODEL_BY_ID: &str = "/api/models/{id}";
pub const MODEL_INSTALL: &str = "/api/models/{id}/install";
pub const MODEL_CURRENT_DEVICE: &str = "/api/models/{id}/current-device";

// ---- History ----
pub const HISTORY: &str = "/api/history";
pub const HISTORY_BY_ID: &str = "/api/history/{id}";
pub const HISTORY_STATUS: &str = "/api/history/{id}/status";
pub const HISTORY_CANCEL: &str = "/api/history/{id}/cancel";
pub const HISTORY_AUDIO_TTS: &str = "/api/history/{id}/audio/text-to-speech";
pub const HISTORY_AUDIO_VOICE_CLONE: &str = "/api/history/{id}/audio/voice-clone";
pub const HISTORY_AUDIO_VOICE_DESIGN: &str = "/api/history/{id}/audio/voice-design";
pub const HISTORY_TEXT_TO_SPEECH: &str = "/api/history/text-to-speech";
pub const HISTORY_MODEL_TRAINING: &str = "/api/history/model-training";
pub const HISTORY_VOICE_CLONE: &str = "/api/history/voice-clone";
pub const HISTORY_VOICE_DESIGN: &str = "/api/history/voice-design";

/// 将路径中的 `{id}` 占位符替换为实际主键。
pub fn with_id(path: &str, id: i64) -> String {
    path.replace("{id}", &id.to_string())
}
