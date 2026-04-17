use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{
    load_configs, resolve_base_log_dir, resolve_storage_dir, save_configs, AttentionImplementation,
    BasicConfig, EnvConfig, HardwareType, LoraMode, RemoteConfig,
};
use crate::service::pipeline::{
    qwen3_tts::QWEN3_TTS_BASE_MODEL,
    script_paths::{resolve_src_model_root, src_model_lora_toggle_script_path, ScriptPlatform},
};
use crate::utils::file_ops::migrate_directory;
use crate::utils::process::run_logged_command;

pub struct EnvConfigState(pub RwLock<EnvConfig>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPayload {
    pub api_url: String,
    pub api_token: String,
    pub model_dir: String,
    pub data_dir: String,
    pub log_cache_dir: String,
    pub hardware_type: String,
    pub attn_implementation: String,
    pub lora_mode: String,
    pub lora_rank: u32,
    pub lora_alpha: u32,
    pub lora_dropout: String,
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
    pub hardware_type: String,
    pub attn_implementation: String,
    pub lora_mode: String,
    pub lora_rank: u32,
    pub lora_alpha: u32,
    pub lora_dropout: String,
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
            hardware_type: config.hardware_type().as_str().to_string(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
            lora_mode: LoraMode::Disabled.as_str().to_string(),
            lora_rank: config.lora_rank(),
            lora_alpha: config.lora_alpha(),
            lora_dropout: config.lora_dropout().to_string(),
            restart_required: false,
            migrated_directories: Vec::new(),
            removable_directories: Vec::new(),
        }
    }
}

fn sync_lora_runtime_dependencies(
    config: &EnvConfig,
    enabled: bool,
) -> std::result::Result<(), String> {
    let app_dir = std::env::current_dir().map_err(|err| format!("解析应用目录失败: {err}"))?;
    let src_model_root = resolve_src_model_root(&app_dir).map_err(|err| err.to_string())?;
    let script_path = src_model_lora_toggle_script_path(&src_model_root);
    let log_dir = resolve_base_log_dir(config.log_dir()).map_err(|err| err.to_string())?;
    let task_log_path = log_dir.join("settings").join("lora_dependency_sync.log");
    let platform = ScriptPlatform::current();
    let mut args = platform.shell_args(&script_path);
    args.push("--base-model".to_string());
    args.push(QWEN3_TTS_BASE_MODEL.to_string());
    args.push("--task-log-file".to_string());
    args.push(task_log_path.to_string_lossy().to_string());
    args.push("--mode".to_string());
    args.push(if enabled {
        "enable".to_string()
    } else {
        "disable".to_string()
    });

    tauri::async_runtime::block_on(async {
        run_logged_command(
            std::path::Path::new(platform.shell_program()),
            &args,
            &src_model_root,
            "sync lora dependencies",
            &task_log_path,
            if enabled {
                "LoRA dependencies enabled"
            } else {
                "LoRA dependencies disabled"
            },
        )
        .await
    })
    .map_err(|err| {
        format!(
            "同步 LoRA 依赖失败，请查看日志 {}: {}",
            task_log_path.display(),
            err
        )
    })
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
    let hardware_type = payload
        .hardware_type
        .parse::<HardwareType>()
        .map_err(|err| err.to_string())?;
    let _requested_lora_mode = payload
        .lora_mode
        .parse::<LoraMode>()
        .map_err(|err| err.to_string())?;
    let effective_lora_rank = payload.lora_rank.max(1);
    let effective_lora_alpha = payload.lora_alpha.max(1);
    let effective_lora_dropout = if payload.lora_dropout.trim().is_empty() {
        "0.05".to_string()
    } else {
        payload.lora_dropout.trim().to_string()
    };
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
            data_dir: Some(resolved_next_data_dir.to_string_lossy().to_string()),
            log_dir: Some(resolved_next_log_dir.to_string_lossy().to_string()),
            model_dir: Some(resolved_next_model_dir.to_string_lossy().to_string()),
        },
        remote: Some(RemoteConfig {
            api_url: Some(payload.api_url.trim().to_string()),
            api_token: Some(payload.api_token.trim().to_string()),
        }),
        training: persisted_config
            .training
            .clone()
            .with_hardware_type(hardware_type)
            .with_attn_implementation(attn_implementation)
            .with_lora_settings(
                LoraMode::Disabled,
                effective_lora_rank,
                effective_lora_alpha,
                effective_lora_dropout,
            ),
    };

    sync_lora_runtime_dependencies(&next_config, false)?;

    save_configs(&next_config).map_err(|err| err.to_string())?;
    *config = next_config.clone();

    Ok(SettingsPayload {
        restart_required: !migrated_directories.is_empty(),
        migrated_directories,
        removable_directories,
        ..SettingsPayload::from_env_config(&next_config)
    })
}
