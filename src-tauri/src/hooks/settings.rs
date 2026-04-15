use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::config::{
    load_configs, resolve_base_log_dir, resolve_storage_dir, save_configs, AttentionImplementation,
    BasicConfig, EnvConfig, HardwareType, QloraMode, QloraQuantType, RemoteConfig,
};
use crate::service::pipeline::script_paths::{
    resolve_src_model_root, src_model_qlora_toggle_script_path, ScriptPlatform,
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
    pub qlora_mode: String,
    pub qlora_rank: u32,
    pub qlora_alpha: u32,
    pub qlora_dropout: String,
    pub qlora_quant_type: String,
    pub qlora_double_quant: bool,
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
    pub qlora_mode: String,
    pub qlora_rank: u32,
    pub qlora_alpha: u32,
    pub qlora_dropout: String,
    pub qlora_quant_type: String,
    pub qlora_double_quant: bool,
}

impl SettingsPayload {
    fn from_env_config(config: &EnvConfig) -> Self {
        Self {
            api_url: config.api_url().unwrap_or_default().to_string(),
            api_token: config.api_token().unwrap_or_default().to_string(),
            model_dir: config.model_dir().unwrap_or_default().to_string(),
            data_dir: config.data_dir().unwrap_or_default().to_string(),
            log_cache_dir: config.log_dir().unwrap_or_default().to_string(),
            hardware_type: config.hardware_type().as_str().to_string(),
            attn_implementation: config.attn_implementation().as_str().to_string(),
            qlora_mode: config.qlora_mode().as_str().to_string(),
            qlora_rank: config.qlora_rank(),
            qlora_alpha: config.qlora_alpha(),
            qlora_dropout: config.qlora_dropout().to_string(),
            qlora_quant_type: config.qlora_quant_type().as_str().to_string(),
            qlora_double_quant: config.qlora_double_quant(),
            restart_required: false,
            migrated_directories: Vec::new(),
            removable_directories: Vec::new(),
        }
    }
}

fn sync_qlora_runtime_dependencies(
    config: &EnvConfig,
    enabled: bool,
) -> std::result::Result<(), String> {
    let app_dir = std::env::current_dir().map_err(|err| format!("解析应用目录失败: {err}"))?;
    let src_model_root = resolve_src_model_root(&app_dir).map_err(|err| err.to_string())?;
    let script_path = src_model_qlora_toggle_script_path(&src_model_root);
    let log_dir = resolve_base_log_dir(config.log_dir()).map_err(|err| err.to_string())?;
    let task_log_path = log_dir.join("settings").join("qlora_dependency_sync.log");
    let platform = ScriptPlatform::current();
    let mut args = platform.shell_args(&script_path);
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
            "sync qlora dependencies",
            &task_log_path,
            if enabled {
                "QLoRA dependencies enabled"
            } else {
                "QLoRA dependencies disabled"
            },
        )
        .await
    })
    .map_err(|err| {
        format!(
            "同步 QLoRA 依赖失败，请查看日志 {}: {}",
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
    let qlora_mode = payload
        .qlora_mode
        .parse::<QloraMode>()
        .map_err(|err| err.to_string())?;
    let qlora_quant_type = payload
        .qlora_quant_type
        .parse::<QloraQuantType>()
        .map_err(|err| err.to_string())?;
    if payload.qlora_rank == 0 {
        return Err("QLoRA Rank 必须大于 0".to_string());
    }
    if payload.qlora_alpha == 0 {
        return Err("QLoRA Alpha 必须大于 0".to_string());
    }
    if payload.qlora_dropout.trim().is_empty() {
        return Err("QLoRA Dropout 不能为空".to_string());
    }
    if matches!(hardware_type, HardwareType::Cpu) && matches!(qlora_mode, QloraMode::Enabled) {
        return Err("CPU 模式下不能启用 QLoRA，请先切换到 CUDA 硬件类型。".to_string());
    }
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
            .with_hardware_type(hardware_type)
            .with_attn_implementation(attn_implementation)
            .with_qlora_settings(
                qlora_mode,
                payload.qlora_rank,
                payload.qlora_alpha,
                payload.qlora_dropout.trim().to_string(),
                qlora_quant_type,
                payload.qlora_double_quant,
            ),
    };

    sync_qlora_runtime_dependencies(&next_config, matches!(qlora_mode, QloraMode::Enabled))?;

    save_configs(&next_config).map_err(|err| err.to_string())?;
    *config = next_config.clone();

    Ok(SettingsPayload {
        restart_required: !migrated_directories.is_empty(),
        migrated_directories,
        removable_directories,
        ..SettingsPayload::from_env_config(&next_config)
    })
}
