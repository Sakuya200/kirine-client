use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Result;

use super::env_config::supported_models_path;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UiConfigCatalog {
    pub task_configs: Vec<TaskParamConfig>,
}

impl UiConfigCatalog {
    pub fn from_task_configs(task_configs: Vec<TaskParamConfig>) -> Self {
        Self { task_configs }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskParamConfig {
    pub task: UiTaskKind,
    #[serde(rename = "base-model")]
    pub base_model: String,
    pub params: Vec<ParamDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UiTaskKind {
    Training,
    Tts,
    VoiceClone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UiParamType {
    Number,
    String,
    Boolean,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UiComponentType {
    InputNumber,
    InputText,
    Textarea,
    Select,
    Switch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ParamDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: UiParamType,
    #[serde(rename = "componentType")]
    pub component_type: UiComponentType,
    #[serde(rename = "componentProps", default)]
    pub component_props: ComponentProps,
    pub required: bool,
    #[serde(rename = "defaultValue", default)]
    pub default_value: Value,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentProps {
    pub label: Option<String>,
    pub text: Option<String>,
    pub text_on: Option<String>,
    pub text_off: Option<String>,
    pub rows: Option<u32>,
    pub placeholder: Option<String>,
    pub help_text: Option<String>,
    pub min: Option<Value>,
    pub max: Option<Value>,
    pub step: Option<Value>,
    pub nullable: Option<bool>,
    pub input_mode: Option<String>,
    pub visible_when: Option<VisibleWhenRule>,
    #[serde(default)]
    pub options: Vec<SelectOption>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VisibleWhenRule {
    pub field: String,
    pub equals: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub label: String,
    pub value: Value,
}

pub fn ui_configs_dir_path() -> Result<PathBuf> {
    let supported_models =
        supported_models_path().context("解析 supported_models.json 路径失败")?;
    let root_dir = supported_models.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("无法解析项目根目录: {}", supported_models.display()),
        )
    })?;

    Ok(root_dir.join("src-model").join("configs"))
}

pub fn load_ui_configs() -> Result<UiConfigCatalog> {
    let config_dir = ui_configs_dir_path().context("解析 UI 配置目录失败")?;
    load_ui_configs_from_dir(&config_dir)
}

pub fn load_ui_configs_from_dir(config_dir: &Path) -> Result<UiConfigCatalog> {
    if !config_dir.exists() {
        bail!("UI 配置目录不存在: {}", config_dir.display());
    }

    let mut config_paths: Vec<PathBuf> = fs::read_dir(config_dir)
        .with_context(|| format!("读取 UI 配置目录失败: {}", config_dir.display()))?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("params-") && name.ends_with(".json"))
                    .unwrap_or(false)
        })
        .collect();

    config_paths.sort();

    if config_paths.is_empty() {
        bail!(
            "UI 配置目录中未找到 params-*.json 文件: {}",
            config_dir.display()
        );
    }

    let mut task_configs = Vec::new();
    for config_path in config_paths {
        let file_content = fs::read_to_string(&config_path)
            .with_context(|| format!("读取 UI 配置文件失败: {}", config_path.display()))?;
        let mut file_task_configs = serde_json::from_str::<Vec<TaskParamConfig>>(&file_content)
            .with_context(|| format!("解析 UI 配置文件失败: {}", config_path.display()))?;
        task_configs.append(&mut file_task_configs);
    }

    Ok(UiConfigCatalog::from_task_configs(task_configs))
}
