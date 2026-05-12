use kirine_client_lib::{
    load_ui_configs_from_dir, TaskParamConfig, UiComponentType, UiParamType, UiTaskKind,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct TempUiConfigDir {
    path: PathBuf,
}

impl TempUiConfigDir {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kirine-client-ui-config-test-{timestamp}"));
        fs::create_dir_all(&path).expect("failed to create temp ui config dir");
        Self { path }
    }

    fn write_config_file(&self, file_name: &str, content: &str) {
        fs::write(self.path.join(file_name), content).expect("failed to write temp ui config file");
    }
}

impl Drop for TempUiConfigDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn deserializes_component_props_variants() {
    let payload = r#"
    [
      {
        "task": "voice-clone",
        "base-model": "vox_cpm2",
        "params": [
          {
            "name": "mode",
            "type": "string",
            "componentType": "select",
            "componentProps": {
              "label": "克隆模式",
              "options": [
                { "label": "参考音频克隆", "value": "reference" },
                { "label": "Ultimate 克隆", "value": "ultimate" }
              ],
              "visibleWhen": { "field": "useLora", "equals": true },
              "nullable": false
            },
            "required": true,
            "defaultValue": "reference",
            "description": "mode"
          }
        ]
      }
    ]
    "#;

    let configs = serde_json::from_str::<Vec<TaskParamConfig>>(payload).expect("failed to deserialize inline ui config payload");
    let param = &configs[0].params[0];

    assert_eq!(configs[0].task, UiTaskKind::VoiceClone);
    assert_eq!(param.param_type, UiParamType::String);
    assert_eq!(param.component_type, UiComponentType::Select);
    assert_eq!(param.component_props.options.len(), 2);
    assert_eq!(param.component_props.visible_when.as_ref().expect("missing visibleWhen rule").field, "useLora");
    assert_eq!(param.component_props.nullable, Some(false));
}

#[test]
fn loads_real_ui_config_files() {
    let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("src-model")
        .join("configs");

    let catalog = load_ui_configs_from_dir(&config_dir).expect("failed to load real ui config files");

    assert!(!catalog.task_configs.is_empty());
    assert!(catalog
        .task_configs
        .iter()
        .any(|item| item.base_model == "qwen3_tts" && item.task == UiTaskKind::Training));
    assert!(catalog
        .task_configs
        .iter()
        .any(|item| item.base_model == "vox_cpm2" && item.task == UiTaskKind::Tts));
    assert!(catalog
        .task_configs
        .iter()
        .any(|item| item.base_model == "moss_tts_local" && item.task == UiTaskKind::VoiceClone));
}

#[test]
fn loads_params_files_from_temp_directory() {
    let temp_dir = TempUiConfigDir::new();
    temp_dir.write_config_file(
        "params-sample.json",
        r#"
        [
          {
            "task": "tts",
            "base-model": "qwen3_tts",
            "params": [
              {
                "name": "voicePrompt",
                "type": "string",
                "componentType": "textarea",
                "componentProps": {
                  "label": "音色提示词",
                  "rows": 3,
                  "placeholder": "输入提示词"
                },
                "required": false,
                "defaultValue": "",
                "description": "prompt"
              }
            ]
          }
        ]
        "#,
    );

    let catalog = load_ui_configs_from_dir(&temp_dir.path).expect("failed to load temp ui config files");

    assert_eq!(catalog.task_configs.len(), 1);
    assert_eq!(catalog.task_configs[0].task, UiTaskKind::Tts);
    assert_eq!(catalog.task_configs[0].params[0].component_type, UiComponentType::Textarea);
}