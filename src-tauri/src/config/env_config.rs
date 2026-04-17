use anyhow::Context;
use config::Config;
use serde::{Deserialize, Serialize};
use std::{
    env::{current_dir, current_exe},
    fs, io,
    path::{Path, PathBuf},
};
use tracing::{error, info};

use crate::{
    config::{
        resolve_base_log_dir, AttentionImplementation, HardwareType, LoraMode,
        StorageMode,
    },
    Result,
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub struct EnvConfig {
    #[serde(default)]
    pub basic: BasicConfig,
    #[serde(default)]
    pub remote: Option<RemoteConfig>,
    #[serde(default)]
    pub training: TrainingConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct BasicConfig {
    pub mode: StorageMode,
    pub data_dir: Option<String>,
    pub log_dir: Option<String>,
    pub model_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case", default)]
pub struct RemoteConfig {
    pub api_url: Option<String>,
    pub api_token: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case", default)]
pub struct TrainingConfig {
    pub prepared_base_models: Vec<String>,
    pub hardware_type: HardwareType,
    pub attn_implementation: AttentionImplementation,
    pub lora_mode: LoraMode,
    pub lora_rank: u32,
    pub lora_alpha: u32,
    pub lora_dropout: String,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            prepared_base_models: Vec::new(),
            hardware_type: HardwareType::default(),
            attn_implementation: AttentionImplementation::default(),
            lora_mode: LoraMode::default(),
            lora_rank: 16,
            lora_alpha: 32,
            lora_dropout: "0.05".to_string(),
        }
    }
}

impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            api_url: Some(String::new()),
            api_token: Some(String::new()),
        }
    }
}

impl TrainingConfig {
    pub fn with_hardware_type(mut self, hardware_type: HardwareType) -> Self {
        self.hardware_type = hardware_type;
        self
    }

    pub fn with_attn_implementation(
        mut self,
        attn_implementation: AttentionImplementation,
    ) -> Self {
        self.attn_implementation = attn_implementation;
        self
    }

    pub fn with_lora_settings(
        mut self,
        lora_mode: LoraMode,
        lora_rank: u32,
        lora_alpha: u32,
        lora_dropout: String,
    ) -> Self {
        self.lora_mode = lora_mode;
        self.lora_rank = lora_rank;
        self.lora_alpha = lora_alpha;
        self.lora_dropout = lora_dropout;
        self
    }
}

impl EnvConfig {
    pub fn mode(&self) -> StorageMode {
        self.basic.mode
    }

    pub fn data_dir(&self) -> Option<&str> {
        self.basic.data_dir.as_deref()
    }

    pub fn log_dir(&self) -> Option<&str> {
        self.basic.log_dir.as_deref()
    }

    pub fn model_dir(&self) -> Option<&str> {
        self.basic.model_dir.as_deref()
    }

    pub fn api_url(&self) -> Option<&str> {
        self.remote
            .as_ref()
            .and_then(|remote| remote.api_url.as_deref())
    }

    pub fn api_token(&self) -> Option<&str> {
        self.remote
            .as_ref()
            .and_then(|remote| remote.api_token.as_deref())
    }

    pub fn prepared_base_models(&self) -> &[String] {
        &self.training.prepared_base_models
    }

    pub fn attn_implementation(&self) -> AttentionImplementation {
        self.training.attn_implementation
    }

    pub fn hardware_type(&self) -> HardwareType {
        self.training.hardware_type
    }

    pub fn lora_mode(&self) -> LoraMode {
        self.training.lora_mode
    }

    pub fn lora_rank(&self) -> u32 {
        self.training.lora_rank
    }

    pub fn lora_alpha(&self) -> u32 {
        self.training.lora_alpha
    }

    pub fn lora_dropout(&self) -> &str {
        &self.training.lora_dropout
    }
}

pub fn application_dir() -> Result<PathBuf> {
    let executable_path = current_exe().context("无法解析当前程序路径")?;
    let executable_dir = executable_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("无法解析程序所在目录: {}", executable_path.display()),
        )
    })?;

    Ok(executable_dir.to_path_buf())
}

pub fn resolve_storage_dir(
    configured_path: Option<&str>,
    default_directory_name: &str,
) -> Result<PathBuf> {
    if let Some(path) = configured_path
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        return Ok(PathBuf::from(path));
    }

    Ok(application_dir()?.join(default_directory_name))
}

pub fn config_path() -> Result<std::path::PathBuf> {
    let current_path = current_dir().context("无法获取当前工作目录")?;
    let primary_path = current_path.join("config.toml");
    if primary_path.exists() {
        println!(
            "[startup] 找到项目根目录下的配置文件: {}",
            primary_path.display()
        );
        return Ok(primary_path);
    }

    let fallback_path = current_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".."))
        .join("config.toml");
    if fallback_path.exists() {
        println!(
            "[startup] 当前目录下未找到 config.toml，已回退到上一级目录配置文件: {}",
            fallback_path.display()
        );
        return Ok(fallback_path);
    }

    eprintln!(
        "[startup] 未找到配置文件: config.toml, 当前目录: {}",
        current_path.display()
    );
    Err(io::Error::new(io::ErrorKind::NotFound, "未找到配置文件: config.toml").into())
}

pub fn load_configs() -> Result<EnvConfig> {
    let config_path = config_path().context("解析配置文件路径失败")?;
    load_configs_from_path(&config_path)
}

fn load_configs_from_path(config_path: &Path) -> Result<EnvConfig> {
    println!("[startup] 开始加载配置文件信息");

    let mut builder = Config::builder();
    let config_name = config_path.to_str().ok_or_else(|| {
        let error = io::Error::new(
            io::ErrorKind::InvalidInput,
            "配置文件路径包含非法 UTF-8 字符",
        );
        eprintln!(
            "[startup] 配置文件路径无效: path={}, error={}",
            config_path.display(),
            error
        );
        error
    })?;
    builder = builder.add_source(config::File::new(config_name, config::FileFormat::Toml));
    let settings = builder
        .build()
        .map_err(|err| {
            eprintln!("[startup] 配置文件加载失败: {err}");
            err
        })
        .with_context(|| format!("从 {} 加载配置文件失败", config_path.display()))?;

    let env_config = settings
        .try_deserialize::<EnvConfig>()
        .map_err(|err| {
            eprintln!("[startup] 配置文件解析失败: {err:?}");
            err
        })
        .with_context(|| format!("将 {} 解析为 EnvConfig 失败", config_path.display()))?;
    let mut normalized_config = env_config;
    materialize_config_defaults(&mut normalized_config)?;
    save_configs_to_path(&normalized_config, config_path)?;
    println!("[startup] 配置文件加载完成: {:?}", normalized_config);

    Ok(normalized_config)
}

pub fn save_configs(env_config: &EnvConfig) -> Result<()> {
    let config_path = config_path()
        .map_err(|err| {
            error!(error = %err, "配置文件路径获取失败");
            err
        })
        .context("配置文件保存前无法解析配置路径")?;
    save_configs_to_path(env_config, &config_path)
}

fn save_configs_to_path(env_config: &EnvConfig, config_path: &Path) -> Result<()> {
    let content = toml::to_string_pretty(env_config)
        .map_err(|err| {
            error!(error = %err, "配置文件序列化失败");
            err
        })
        .context("将 EnvConfig 序列化为 TOML 失败")?;

    fs::write(&config_path, content)
        .map_err(|err| {
            error!(path = %config_path.display(), error = %err, "配置文件写入失败");
            err
        })
        .with_context(|| format!("写入配置文件失败: {}", config_path.display()))?;

    info!("配置文件保存完成: {}", config_path.display());
    Ok(())
}

fn materialize_config_defaults(config: &mut EnvConfig) -> Result<()> {
    let resolved_data_dir = resolve_storage_dir(config.data_dir(), "data")?
        .to_string_lossy()
        .to_string();
    config.basic.data_dir = Some(resolved_data_dir);

    let resolved_log_dir = resolve_base_log_dir(config.log_dir())?
        .to_string_lossy()
        .to_string();
    config.basic.log_dir = Some(resolved_log_dir);

    let resolved_model_dir = resolve_storage_dir(config.model_dir(), "models")?
        .to_string_lossy()
        .to_string();
    config.basic.model_dir = Some(resolved_model_dir);

    let remote_config = config.remote.get_or_insert_with(RemoteConfig::default);
    remote_config.api_url = Some(
        remote_config
            .api_url
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
    );

    remote_config.api_token = Some(
        remote_config
            .api_token
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
    );

    Ok(())
}
