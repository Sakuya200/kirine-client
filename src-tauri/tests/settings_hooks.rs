//! 覆盖 3 个 settings hook 的底层配置函数：`get_settings_config`/`save_settings_config`
//! 经 `load_configs`/`save_configs`，`get_ui_config` 经 `load_ui_configs`。
//!
//! hooks 是 Tauri `State` 的薄封装，真实逻辑在这些 `pub` 配置函数中。`load_configs`/
//! `save_configs` 以 cwd 下的 `config.toml` 为目标，故用「临时 cwd + 进程级 Mutex 串行化」
//! 隔离，不污染仓库真实 config.toml。
//!
//! 关键：`std::env::set_current_dir` 是进程全局的，而 `load_ui_configs` 的 src-model 发现
//! 也依赖 cwd。因此**所有** settings 测试都获取同一把 `current_dir_lock` 串行执行（含
//! 不改 cwd 的 `load_ui_configs` 测试），并用 poison 恢复避免级联失败。

use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use kirine_client_lib::{
    load_configs, load_ui_configs, load_ui_configs_from_dir, save_configs, EnvConfig, HistoryTaskType,
    StorageMode, TaskParamConfig, UiComponentType, UiParamType,
};

fn current_dir_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// 获取串行锁，并对中毒互斥量做恢复（避免某测试 panic 后级联失败）。
fn serial_lock() -> std::sync::MutexGuard<'static, ()> {
    current_dir_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct TempCwd {
    original: PathBuf,
    path: PathBuf,
}

impl TempCwd {
    /// 调用前必须已持有 `serial_lock()`，保证捕获的 `original` 不被其他测试污染。
    fn new(label: &str) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("kirine-client-settings-{label}-{timestamp}"));
        fs::create_dir_all(&path).expect("failed to create temp cwd");
        let original = std::env::current_dir().expect("failed to capture current dir");
        Self { original, path }
    }

    fn enter(&self) {
        std::env::set_current_dir(&self.path).expect("failed to enter temp cwd");
    }
}

impl Drop for TempCwd {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn load_configs_backfills_missing_sections_and_persists_defaults() {
    let _lock = serial_lock();
    let temp = TempCwd::new("load");

    // 仅写 [training] 段，load_configs 应回填 basic/remote 并持久化
    fs::write(temp.path.join("config.toml"), "[training]\nattn_implementation = \"sdpa\"\n")
        .expect("failed to write config fixture");
    temp.enter();

    let config = load_configs().expect("failed to load config");
    assert_eq!(config.mode(), StorageMode::Local);
    assert!(config.data_dir().is_some());
    assert!(config.log_dir().is_some());
    assert!(config.model_dir().is_some());

    let persisted =
        fs::read_to_string(temp.path.join("config.toml")).expect("failed to read persisted config");
    assert!(persisted.contains("[basic]"), "persisted: {persisted}");
    assert!(persisted.contains("mode = \"local\""));
    assert!(persisted.contains("[remote]"));
}

#[test]
fn save_configs_round_trips_to_config_toml() {
    let _lock = serial_lock();
    let temp = TempCwd::new("save");
    // save_configs 内部会重新解析 config_path()，要求 config.toml 已存在。
    fs::write(temp.path.join("config.toml"), "# stub\n").expect("write stub config");
    temp.enter();

    save_configs(&EnvConfig::default()).expect("failed to save config");

    let persisted =
        fs::read_to_string(temp.path.join("config.toml")).expect("failed to read saved config");
    assert!(persisted.contains("[basic]"));
    assert!(persisted.contains("mode = \"local\""));

    // 再读回应能解析为同样的 mode
    let reloaded = load_configs().expect("failed to reload config");
    assert_eq!(reloaded.mode(), StorageMode::Local);
}

#[test]
fn load_ui_configs_returns_nonempty_catalog_from_real_src_model() {
    // 串行：避免与 load/save_configs 的临时 cwd 冲突（src-model 发现依赖 cwd）。
    let _lock = serial_lock();
    let catalog = load_ui_configs().expect("failed to load real ui config files");
    assert!(!catalog.task_configs.is_empty());

    // 覆盖预期 base_model × task 对（来自 6 个模型子模块的 params-config.json）
    assert!(catalog
        .task_configs
        .iter()
        .any(|c| c.base_model == "qwen3_tts" && c.task == HistoryTaskType::ModelTraining));
    assert!(catalog
        .task_configs
        .iter()
        .any(|c| c.base_model == "vox_cpm2" && c.task == HistoryTaskType::TextToSpeech));
}

#[test]
fn load_ui_configs_from_dir_deserializes_task_param_config() {
    // 纯目录隔离，但仍串行以避免与其他测试的 cwd 变更冲突。
    let _lock = serial_lock();
    let temp = TempCwd::new("uidir");
    fs::write(
        temp.path.join("params-config.json"),
        r#"
        [
          {
            "task": "text-to-speech",
            "base-model": "qwen3_tts",
            "params": [
              {
                "name": "voicePrompt",
                "type": "string",
                "componentType": "textarea",
                "componentProps": { "label": "音色提示词", "rows": 3, "placeholder": "输入提示词" },
                "required": false,
                "defaultValue": "",
                "description": "prompt"
              }
            ]
          }
        ]
        "#,
    )
    .expect("failed to write temp ui config file");

    // load_ui_configs_from_dir 接收显式目录参数，不依赖 cwd；此处不 enter temp。
    let catalog =
        load_ui_configs_from_dir(&temp.path).expect("failed to load temp ui config files");
    assert_eq!(catalog.task_configs.len(), 1);
    let cfg = &catalog.task_configs[0];
    assert_eq!(cfg.task, HistoryTaskType::TextToSpeech);
    assert_eq!(cfg.base_model, "qwen3_tts");
    assert_eq!(cfg.params[0].param_type, UiParamType::String);
    assert_eq!(cfg.params[0].component_type, UiComponentType::Textarea);
}

#[test]
fn task_param_config_deserializes_component_props_variants() {
    let _lock = serial_lock();
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

    let configs = serde_json::from_str::<Vec<TaskParamConfig>>(payload)
        .expect("failed to deserialize inline ui config payload");
    let param = &configs[0].params[0];

    assert_eq!(configs[0].task, HistoryTaskType::VoiceClone);
    assert_eq!(param.param_type, UiParamType::String);
    assert_eq!(param.component_type, UiComponentType::Select);
    assert_eq!(param.component_props.options.len(), 2);
    assert_eq!(
        param
            .component_props
            .visible_when
            .as_ref()
            .expect("missing visibleWhen rule")
            .field,
        "useLora"
    );
    assert_eq!(param.component_props.nullable, Some(false));
}
