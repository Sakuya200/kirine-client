use std::{fs, io, path::PathBuf};

use rand::random;
use sqlx::{sqlite::SqlitePoolOptions, Row};

use crate::service::models::{
    CreateSpeakerPayload, HistoryRecord, HistoryRecordSummary, HistoryTaskType, ModelInfo,
    SpeakerInfo, SpeakerPageResult, SpeakerSource, TaskStatus, UpdateSpeakerPayload,
};
use crate::Result;

// `service` / `config` 模块在 lib.rs 中为私有，集成测试（外部 crate）无法直接命名其中的
// 类型与纯函数。`test_support` 作为对外的公共测试桥接模块，统一重导出测试所需的类型与
// 抽取出的纯函数 `build_llm_task_script_args`（规则4 参数契约验证）。
pub use crate::config::HardwareType;
pub use crate::hooks::streaming::{
    build_sine_wave_stream_events, generate_sine_wave_wav, AudioStreamEvent,
};
pub use crate::service::models;
pub use crate::service::pipeline::build_llm_task_script_args;
pub use crate::service::pipeline::api::{
    PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
    PythonScriptTaskKind, StreamingArgs, StreamingSpeakerArg, TTSArgs, TrainingArgs,
    VoiceCloneArgs, VoiceDesignArgs,
};
pub use crate::service::pipeline::streaming::{
    frame_to_event, parse_streaming_frame, serialize_input_entry, StreamingContextBasic,
    StreamingContextJson, StreamingFrame, StreamingFramePayload, StreamingMessageEntry,
    StreamingSpeaker,
};
pub use crate::service::{LocalService, Service};
pub use crate::service::models::{PageRequest, SpeakerFilter, SpeakerStatus};

/// 临时库测试设施：每个 `LocalServiceHarness::new` 在 `std::env::temp_dir` 下创建独立
/// 临时目录，经 `LocalService::from_paths` 走「建库 → 全量迁移 → sync 模型目录」链路，
/// 所有 DB 操作落到该临时库（规则1&2）。`shutdown` 关闭连接并清理临时目录。
pub struct LocalServiceHarness {
    root_dir: PathBuf,
    data_dir: PathBuf,
    model_dir: PathBuf,
    service: LocalService,
}

impl LocalServiceHarness {
    pub async fn new(label: &str) -> Result<Self> {
        let root_dir = test_root(label);
        let data_dir = root_dir.join("data");
        let model_dir = root_dir.join("models");
        let service =
            LocalService::from_paths(root_dir.clone(), data_dir.clone(), model_dir.clone())
                .await?;

        Ok(Self {
            root_dir,
            data_dir,
            model_dir,
            service,
        })
    }

    /// 直接访问 `LocalService`，测试可调用任意 `Service` trait 方法（需 `use ...Service`）。
    pub fn service(&self) -> &LocalService {
        &self.service
    }

    pub fn app_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn model_dir(&self) -> &PathBuf {
        &self.model_dir
    }

    pub fn database_file_exists(&self) -> bool {
        self.data_dir.join("app.db").exists()
    }

    pub async fn speakers_query_succeeds(&self) -> Result<bool> {
        self.service.list_speaker_infos(PageRequest::default()).await?;
        Ok(true)
    }

    pub async fn create_test_speaker(&self) -> Result<SpeakerInfo> {
        self.service
            .create_speaker_info(CreateSpeakerPayload {
                speaker_name: "SeaOrm Speaker".to_string(),
                samples: 3,
                base_model: "qwen3_tts".to_string(),
                description: "created by test".to_string(),
                status: SpeakerStatus::Ready,
                source: SpeakerSource::Local,
            })
            .await
    }

    pub async fn list_speakers(&self) -> Result<Vec<SpeakerInfo>> {
        self.service
            .list_speaker_infos(PageRequest {
                page: 1,
                page_size: 1000,
                filter: None,
            })
            .await
            .map(|result| result.items)
    }

    /// 以分页请求获取说话人，返回完整 `SpeakerPageResult`（含统计），
    /// 用于覆盖分页 / 筛选 / 统计逻辑。
    pub async fn list_speakers_paged(
        &self,
        request: PageRequest<SpeakerFilter>,
    ) -> Result<SpeakerPageResult> {
        self.service.list_speaker_infos(request).await
    }

    pub async fn list_model_infos(&self) -> Result<Vec<ModelInfo>> {
        self.service
            .list_model_infos(PageRequest {
                page: 1,
                page_size: 1000,
                filter: None,
            })
            .await
            .map(|result| result.items)
    }

    pub async fn list_history_records(&self) -> Result<Vec<HistoryRecordSummary>> {
        self.service
            .list_history_records(PageRequest {
                page: 1,
                page_size: 1000,
                filter: None,
            })
            .await
            .map(|result| result.items)
    }

    pub async fn get_history_record(&self, history_id: i64) -> Result<HistoryRecord> {
        self.service.get_history_record(history_id).await
    }

    /// 读取 `task_history.status`（直接走原生 SQL，不加载 detail）。
    /// 用于断言清扫/状态机等仅关心 status 的场景，避免对未种子化 detail 行的依赖。
    pub async fn history_status(&self, history_id: i64) -> Result<TaskStatus> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let row = sqlx::query("SELECT status FROM task_history WHERE id = ? AND deleted = 0")
            .bind(history_id)
            .fetch_optional(&pool)
            .await?;
        pool.close().await;
        let row = row.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("history row {history_id} not found"))
        })?;
        let status: String = row.get("status");
        status
            .parse::<TaskStatus>()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e).into())
    }

    /// 触发流式会话启动清扫（调用真实 `LocalService::sweep_stale_streaming_sessions_impl`）。
    pub async fn sweep_stale_streaming_sessions(&self) -> Result<()> {
        self.service.sweep_stale_streaming_sessions_impl().await
    }

    pub fn src_model_root(&self) -> PathBuf {
        self.root_dir.join("src-model")
    }

    pub fn ensure_src_model_root(&self) -> Result<PathBuf> {
        let path = self.src_model_root();
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub async fn update_test_speaker(&self, id: i64) -> Result<SpeakerInfo> {
        self.service
            .update_speaker_info(UpdateSpeakerPayload {
                id,
                speaker_name: "Updated Speaker".to_string(),
                description: "updated by test".to_string(),
            })
            .await
    }

    pub async fn delete_speaker(&self, id: i64) -> Result<bool> {
        self.service.delete_speaker_info(id).await
    }

    // ---- 模型下载状态（覆盖 install/uninstall 的 DB 侧，不真正执行脚本） ----

    pub async fn model_downloaded(
        &self,
        base_model: &str,
        model_version: &str,
    ) -> Result<bool> {
        self.service
            .model_downloaded_impl(base_model, model_version)
            .await
    }

    pub async fn set_model_downloaded(
        &self,
        base_model: &str,
        model_version: &str,
        downloaded: bool,
    ) -> Result<()> {
        self.service
            .set_model_downloaded_impl(base_model, model_version, downloaded)
            .await
    }

    // ---- 历史记录种子（直接 SQL 插入，避免触发 create_*_task 的后台 pipeline） ----

    const SEED_NOW: &'static str = "2026-06-27 10:00:00";

    /// 插入一条 `task_history` 行。`task_type`/`status` 以 `as_str()`（kebab / lowercase）落库，
    /// 与生产写入格式一致。`finished_time` 在终态时置为当前种子时间。
    pub async fn seed_history(
        &self,
        id: i64,
        task_type: HistoryTaskType,
        title: &str,
        status: TaskStatus,
    ) -> Result<()> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let finished = if status.is_finished() {
            Some(Self::SEED_NOW)
        } else {
            None
        };
        sqlx::query(
            r#"
            INSERT INTO task_history (
                id, task_type, title, speaker_id, speaker_name_snapshot, status,
                duration_seconds, create_time, modify_time, finished_time, device, deleted
            ) VALUES (?, ?, ?, NULL, '-', ?, 0, ?, ?, ?, 'cpu', 0)
            "#,
        )
        .bind(id)
        .bind(task_type.as_str())
        .bind(title)
        .bind(status.as_str())
        .bind(Self::SEED_NOW)
        .bind(Self::SEED_NOW)
        .bind(finished)
        .execute(&pool)
        .await?;
        pool.close().await;
        Ok(())
    }

    pub async fn seed_tts_detail(
        &self,
        history_id: i64,
        base_model: &str,
        model_version: &str,
        output_file_path: Option<&str>,
    ) -> Result<()> {
        self.seed_detail(
            "tts_tasks",
            history_id,
            &[
                ("speaker_id", None),
                ("model_path", None),
                ("base_model", Some(base_model)),
                ("model_version", Some(model_version)),
                ("language", Some("chinese")),
                ("format", Some("wav")),
                ("export_audio_name", Some("tts-export")),
                ("text", Some("你好")),
                ("model_params_json", Some("{}")),
                ("file_name", Some("tts.wav")),
                ("output_file_path", output_file_path),
            ],
            vec![("char_count", "2")],
        )
        .await
    }

    pub async fn seed_voice_clone_detail(
        &self,
        history_id: i64,
        base_model: &str,
        model_version: &str,
        output_file_path: Option<&str>,
    ) -> Result<()> {
        self.seed_detail(
            "voice_clone_tasks",
            history_id,
            &[
                ("base_model", Some(base_model)),
                ("model_version", Some(model_version)),
                ("language", Some("chinese")),
                ("format", Some("wav")),
                ("export_audio_name", Some("vc-export")),
                ("ref_audio_name", Some("ref.wav")),
                ("ref_audio_path", Some("/tmp/ref.wav")),
                ("ref_text", Some("参考")),
                ("text", Some("生成")),
                ("model_params_json", Some("{}")),
                ("file_name", Some("vc.wav")),
                ("output_file_path", output_file_path),
            ],
            vec![("char_count", "2")],
        )
        .await
    }

    pub async fn seed_voice_design_detail(
        &self,
        history_id: i64,
        base_model: &str,
        model_version: &str,
        output_file_path: Option<&str>,
    ) -> Result<()> {
        self.seed_detail(
            "voice_design_tasks",
            history_id,
            &[
                ("base_model", Some(base_model)),
                ("model_version", Some(model_version)),
                ("language", Some("chinese")),
                ("format", Some("wav")),
                ("export_audio_name", Some("vd-export")),
                ("prompt", Some("温柔")),
                ("text", Some("生成")),
                ("model_params_json", Some("{}")),
                ("file_name", Some("vd.wav")),
                ("output_file_path", output_file_path),
            ],
            vec![("char_count", "2")],
        )
        .await
    }

    pub async fn seed_training_detail(
        &self,
        history_id: i64,
        base_model: &str,
        model_version: &str,
    ) -> Result<()> {
        self.seed_detail(
            "model_training_tasks",
            history_id,
            &[
                ("language", Some("chinese")),
                ("base_model", Some(base_model)),
                ("model_version", Some(model_version)),
                ("speaker_name", Some("训练说话人")),
                ("description", Some("")),
                ("model_params_json", Some("{}")),
                ("samples_json", Some("[]")),
                ("notes_json", Some("[]")),
                ("output_speaker_id", None),
            ],
            vec![("sample_count", "1")],
        )
        .await
    }

    /// 通用 detail 行插入。`string_cols` 以绑定参数写入（`None` → NULL），
    /// `int_cols` 以字符串字面量直接拼入 SQL（种子常量，非用户输入）。
    async fn seed_detail(
        &self,
        table: &str,
        history_id: i64,
        string_cols: &[(&'static str, Option<&str>)],
        int_cols: Vec<(&'static str, &'static str)>,
    ) -> Result<()> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let mut cols: Vec<&str> = vec!["history_id", "create_time", "modify_time", "deleted"];
        cols.extend(string_cols.iter().map(|(name, _)| *name));
        cols.extend(int_cols.iter().map(|(name, _)| *name));
        let mut vals: Vec<String> = vec!["?".into(), "?".into(), "?".into(), "0".into()];
        vals.extend(string_cols.iter().map(|_| "?".to_string()));
        vals.extend(int_cols.iter().map(|(_, raw)| raw.to_string()));
        let sql = format!(
            "INSERT INTO {table} ({}) VALUES ({})",
            cols.join(", "),
            vals.join(", ")
        );
        let mut q = sqlx::query(&sql)
            .bind(history_id)
            .bind(Self::SEED_NOW)
            .bind(Self::SEED_NOW);
        for (_, value) in string_cols {
            q = q.bind(value);
        }
        q.execute(&pool).await?;
        pool.close().await;
        Ok(())
    }

    // ---- DB 内省 helper ----

    pub async fn table_exists(&self, table_name: &str) -> Result<bool> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let row = sqlx::query(
            "SELECT COUNT(1) AS count FROM sqlite_master WHERE type = 'table' AND name = ?",
        )
        .bind(table_name)
        .fetch_one(&pool)
        .await?;
        pool.close().await;

        Ok(row.get::<i64, _>("count") > 0)
    }

    pub async fn table_has_column(&self, table_name: &str, column_name: &str) -> Result<bool> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let pragma = format!("PRAGMA table_info({table_name})");
        let rows = sqlx::query(&pragma).fetch_all(&pool).await?;
        pool.close().await;

        Ok(rows
            .iter()
            .any(|row| row.get::<String, _>("name") == column_name))
    }

    pub async fn task_detail_id_for_history(
        &self,
        table_name: &str,
        history_id: i64,
    ) -> Result<Option<i64>> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let sql = format!(
            "SELECT id FROM {table_name} WHERE history_id = ? AND deleted = 0"
        );
        let row = sqlx::query(&sql)
            .bind(history_id)
            .fetch_optional(&pool)
            .await?;
        pool.close().await;

        Ok(row.map(|row| row.get::<i64, _>("id")))
    }

    /// 读取 detail 行的 `output_file_path`（用于断言任务行写入了预期音频路径）。
    pub async fn detail_output_file_path(
        &self,
        table_name: &str,
        history_id: i64,
    ) -> Result<Option<String>> {
        let pool = open_sqlite_pool(&self.data_dir.join("app.db")).await?;
        let sql = format!(
            "SELECT output_file_path FROM {table_name} WHERE history_id = ? AND deleted = 0"
        );
        let row = sqlx::query(&sql)
            .bind(history_id)
            .fetch_optional(&pool)
            .await?;
        pool.close().await;

        Ok(row.and_then(|row| row.get::<Option<String>, _>("output_file_path")))
    }

    pub async fn shutdown(self) -> Result<()> {
        self.service.close().await?;
        // SQLite 启用 WAL 模式时会产生 -shm/-wal 旁路文件，连接关闭后 Windows 上
        // 可能仍被 OS 短暂占用，立即 remove_dir_all 会偶发 os error 32。重试等待句柄释放；
        // 若最终仍失败则按 best-effort 放行，避免临时文件清理阻断测试断言结果。
        for attempt in 0..15 {
            match fs::remove_dir_all(&self.root_dir) {
                Ok(()) => return Ok(()),
                Err(_) if attempt < 14 => {
                    tokio::time::sleep(std::time::Duration::from_millis(40)).await;
                }
                Err(err) => {
                    log::warn!(
                        "failed to remove test root dir after retries: {}: {}",
                        self.root_dir.display(),
                        err
                    );
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

fn test_root(label: &str) -> PathBuf {
    let unique = random::<u64>();
    std::env::temp_dir().join(format!("kirine-client-{label}-{unique}"))
}

fn sqlite_database_url(db_path: &PathBuf) -> String {
    let normalized = db_path.to_string_lossy().replace('\\', "/");
    format!("sqlite://{}?mode=rwc", normalized)
}

async fn open_sqlite_pool(db_path: &PathBuf) -> Result<sqlx::SqlitePool> {
    let database_url = sqlite_database_url(db_path);
    Ok(SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?)
}
