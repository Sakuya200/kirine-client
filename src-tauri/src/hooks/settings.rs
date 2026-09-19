use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{
    resolve_base_log_dir, resolve_storage_dir, save_configs, AttentionImplementation, BasicConfig,
    EnvConfig, RemoteConfig, UiConfigCatalog,
};
use crate::error::{codes, AppError};
use crate::service::{ServiceImpl, ServiceState};
use crate::utils::file_ops::migrate_directory;

pub struct EnvConfigState(pub RwLock<EnvConfig>);
pub struct UiConfigState(pub Arc<UiConfigCatalog>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub api_url: String,
    pub api_token: String,
    pub model_dir: String,
    pub data_dir: String,
    pub log_cache_dir: String,
    pub attn_implementation: String,
    #[serde(default)]
    pub language: Option<String>,
    pub restart_required: bool,
    pub migrated_directories: Vec<String>,
    pub removable_directories: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsPayload {
    pub api_url: String,
    pub api_token: String,
    pub model_dir: String,
    pub data_dir: String,
    pub log_cache_dir: String,
    pub attn_implementation: String,
    #[serde(default)]
    pub language: Option<String>,
}

impl SettingsPayload {
    fn from_env_config(config: &EnvConfig) -> Self {
        Self {
            api_url: config.api_url().unwrap_or_default().to_string(),
            api_token: config.api_token().unwrap_or_default().to_string(),
            model_dir: resolve_storage_dir(config.model_dir(), "models")
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default(),
            data_dir: resolve_storage_dir(config.data_dir(), "data")
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default(),
            log_cache_dir: resolve_base_log_dir(config.log_dir())
                .map(|path| path.to_string_lossy().to_string())
                .unwrap_or_default(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
            language: config.basic.language.clone(),
            restart_required: false,
            migrated_directories: Vec::new(),
            removable_directories: Vec::new(),
        }
    }
}

fn normalized_path(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn push_migrated_directory(list: &mut Vec<String>, migrated: bool, label: &str) {
    if migrated {
        list.push(label.to_string());
    }
}

#[tauri::command]
pub fn get_settings_config(
    state: State<'_, EnvConfigState>,
) -> std::result::Result<SettingsPayload, AppError> {
    let config = state
        .0
        .read()
        .map_err(|_| AppError::coded(codes::SETTINGS_READ_STATE_FAILED, "读取配置状态失败"))?;

    Ok(SettingsPayload::from_env_config(&config))
}

#[tauri::command]
pub fn get_ui_config(
    state: State<'_, UiConfigState>,
) -> std::result::Result<UiConfigCatalog, String> {
    Ok(state.0.as_ref().clone())
}

#[tauri::command]
pub fn save_settings_config(
    payload: SaveSettingsPayload,
    state: State<'_, EnvConfigState>,
    service_state: State<'_, ServiceState>,
) -> std::result::Result<SettingsPayload, AppError> {
    let mut config = state
        .0
        .write()
        .map_err(|_| AppError::coded(codes::SETTINGS_WRITE_STATE_FAILED, "写入配置状态失败"))?;

    let persisted_config = config.clone();
    let attn_implementation = payload
        .attn_implementation
        .parse::<AttentionImplementation>()
        .map_err(|err| AppError::plain(err.to_string()))?;
    let next_data_dir = normalized_path(&payload.data_dir);
    let next_log_dir = normalized_path(&payload.log_cache_dir);
    let next_model_dir = normalized_path(&payload.model_dir);

    let current_data_dir =
        resolve_storage_dir(persisted_config.data_dir(), "data")?;
    let current_log_dir =
        resolve_base_log_dir(persisted_config.log_dir())?;
    let current_model_dir = resolve_storage_dir(persisted_config.model_dir(), "models")
        ?;

    let resolved_next_data_dir =
        resolve_storage_dir(next_data_dir.as_deref(), "data")?;
    let resolved_next_log_dir =
        resolve_base_log_dir(next_log_dir.as_deref())?;
    let resolved_next_model_dir =
        resolve_storage_dir(next_model_dir.as_deref(), "models")?;

    let mut migrated_directories = Vec::new();
    let mut removable_directories = Vec::new();
    let data_migrated =
        migrate_directory(&current_data_dir, &resolved_next_data_dir, "数据", false)
            ?;
    push_migrated_directory(&mut migrated_directories, data_migrated, "数据目录");
    if data_migrated {
        removable_directories.push(current_data_dir.display().to_string());
    }
    let log_migrated = migrate_directory(&current_log_dir, &resolved_next_log_dir, "日志", false)
        ?;
    push_migrated_directory(&mut migrated_directories, log_migrated, "日志目录");
    if log_migrated {
        removable_directories.push(current_log_dir.display().to_string());
    }
    let model_migrated =
        migrate_directory(&current_model_dir, &resolved_next_model_dir, "模型", true)
            ?;
    push_migrated_directory(&mut migrated_directories, model_migrated, "模型目录");

    let next_config = EnvConfig {
        basic: BasicConfig {
            mode: persisted_config.basic.mode,
            data_dir: Some(resolved_next_data_dir.to_string_lossy().to_string()),
            log_dir: Some(resolved_next_log_dir.to_string_lossy().to_string()),
            model_dir: Some(resolved_next_model_dir.to_string_lossy().to_string()),
            language: payload.language.clone().or(persisted_config.basic.language.clone()),
        },
        remote: Some(RemoteConfig {
            api_url: Some(payload.api_url.trim().to_string()),
            api_token: Some(payload.api_token.trim().to_string()),
        }),
        training: persisted_config
            .training
            .clone()
            .with_attn_implementation(attn_implementation),
    };

    save_configs(&next_config)?;
    *config = next_config.clone();
    if let ServiceImpl::Local(local) = &service_state.0 {
        local
            .replace_runtime_config(next_config.clone())
            ?;
    }

    Ok(SettingsPayload {
        restart_required: !migrated_directories.is_empty(),
        migrated_directories,
        removable_directories,
        ..SettingsPayload::from_env_config(&next_config)
    })
}

/// 独立保存 UI 语言，不经 save_settings_config 的目录迁移逻辑。
#[tauri::command]
pub fn save_ui_language(
    language: String,
    state: State<'_, EnvConfigState>,
) -> std::result::Result<(), AppError> {
    let mut config = state
        .0
        .write()
        .map_err(|_| AppError::coded(codes::SETTINGS_WRITE_STATE_FAILED, "写入配置状态失败"))?;
    config.basic.language = Some(language);
    save_configs(&config).map_err(AppError::from_anyhow)
}
