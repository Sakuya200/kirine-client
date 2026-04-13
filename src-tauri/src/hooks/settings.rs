use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{
    load_configs, resolve_base_log_dir, resolve_storage_dir, save_configs, AttentionImplementation,
    BasicConfig, EnvConfig, RemoteConfig,
};
use crate::utils::file_ops::migrate_directory;

pub struct EnvConfigState(pub RwLock<EnvConfig>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub api_url: String,
    pub api_token: String,
    pub model_dir: String,
    pub data_dir: String,
    pub log_cache_dir: String,
    pub attn_implementation: String,
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
}

impl SettingsPayload {
    fn from_env_config(config: &EnvConfig) -> Self {
        Self {
            api_url: config.api_url().unwrap_or_default().to_string(),
            api_token: config.api_token().unwrap_or_default().to_string(),
            model_dir: config.model_dir().unwrap_or_default().to_string(),
            data_dir: config.data_dir().unwrap_or_default().to_string(),
            log_cache_dir: config.log_dir().unwrap_or_default().to_string(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
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
) -> std::result::Result<SettingsPayload, String> {
    let config = state.0.read().map_err(|_| "读取配置状态失败".to_string())?;

    Ok(SettingsPayload::from_env_config(&config))
}

#[tauri::command]
pub fn save_settings_config(
    payload: SaveSettingsPayload,
    state: State<'_, EnvConfigState>,
) -> std::result::Result<SettingsPayload, String> {
    let mut config = state
        .0
        .write()
        .map_err(|_| "写入配置状态失败".to_string())?;

    let persisted_config = load_configs().unwrap_or_else(|_| config.clone());
    let attn_implementation = payload
        .attn_implementation
        .parse::<AttentionImplementation>()
        .map_err(|err| err.to_string())?;
    let next_data_dir = normalized_path(&payload.data_dir);
    let next_log_dir = normalized_path(&payload.log_cache_dir);
    let next_model_dir = normalized_path(&payload.model_dir);

    let current_data_dir =
        resolve_storage_dir(persisted_config.data_dir(), "data").map_err(|err| err.to_string())?;
    let current_log_dir =
        resolve_base_log_dir(persisted_config.log_dir()).map_err(|err| err.to_string())?;
    let current_model_dir = resolve_storage_dir(persisted_config.model_dir(), "models")
        .map_err(|err| err.to_string())?;

    let resolved_next_data_dir =
        resolve_storage_dir(next_data_dir.as_deref(), "data").map_err(|err| err.to_string())?;
    let resolved_next_log_dir =
        resolve_base_log_dir(next_log_dir.as_deref()).map_err(|err| err.to_string())?;
    let resolved_next_model_dir =
        resolve_storage_dir(next_model_dir.as_deref(), "models").map_err(|err| err.to_string())?;

    let mut migrated_directories = Vec::new();
    let mut removable_directories = Vec::new();
    let data_migrated =
        migrate_directory(&current_data_dir, &resolved_next_data_dir, "数据", false)
            .map_err(|err| err.to_string())?;
    push_migrated_directory(&mut migrated_directories, data_migrated, "数据目录");
    if data_migrated {
        removable_directories.push(current_data_dir.display().to_string());
    }
    let log_migrated = migrate_directory(&current_log_dir, &resolved_next_log_dir, "日志", false)
        .map_err(|err| err.to_string())?;
    push_migrated_directory(&mut migrated_directories, log_migrated, "日志目录");
    if log_migrated {
        removable_directories.push(current_log_dir.display().to_string());
    }
    let model_migrated =
        migrate_directory(&current_model_dir, &resolved_next_model_dir, "模型", true)
            .map_err(|err| err.to_string())?;
    push_migrated_directory(&mut migrated_directories, model_migrated, "模型目录");

    let next_config = EnvConfig {
        basic: BasicConfig {
            mode: persisted_config.basic.mode,
            data_dir: Some(next_data_dir.unwrap_or_default()),
            log_dir: Some(next_log_dir.unwrap_or_default()),
            model_dir: Some(next_model_dir.unwrap_or_default()),
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

    save_configs(&next_config).map_err(|err| err.to_string())?;
    *config = next_config.clone();

    Ok(SettingsPayload {
        restart_required: !migrated_directories.is_empty(),
        migrated_directories,
        removable_directories,
        ..SettingsPayload::from_env_config(&next_config)
    })
}
