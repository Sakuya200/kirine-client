use kirine_client_lib::{load_configs, StorageMode};
use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

fn current_dir_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TempConfigDir {
    path: PathBuf,
}

impl TempConfigDir {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("kirine-client-config-test-{timestamp}"));
        fs::create_dir_all(&path).expect("failed to create temp config dir");
        Self { path }
    }

    fn config_path(&self) -> PathBuf {
        self.path.join("config.toml")
    }
}

impl Drop for TempConfigDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn load_configs_backfills_missing_sections_and_persists_defaults() {
    let _lock = current_dir_lock()
        .lock()
        .expect("failed to lock current dir");
    let original_dir = std::env::current_dir().expect("failed to capture current dir");
    let temp_dir = TempConfigDir::new();
    let config_path = temp_dir.config_path();
    fs::write(&config_path, "[training]\nlora_rank = 24\n")
        .expect("failed to write config fixture");

    std::env::set_current_dir(&temp_dir.path).expect("failed to enter temp config dir");

    let test_result = (|| {
        let config = load_configs().expect("failed to load config");

        assert_eq!(config.mode(), StorageMode::Local);
        assert!(config.data_dir().is_some());
        assert!(config.log_dir().is_some());
        assert!(config.model_dir().is_some());
        assert_eq!(config.lora_rank(), 24);
        assert_eq!(config.api_url(), Some(""));
        assert_eq!(config.api_token(), Some(""));

        let persisted = fs::read_to_string(&config_path).expect("failed to read persisted config");
        assert!(persisted.contains("[basic]"));
        assert!(persisted.contains("mode = \"local\""));
        assert!(persisted.contains("[remote]"));
        assert!(persisted.contains("api_url = \"\""));
        assert!(persisted.contains("api_token = \"\""));
        assert!(persisted.contains("lora_rank = 24"));
    })();

    std::env::set_current_dir(original_dir).expect("failed to restore current dir");
    test_result
}
