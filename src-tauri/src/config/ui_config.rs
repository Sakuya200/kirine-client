use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{config::PARAMS_CONFIG_FILE_NAME, config::discover_model_config_file_paths, Result};

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

impl UiTaskKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Tts => "tts",
            Self::VoiceClone => "voice-clone",
        }
    }
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
    InputAudioFile,
    InputTextFile,
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

pub fn load_ui_configs() -> Result<UiConfigCatalog> {
    let config_paths = discover_model_config_file_paths(PARAMS_CONFIG_FILE_NAME)?;
    load_ui_configs_from_paths(&config_paths)
}

pub fn load_ui_configs_from_dir(config_dir: &Path) -> Result<UiConfigCatalog> {
    if !config_dir.exists() {
        bail!("UI 配置目录不存在: {}", config_dir.display());
    }

    let config_path = config_dir.join(PARAMS_CONFIG_FILE_NAME);
    if !config_path.is_file() {
        bail!(
            "UI 配置目录中未找到 {}: {}",
            PARAMS_CONFIG_FILE_NAME,
            config_dir.display()
        );
    }

    let mut task_configs = Vec::new();
    let mut seen_config_keys = HashSet::new();
    let file_content = fs::read_to_string(&config_path)
        .with_context(|| format!("读取 UI 配置文件失败: {}", config_path.display()))?;
    let file_task_configs = serde_json::from_str::<Vec<TaskParamConfig>>(&file_content)
        .with_context(|| format!("解析 UI 配置文件失败: {}", config_path.display()))?;
    for task_config in file_task_configs {
        let dedup_key = format!(
            "{}:{}",
            task_config.base_model.trim(),
            task_config.task.as_str()
        );
        if !seen_config_keys.insert(dedup_key.clone()) {
            tracing::warn!(
                key = %dedup_key,
                source = %config_path.display(),
                "检测到重复任务参数配置，已跳过"
            );
            continue;
        }

        task_configs.push(task_config);
    }

    Ok(UiConfigCatalog::from_task_configs(task_configs))
}

fn load_ui_configs_from_paths(config_paths: &[PathBuf]) -> Result<UiConfigCatalog> {
    if config_paths.is_empty() {
        bail!(
            "未发现任何 UI 参数配置文件（{}）",
            PARAMS_CONFIG_FILE_NAME
        );
    }

    let mut task_configs = Vec::new();
    let mut seen_config_keys = HashSet::new();

    for config_path in config_paths {
        let file_content = fs::read_to_string(config_path)
            .with_context(|| format!("读取 UI 配置文件失败: {}", config_path.display()))?;
        let file_task_configs = serde_json::from_str::<Vec<TaskParamConfig>>(&file_content)
            .with_context(|| format!("解析 UI 配置文件失败: {}", config_path.display()))?;

        for task_config in file_task_configs {
            let dedup_key = format!(
                "{}:{}",
                task_config.base_model.trim(),
                task_config.task.as_str()
            );
            if !seen_config_keys.insert(dedup_key.clone()) {
                tracing::warn!(
                    key = %dedup_key,
                    source = %config_path.display(),
                    "检测到重复任务参数配置，已跳过"
                );
                continue;
            }

            task_configs.push(task_config);
        }
    }

    if task_configs.is_empty() {
        bail!(
            "已发现参数配置文件，但未加载到可用任务参数配置（{}）",
            PARAMS_CONFIG_FILE_NAME
        );
    }

    Ok(UiConfigCatalog::from_task_configs(task_configs))
}
