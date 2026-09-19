//! 结构化应用错误：`code` 命中前端 `errors.*` 词条时按码翻译，未命中透传 `message`。
//! 命令错误通道从 `String`/`anyhow::Error` 渐进迁移到本类型（见 docs 计划 i18n Task 6）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 首批错误码常量。前端词条位于 `src/locales/<lang>/errors.ts`，两侧须同步。
pub mod codes {
    pub const MODEL_DEVICE_UNSUPPORTED: &str = "model.deviceUnsupported";
    pub const MODEL_NOT_INSTALLED: &str = "model.notInstalled";
    pub const TASK_HANDLE_READ_FAILED: &str = "task.handleReadFailed";
    pub const TASK_TERMINATE_SIGNAL_FAILED: &str = "task.terminateSignalFailed";
    pub const SETTINGS_READ_STATE_FAILED: &str = "settings.readStateFailed";
    pub const SETTINGS_WRITE_STATE_FAILED: &str = "settings.writeStateFailed";
    pub const VALIDATION_SPEAKER_NAME_REQUIRED: &str = "validation.speakerNameRequired";
    pub const VALIDATION_SPEAKER_DESCRIPTION_REQUIRED: &str = "validation.speakerDescriptionRequired";
    pub const VALIDATION_BASE_MODEL_TYPE_REQUIRED: &str = "validation.baseModelTypeRequired";
    pub const VALIDATION_MODEL_VERSION_REQUIRED: &str = "validation.modelVersionRequired";
    pub const VALIDATION_REF_AUDIO_REQUIRED: &str = "validation.refAudioRequired";
    pub const VALIDATION_TEXT_REQUIRED: &str = "validation.textRequired";
    pub const VALIDATION_PROMPT_REQUIRED: &str = "validation.promptRequired";
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub params: HashMap<String, String>,
}

impl AppError {
    pub fn coded(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: Some(code.to_string()),
            message: message.into(),
            params: HashMap::new(),
        }
    }

    pub fn plain(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            params: HashMap::new(),
        }
    }

    pub fn with_param(mut self, key: &str, value: impl Into<String>) -> Self {
        self.params.insert(key.to_string(), value.into());
        self
    }

    /// 沿 anyhow 传播链抛出；hooks 边界用 [`from_anyhow`] 还原。
    pub fn into_anyhow(self) -> anyhow::Error {
        anyhow::Error::new(self)
    }

    /// hooks 命令边界统一转换：命中 AppError 则透传，否则按 plain 包装原文。
    pub fn from_anyhow(err: anyhow::Error) -> Self {
        match err.downcast::<AppError>() {
            Ok(app_err) => app_err,
            Err(err) => AppError::plain(err.to_string()),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::plain(err.to_string())
    }
}
