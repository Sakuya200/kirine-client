use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

mod env_config;
mod log;

pub use env_config::{
    application_dir, load_configs, resolve_storage_dir, save_configs, BasicConfig, EnvConfig,
    RemoteConfig,
};
pub use log::{init_log, resolve_base_log_dir};

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageMode {
    #[default]
    Local,
    Remote,
}

pub type BaseModel = String;

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HardwareType {
    Cpu,
    #[default]
    Cuda,
}

impl HardwareType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
        }
    }
}

impl fmt::Display for HardwareType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HardwareType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "cpu" => Ok(Self::Cpu),
            "cuda" | "cuda:0" => Ok(Self::Cuda),
            other => Err(format!("不支持的硬件类型: {}", other)),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttentionImplementation {
    #[default]
    Sdpa,
    FlashAttention2,
    Eager,
}

impl AttentionImplementation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sdpa => "sdpa",
            Self::FlashAttention2 => "flash_attention_2",
            Self::Eager => "eager",
        }
    }
}

impl fmt::Display for AttentionImplementation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for AttentionImplementation {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "sdpa" => Ok(Self::Sdpa),
            "flash_attention_2" => Ok(Self::FlashAttention2),
            "eager" => Ok(Self::Eager),
            other => Err(format!("不支持的注意力实现类型: {}", other)),
        }
    }
}
