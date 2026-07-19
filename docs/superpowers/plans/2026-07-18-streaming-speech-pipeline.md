# 流式语音流水线（Rust 端）实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Rust 端实现长期存活的流式语音会话流水线（DB / 迁移 / entity / service / pipeline 进程管理 / hooks / 音频分帧转发），并接入当前前端聊天页。

**Architecture:** 一个会话 = 一行 `task_history`（新增 `HistoryTaskType::StreamingSpeech`）+ `streaming_tasks` 详情表（持 `context.json` / `input.jsonl` / 音频目录的占位符路径）。会话创建时拉起一个长期脚本进程（经 `begin_llm_task`），应用追加 `input.jsonl` 喂入文本，脚本以 JSON 行分帧写 stdout，Rust 后台 tokio reader 按 `contextId` 分发到前端 `Channel<AudioStreamEvent>`。取消复用 `active_task_controls` 的 `watch::Sender<bool>`，置位即 `child.kill()`。

**Tech Stack:** Rust（Tauri 2 / SeaORM / tokio）、SQLite、Vue 3 + Pinia + TypeScript（前端接入）。

## Global Constraints

- **DB schema 同步**：表结构变更须同步 `db/tables.sql` 与 `db/tables_pgsql.sql`（项目硬规范）。`voice_design_tasks` 的先例是**仅**在新迁移 + `db/tables*.sql` 中存在，**不**改 `create_local_schema.rs`；本表照此先例。
- **路径占位符**：DB 中路径一律用 `%DATA_DIR_PATH%` 占位符（`serialize_task_path` / `resolve_task_path`），不存绝对路径。
- **时间字段**：实体时间字段 `create_time` / `modify_time`，类型 `string`（`now_string()`），后端生成、前端只读。
- **任务参数契约**（规则4）：`create_*_task` / 流式进程 spawn 路径**不**进单测；以纯函数（args / params 文件 / 帧解析 / context ser-de）断言契约。
- **临时库隔离**（规则1&2）：DB 集成测试用 `LocalServiceHarness`（`std::env::temp_dir` 独立目录）。
- **前端不写自动化测试**：前端任务以手动验证为准。
- **本范围不含**：真实 `streaming.py`、`begin_llm_task` stdout 透传改造（属脚本侧后续）。Rust 按"spawn `begin_llm_task` 且 stdout 可读"假设编写。
- **迁移版本**：`LOCAL_SCHEMA_VERSION` `27 -> 28`，新迁移编号 `000011`，日期 `2026-07-18`。
- **taskId 来源更正**：spec 中"taskId 由前端生成"为占位期遗留；真实后端 `create_streaming_speech_task` 返回 `task_history.id` 作为 `taskId`，前端回填 store；`contextId` 仍前端生成（消息键 + 音频文件名 + 帧路由键）。

---

## File Structure

### 新增（Rust）
| 文件 | 职责 |
|------|------|
| `src-tauri/src/migration/m20260718_000011_add_streaming_tasks.rs` | 建 `streaming_tasks` 表 + 唯一索引 + FK |
| `src-tauri/src/service/local/entity/streaming_task.rs` | SeaORM Entity |
| `src-tauri/src/service/local/streaming.rs` | `create_streaming_speech_task_impl` / `send_streaming_message_impl` / `cancel_streaming_task_impl` / `sweep_stale_streaming_sessions_impl` / `start_streaming_session` |
| `src-tauri/src/service/pipeline/streaming.rs` | 纯函数（帧解析 / context+input ser-de / invocation 构造 / 路径解析）+ 长期进程 runner（stdout reader + Channel 分发 + cancel kill）+ `StreamingSessionExtra` / `MessageChannel` 类型 |
| `src-tauri/tests/streaming_schema.rs` | 迁移：表/列存在 + `HistoryTaskType::StreamingSpeech` 枚举契约 |
| `src-tauri/tests/streaming_service.rs` | 启动清扫 sweep：stale Running -> Cancelled |

### 修改（Rust）
| 文件 | 改动 |
|------|------|
| `src-tauri/src/migration/mod.rs` | 注册新迁移 + `LOCAL_SCHEMA_VERSION = "28"` |
| `src-tauri/src/service/models.rs` | `HistoryTaskType::StreamingSpeech`（as_str/storage_dir）+ `CreateStreamingSpeechTaskPayload` / `StreamingSpeakerInput` / `StreamingSpeechTaskResult` / `SendStreamingMessagePayload` |
| `src-tauri/src/service/pipeline/api/mod.rs` | `PythonScriptTaskKind::StreamingSpeech` + `StreamingArgs` + `PythonScriptTaskArgs::Streaming` + 内部测试 |
| `src-tauri/src/service/pipeline/mod.rs` | `pub mod streaming;` |
| `src-tauri/src/service/local/entity/mod.rs` | `pub mod streaming_task;` |
| `src-tauri/src/service/local/mod.rs` | `mod streaming;` + `Service` trait 3 方法 impl + `ActiveTaskControl` 增 `streaming_extra` + `init_db` 调 sweep + `start_streaming_session` |
| `src-tauri/src/service/mod.rs` | `Service` trait 增 3 方法签名 |
| `src-tauri/src/service/remote/mod.rs` | 3 方法 stub（bail 远程不支持） |
| `src-tauri/src/common/task_paths.rs` | `task_log_file_prefix` 增 `StreamingSpeech` 分支 + 流式路径辅助函数 |
| `src-tauri/src/hooks/streaming.rs` | 新增 3 个 `#[tauri::command]`（保留 `stream_audio_placeholder`） |
| `src-tauri/src/hooks/mod.rs` | `load_hooks` 注册 3 新命令 |
| `src-tauri/src/test_support.rs` | `sweep_stale_streaming_sessions` harness helper |
| `src-tauri/tests/pipeline_params.rs` | 增 streaming args 契约断言 |
| `db/tables.sql` / `db/tables_pgsql.sql` | 增 `streaming_tasks` 表 + 索引 |

### 修改（前端）
| 文件 | 改动 |
|------|------|
| `src/stores/streamingSpeech.ts` | `sendMessage` 异步化：首条建会话拿 `taskId` 回填；assistant 消息携带 `synthText`/`speakerName`；新增 `activeTaskId` + `terminateSession` |
| `src/types/streaming.ts` | `StreamingChatMessage` 增 `synthText` |
| `src/hooks/useStreamableAudioPlayer.ts` | `startStreaming` 改调 `send_streaming_message`，签名扩 `(taskId, contextId, speakerName, text)` |
| `src/components/common/StreamableAudioPlayer.vue` | `mode='stream'` 增 `speakerName` / `synthText` props 透传 |
| `src/views/StreamingSpeechView.vue` | 透传 `speakerName`/`synthText`；新增「终止会话」按钮 |

---

## Task 1: DB 迁移 + 枚举 + 路径辅助

**Files:**
- Create: `src-tauri/src/migration/m20260718_000011_add_streaming_tasks.rs`
- Modify: `src-tauri/src/migration/mod.rs`
- Modify: `src-tauri/src/service/models.rs:90-114`（`HistoryTaskType` enum + impl）
- Modify: `src-tauri/src/common/task_paths.rs:114-121`（`task_log_file_prefix`）+ 新增辅助函数
- Modify: `db/tables.sql` + `db/tables_pgsql.sql`
- Test: `src-tauri/tests/streaming_schema.rs`

**Interfaces:**
- Produces: `HistoryTaskType::StreamingSpeech`（`as_str() == "streaming-speech"`、`storage_dir() == "streaming"`）；`task_log_file_prefix(StreamingSpeech) == "streaming"`；`streaming_tasks` 表；`task_paths::streaming_sample_dir` / `streaming_context_json_path` / `streaming_input_cache_path` / `streaming_output_audio_dir` / `streaming_message_audio_path`。

- [ ] **Step 1: 写迁移测试（TDD）**

Create `src-tauri/tests/streaming_schema.rs`:

```rust
use kirine_client::test_support::LocalServiceHarness;
use kirine_client::service::models::HistoryTaskType;

#[tokio::test]
async fn streaming_tasks_table_exists_with_expected_columns() {
    let harness = LocalServiceHarness::new("streaming_schema").await.expect("harness");
    assert!(harness.table_exists("streaming_tasks").await.expect("table_exists"));
    for col in [
        "id", "history_id", "base_model", "model_version", "language", "device",
        "model_params_json", "context_file_path", "input_cache_file_path",
        "output_audio_dir", "message_count", "create_time", "modify_time", "deleted",
    ] {
        assert!(
            harness.table_has_column("streaming_tasks", col).await.expect("table_has_column"),
            "streaming_tasks missing column {col}"
        );
    }
    assert_eq!(HistoryTaskType::StreamingSpeech.as_str(), "streaming-speech");
    assert_eq!(HistoryTaskType::StreamingSpeech.storage_dir(), "streaming");
    harness.shutdown().await.expect("shutdown");
}
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --test streaming_schema --manifest-path src-tauri/Cargo.toml`
Expected: 编译失败（`HistoryTaskType::StreamingSpeech` 不存在 / `streaming_tasks` 表不存在）。

- [ ] **Step 3: 写迁移文件**

Create `src-tauri/src/migration/m20260718_000011_add_streaming_tasks.rs`:

```rust
use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20260718_000011_add_streaming_tasks"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(StreamingTasks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(StreamingTasks::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(StreamingTasks::HistoryId).integer().not_null())
                    .col(ColumnDef::new(StreamingTasks::BaseModel).string().not_null())
                    .col(ColumnDef::new(StreamingTasks::ModelVersion).string().not_null())
                    .col(ColumnDef::new(StreamingTasks::Language).string().not_null())
                    .col(
                        ColumnDef::new(StreamingTasks::Device)
                            .string()
                            .not_null()
                            .default("cpu"),
                    )
                    .col(
                        ColumnDef::new(StreamingTasks::ModelParamsJson)
                            .text()
                            .not_null()
                            .default("{}"),
                    )
                    .col(ColumnDef::new(StreamingTasks::ContextFilePath).text().not_null())
                    .col(ColumnDef::new(StreamingTasks::InputCacheFilePath).text().not_null())
                    .col(ColumnDef::new(StreamingTasks::OutputAudioDir).text().not_null())
                    .col(
                        ColumnDef::new(StreamingTasks::MessageCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(StreamingTasks::CreateTime).string().not_null())
                    .col(ColumnDef::new(StreamingTasks::ModifyTime).string().not_null())
                    .col(
                        ColumnDef::new(StreamingTasks::Deleted)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_streaming_tasks_history")
                            .from(StreamingTasks::Table, StreamingTasks::HistoryId)
                            .to(TaskHistory::Table, TaskHistory::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_streaming_tasks_history_id")
                    .table(StreamingTasks::Table)
                    .col(StreamingTasks::HistoryId)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_index(
                Index::drop()
                    .name("idx_streaming_tasks_history_id")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(StreamingTasks::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(DeriveIden)]
enum TaskHistory {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum StreamingTasks {
    Table,
    Id,
    HistoryId,
    BaseModel,
    ModelVersion,
    Language,
    Device,
    ModelParamsJson,
    ContextFilePath,
    InputCacheFilePath,
    OutputAudioDir,
    MessageCount,
    CreateTime,
    ModifyTime,
    Deleted,
}
```

- [ ] **Step 4: 注册迁移 + 升版本号**

Modify `src-tauri/src/migration/mod.rs`:

在 `mod m20260706_000010_rename_speakers_name_to_speaker_name;` 之后新增一行：
```rust
mod m20260718_000011_add_streaming_tasks;
```

把 `const LOCAL_SCHEMA_VERSION: &str = "27";` 改为：
```rust
const LOCAL_SCHEMA_VERSION: &str = "28";
```

在 `migrations()` 的 `vec![...]` 末尾（`m20260706_000010...` 之后）追加：
```rust
            Box::new(m20260718_000011_add_streaming_tasks::Migration),
```

- [ ] **Step 5: 加 `HistoryTaskType::StreamingSpeech`**

Modify `src-tauri/src/service/models.rs`。在 `pub enum HistoryTaskType` 中 `VoiceDesign,` 之后加 `StreamingSpeech,`：

```rust
pub enum HistoryTaskType {
    ModelTraining,
    TextToSpeech,
    VoiceClone,
    VoiceDesign,
    StreamingSpeech,
}
```

`as_str` 的 match 末尾加：
```rust
            Self::StreamingSpeech => "streaming-speech",
```

`storage_dir` 的 match 末尾加：
```rust
            Self::StreamingSpeech => "streaming",
```

- [ ] **Step 6: 加 `task_log_file_prefix` 分支 + 流式路径辅助**

Modify `src-tauri/src/common/task_paths.rs`。在文件顶部常量区新增：
```rust
const STREAMING_CONTEXT_JSON_NAME: &str = "context.json";
const STREAMING_INPUT_CACHE_NAME: &str = "input.jsonl";
const STREAMING_AUDIO_DIR_NAME: &str = "audio";
```

`task_log_file_prefix` 的 match 末尾加：
```rust
        HistoryTaskType::StreamingSpeech => "streaming",
```

在 `voice_design_params_json_path` 函数之后新增：
```rust
pub(crate) fn streaming_context_json_path(sample_root: &Path) -> PathBuf {
    sample_root.join(STREAMING_CONTEXT_JSON_NAME)
}

pub(crate) fn streaming_input_cache_path(sample_root: &Path) -> PathBuf {
    sample_root.join(STREAMING_INPUT_CACHE_NAME)
}

pub(crate) fn streaming_output_audio_dir(sample_root: &Path) -> PathBuf {
    sample_root.join(STREAMING_AUDIO_DIR_NAME)
}

pub(crate) fn streaming_message_audio_path(audio_dir: &Path, context_id: &str) -> PathBuf {
    audio_dir.join(format!("{}.wav", context_id))
}
```

- [ ] **Step 7: 同步 `db/tables.sql`**

在 `voice_design_tasks` 表之后、索引区之前新增：
```sql
CREATE TABLE
    IF NOT EXISTS streaming_tasks (
        id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
        history_id INTEGER NOT NULL,
        base_model TEXT NOT NULL,
        model_version TEXT NOT NULL,
        language TEXT NOT NULL,
        device TEXT NOT NULL DEFAULT 'cpu',
        model_params_json TEXT NOT NULL DEFAULT '{}',
        context_file_path TEXT NOT NULL,
        input_cache_file_path TEXT NOT NULL,
        output_audio_dir TEXT NOT NULL,
        message_count INTEGER NOT NULL DEFAULT 0,
        create_time TEXT NOT NULL,
        modify_time TEXT NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_streaming_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_streaming_tasks_history_id ON streaming_tasks (history_id);
```

- [ ] **Step 8: 同步 `db/tables_pgsql.sql`**

在 pgsql 文件对应位置新增（类型按 PostgreSQL 语法；索引名一致）：
```sql
CREATE TABLE
    IF NOT EXISTS streaming_tasks (
        id BIGSERIAL NOT NULL PRIMARY KEY,
        history_id BIGINT NOT NULL,
        base_model VARCHAR NOT NULL,
        model_version VARCHAR NOT NULL,
        language VARCHAR NOT NULL,
        device VARCHAR NOT NULL DEFAULT 'cpu',
        model_params_json TEXT NOT NULL DEFAULT '{}',
        context_file_path TEXT NOT NULL,
        input_cache_file_path TEXT NOT NULL,
        output_audio_dir TEXT NOT NULL,
        message_count INTEGER NOT NULL DEFAULT 0,
        create_time VARCHAR NOT NULL,
        modify_time VARCHAR NOT NULL,
        deleted INTEGER NOT NULL DEFAULT 0,
        CONSTRAINT fk_streaming_tasks_history FOREIGN KEY (history_id) REFERENCES task_history (id) ON DELETE CASCADE
    );

CREATE UNIQUE INDEX IF NOT EXISTS idx_streaming_tasks_history_id ON streaming_tasks (history_id);
```

- [ ] **Step 9: 运行测试，确认通过**

Run: `cargo test --test streaming_schema --manifest-path src-tauri/Cargo.toml`
Expected: PASS。

- [ ] **Step 10: 提交**

```bash
git add src-tauri/src/migration/m20260718_000011_add_streaming_tasks.rs src-tauri/src/migration/mod.rs src-tauri/src/service/models.rs src-tauri/src/common/task_paths.rs db/tables.sql db/tables_pgsql.sql src-tauri/tests/streaming_schema.rs
git commit -m "feat(streaming): 新增 streaming_tasks 表与 StreamingSpeech 任务类型"
```

---

## Task 2: 帧解析 + context/input ser-de 纯函数

**Files:**
- Create: `src-tauri/src/service/pipeline/streaming.rs`
- Modify: `src-tauri/src/service/pipeline/mod.rs`（`pub mod streaming;`）

**Interfaces:**
- Produces: `parse_streaming_frame(line: &str) -> Result<Option<StreamingFrame>>`；`StreamingFrame { context_id, payload: StreamingFramePayload }`（`Started` / `Chunk{bytes: Vec<u8>}` / `Finished` / `Error{message}`）；`StreamingContextJson` / `StreamingSpeaker` / `StreamingMessageEntry`（ser/de）；`serialize_input_entry(context_id, speaker_name, text, audio_path) -> String`。

- [ ] **Step 1: 写失败测试（模块内 `#[cfg(test)]`）**

Create `src-tauri/src/service/pipeline/streaming.rs` 先只写类型 + 测试：

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::hooks::streaming::AudioStreamEvent;

/// 脚本 stdout 单帧（JSON 行）。`bytes` 为 base64。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StreamingFrame {
    pub context_id: String,
    pub payload: StreamingFramePayload,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamingFramePayload {
    Started,
    Chunk { bytes: Vec<u8> },
    Finished,
    Error { message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StreamingContextJson {
    pub basic: StreamingContextBasic,
    pub messages: Vec<StreamingMessageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingContextBasic {
    pub task_id: i64,
    pub base_model: String,
    pub model_version: String,
    pub device: String,
    pub language: String,
    pub speakers: Vec<StreamingSpeaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingSpeaker {
    pub id: String,
    pub name: String,
    pub base_model: String,
    #[serde(default)]
    pub model_version: Option<String>,
    pub ref_audio_path: String,
    pub ref_audio_name: String,
    pub ref_text: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StreamingMessageEntry {
    pub context_id: String,
    pub speaker_name: String,
    pub text: String,
    pub audio_path: String,
}

pub(crate) fn serialize_input_entry(
    context_id: &str,
    speaker_name: &str,
    text: &str,
    audio_path: &str,
) -> String {
    serde_json::to_string(&StreamingMessageEntry {
        context_id: context_id.to_string(),
        speaker_name: speaker_name.to_string(),
        text: text.to_string(),
        audio_path: audio_path.to_string(),
    })
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_started_frame() {
        let frame = parse_streaming_frame(r#"{"type":"started","contextId":"msg-1"}"#)
            .expect("parse")
            .expect("some");
        assert_eq!(frame.context_id, "msg-1");
        assert_eq!(frame.payload, StreamingFramePayload::Started);
    }

    #[test]
    fn parses_chunk_frame_with_base64_bytes() {
        // "hi" -> base64 "aGk="
        let frame = parse_streaming_frame(
            r#"{"type":"chunk","contextId":"msg-1","bytes":"aGk="}"#,
        )
        .expect("parse")
        .expect("some");
        assert_eq!(frame.payload, StreamingFramePayload::Chunk { bytes: vec![b'h', b'i'] });
    }

    #[test]
    fn parses_finished_and_error_frames() {
        let fin = parse_streaming_frame(r#"{"type":"finished","contextId":"msg-1"}"#)
            .expect("parse")
            .expect("some");
        assert_eq!(fin.payload, StreamingFramePayload::Finished);

        let err = parse_streaming_frame(
            r#"{"type":"error","contextId":"msg-1","message":"boom"}"#,
        )
        .expect("parse")
        .expect("some");
        assert_eq!(
            err.payload,
            StreamingFramePayload::Error { message: "boom".to_string() }
        );
    }

    #[test]
    fn blank_line_yields_none() {
        assert!(parse_streaming_frame("   ").expect("parse").is_none());
    }

    #[test]
    fn malformed_line_is_err() {
        assert!(parse_streaming_frame("not json").is_err());
    }

    #[test]
    fn context_json_round_trips() {
        let ctx = StreamingContextJson {
            basic: StreamingContextBasic {
                task_id: 7,
                base_model: "gpt_sovits_cpufast".to_string(),
                model_version: "v1".to_string(),
                device: "cpu".to_string(),
                language: "chinese".to_string(),
                speakers: vec![StreamingSpeaker {
                    id: "spk-1".to_string(),
                    name: "A".to_string(),
                    base_model: "gpt_sovits_cpufast".to_string(),
                    model_version: None,
                    ref_audio_path: "/ref.wav".to_string(),
                    ref_audio_name: "ref.wav".to_string(),
                    ref_text: "参考".to_string(),
                    description: None,
                }],
            },
            messages: vec![StreamingMessageEntry {
                context_id: "msg-2".to_string(),
                speaker_name: "A".to_string(),
                text: "你好".to_string(),
                audio_path: "%DATA_DIR_PATH%/streaming_7/audio/msg-2.wav".to_string(),
            }],
        };
        let json = serde_json::to_string(&ctx).expect("serialize");
        let back: StreamingContextJson = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.basic.task_id, 7);
        assert_eq!(back.basic.speakers[0].ref_audio_path, "/ref.wav");
        assert_eq!(back.messages[0].context_id, "msg-2");
        // camelCase 字段名
        assert!(json.contains("\"taskId\""));
        assert!(json.contains("\"contextId\""));
        assert!(json.contains("\"audioPath\""));
    }

    #[test]
    fn input_entry_serializes_single_line() {
        let line = serialize_input_entry("msg-2", "A", "你好", "/p/a.wav");
        assert!(!line.contains('\n'));
        let v: serde_json::Value = serde_json::from_str(&line).expect("parse");
        assert_eq!(v["contextId"], "msg-2");
        assert_eq!(v["text"], "你好");
    }
}
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kirine-client streaming::tests -- --nocapture`（或 `cargo test -p kirine-client streaming` ）
Expected: 编译失败（`parse_streaming_frame` 未定义）。

- [ ] **Step 3: 实现 `parse_streaming_frame`**

在 `streaming.rs` 的 `serialize_input_entry` 之后、`#[cfg(test)]` 之前新增：

```rust
#[derive(Deserialize)]
struct RawFrame {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "contextId")]
    context_id: String,
    #[serde(default)]
    bytes: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

/// 解析脚本 stdout 一行。空行返回 `Ok(None)`；坏行返回 `Err`。
pub(crate) fn parse_streaming_frame(line: &str) -> Result<Option<StreamingFrame>> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let raw: RawFrame = serde_json::from_str(trimmed).context("failed to parse streaming frame")?;
    let payload = match raw.kind.as_str() {
        "started" => StreamingFramePayload::Started,
        "chunk" => {
            let bytes = base64_decode(raw.bytes.as_deref().unwrap_or(""))?;
            StreamingFramePayload::Chunk { bytes }
        }
        "finished" => StreamingFramePayload::Finished,
        "error" => StreamingFramePayload::Error {
            message: raw.message.unwrap_or_default(),
        },
        other => anyhow::bail!("unknown streaming frame type: {other}"),
    };
    Ok(Some(StreamingFrame { context_id: raw.context_id, payload }))
}

/// 将帧映射为下发前端的 `AudioStreamEvent`。
pub(crate) fn frame_to_event(frame: &StreamingFrame) -> Option<AudioStreamEvent> {
    match &frame.payload {
        StreamingFramePayload::Started => Some(AudioStreamEvent::Started),
        StreamingFramePayload::Chunk { bytes } => Some(AudioStreamEvent::Chunk { bytes: bytes.clone() }),
        StreamingFramePayload::Finished => Some(AudioStreamEvent::Finished),
        StreamingFramePayload::Error { message } => Some(AudioStreamEvent::Error { message: message.clone() }),
    }
}

fn base64_decode(value: &str) -> Result<Vec<u8>> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(value).context("failed to base64-decode chunk bytes")
}
```

- [ ] **Step 4: 注册模块 + 加 `base64` 依赖**

Modify `src-tauri/src/service/pipeline/mod.rs`：在 `pub mod voice_design;` 之后加 `pub mod streaming;`。

检查 `src-tauri/Cargo.toml` 是否已有 `base64` 依赖；若无，在 `[dependencies]` 加：
```toml
base64 = "0.22"
```
（若已存在则跳过。）

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kirine-client streaming::tests`
Expected: 6 个测试 PASS。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/service/pipeline/streaming.rs src-tauri/src/service/pipeline/mod.rs src-tauri/Cargo.toml
git commit -m "feat(streaming): 帧解析与 context/input ser-de 纯函数"
```

---

## Task 3: Models payload/result + StreamingArgs

**Files:**
- Modify: `src-tauri/src/service/models.rs`（新增 4 个 struct）
- Modify: `src-tauri/src/service/pipeline/api/mod.rs`（`PythonScriptTaskKind` / `PythonScriptTaskArgs` + `StreamingArgs` + 内部测试）

**Interfaces:**
- Produces: `CreateStreamingSpeechTaskPayload { base_model, model_version, device, language, model_params, speakers: Vec<StreamingSpeakerInput> }`；`StreamingSpeakerInput { name, base_model, model_version?, ref_audio_path, ref_audio_name, ref_text, description? }`；`StreamingSpeechTaskResult { task_id, context_file_path, input_cache_file_path, output_audio_dir, status, created_at }`；`SendStreamingMessagePayload { task_id, context_id, speaker_name, text }`；`PythonScriptTaskKind::StreamingSpeech`；`StreamingArgs { context_file_path, input_cache_file_path, output_audio_dir, speakers: Vec<StreamingSpeakerArg> }`；`PythonScriptTaskArgs::Streaming(StreamingArgs)`。

- [ ] **Step 1: 写失败测试（api 模块内）**

Modify `src-tauri/src/service/pipeline/api/mod.rs`。先在 `PythonScriptTaskKind` 加 `StreamingSpeech,`，在 `PythonScriptTaskArgs` 加 `Streaming(StreamingArgs)`，并定义 `StreamingArgs` + `StreamingSpeakerArg`（见 Step 3），然后在文件末尾的 `#[cfg(test)] mod tests` 内追加测试：

```rust
    #[test]
    fn writes_streaming_params_file_with_expected_fields() {
        let spec = PythonScriptInvocationSpec {
            version: "1.0.0".to_string(),
            base_model: "gpt_sovits_cpufast".to_string(),
            model_version: "v1".to_string(),
            kind: PythonScriptTaskKind::StreamingSpeech,
            runtime: runtime(),
            args: PythonScriptTaskArgs::Streaming(StreamingArgs {
                context_file_path: "/ctx/context.json".to_string(),
                input_cache_file_path: "/ctx/input.jsonl".to_string(),
                output_audio_dir: "/ctx/audio".to_string(),
                speakers: vec![StreamingSpeakerArg {
                    name: "A".to_string(),
                    ref_audio_path: "/ref.wav".to_string(),
                    ref_text: "参考".to_string(),
                }],
            }),
        };

        let json = write_and_read(&spec, "streaming");
        assert_eq!(json["kind"], "StreamingSpeech");
        assert_eq!(json["args"]["Streaming"]["context_file_path"], "/ctx/context.json");
        assert_eq!(json["args"]["Streaming"]["input_cache_file_path"], "/ctx/input.jsonl");
        assert_eq!(json["args"]["Streaming"]["speakers"][0]["name"], "A");
        assert_eq!(json["args"]["Streaming"]["speakers"][0]["ref_text"], "参考");
    }
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kirine-client pipeline::api::tests::writes_streaming_params_file_with_expected_fields`
Expected: 编译失败（`StreamingArgs` / `StreamingSpeech` 未定义）。

- [ ] **Step 3: 实现 `StreamingArgs` + 枚举变体**

In `src-tauri/src/service/pipeline/api/mod.rs`:

`PythonScriptTaskKind` 加变体：
```rust
pub(crate) enum PythonScriptTaskKind {
    Training,
    TextToSpeech,
    VoiceClone,
    VoiceDesign,
    StreamingSpeech,
}
```

在 `VoiceDesignArgs` 之后新增：
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StreamingSpeakerArg {
    pub name: String,
    pub ref_audio_path: String,
    pub ref_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StreamingArgs {
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub speakers: Vec<StreamingSpeakerArg>,
}
```

`PythonScriptTaskArgs` 加变体：
```rust
pub(crate) enum PythonScriptTaskArgs {
    Training(TrainingArgs),
    TextToSpeech(TTSArgs),
    VoiceClone(VoiceCloneArgs),
    VoiceDesign(VoiceDesignArgs),
    Streaming(StreamingArgs),
}
```

- [ ] **Step 4: 加 models payload/result**

In `src-tauri/src/service/models.rs`，在 `CreateVoiceDesignTaskPayload` 之后新增：

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingSpeakerInput {
    pub name: String,
    pub base_model: BaseModel,
    #[serde(default)]
    pub model_version: Option<String>,
    pub ref_audio_path: String,
    pub ref_audio_name: String,
    pub ref_text: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStreamingSpeechTaskPayload {
    pub base_model: BaseModel,
    pub model_version: String,
    pub device: HardwareType,
    pub language: AppLanguage,
    pub model_params: Value,
    pub speakers: Vec<StreamingSpeakerInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendStreamingMessagePayload {
    pub task_id: i64,
    pub context_id: String,
    pub speaker_name: String,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamingSpeechTaskResult {
    pub task_id: i64,
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub status: TaskStatus,
    pub created_at: String,
}
```

确保 `models.rs` 顶部已 `use crate::config::{BaseModel, HardwareType};` 且 `AppLanguage`、`Value`、`TaskStatus` 在作用域内（既有文件已具备，无需额外 import）。

- [ ] **Step 5: 运行测试，确认通过**

Run: `cargo test --manifest-path src-tauri/Cargo.toml -p kirine-client pipeline::api::tests`
Expected: 全部 PASS（含新增 streaming 用例）。

- [ ] **Step 6: 提交**

```bash
git add src-tauri/src/service/pipeline/api/mod.rs src-tauri/src/service/models.rs
git commit -m "feat(streaming): payload/result 类型与 StreamingArgs 契约"
```

---

## Task 4: 流式进程 runner（stdout reader + Channel 分发 + cancel kill）

**Files:**
- Modify: `src-tauri/src/service/pipeline/streaming.rs`（追加 runner + 路径解析 + invocation 构造）
- Modify: `src-tauri/src/tests/pipeline_params.rs`（streaming args 契约断言）
- Modify: `src-tauri/src/test_support.rs`（重导出 `build_streaming_invocation` 用于断言，若需要）

**Interfaces:**
- Produces: `StreamingPipelineRequest { task_id }`；`resolve_streaming_paths(service, task_id, base_model, model_version) -> Result<ResolvedStreamingPaths>`；`build_streaming_invocation(paths, base_model, model_version, device, speakers) -> PythonScriptInvocationSpec`；`StreamingSessionExtra { message_channels: Arc<RwLock<HashMap<String, MessageChannel>>> }`；`MessageChannel { on_event: Channel<AudioStreamEvent>, done: oneshot::Sender<StreamMessageOutcome> }`；`StreamMessageOutcome { cancelled: bool, errored: bool }`；`LoadedStreamingDetail { base_model, model_version, device, speakers }`；`pub(crate) async fn run_streaming_session(service, request, base_model, detail, extra) -> Result<()>`（长期存活：spawn 子进程 + 读 stdout + 分发 + cancel kill + 退出时清理 pending channels；`detail` / `extra` 由 `start_streaming_session`（Task 5）传入，本函数不调用 `load_streaming_task_detail` / `streaming_session_extra`，避免前向依赖）。

- [ ] **Step 1: 写 args 契约断言（pipeline_params 测试）**

先查看 `src-tauri/tests/pipeline_params.rs` 现有结构，在其末尾追加（用 `build_llm_task_script_args`，复用既有纯函数；streaming 走同一 args 形状）：

```rust
#[test]
fn streaming_script_args_match_begin_llm_task_contract() {
    use kirine_client::service::pipeline::build_llm_task_script_args;
    let script = std::path::Path::new("/src-model/gpt_sovits_cpufast/streaming.py");
    let params = std::path::Path::new("/d/streaming_1/streaming.params.json");
    let task_log = std::path::Path::new("/log/task/streaming-1.log");
    let args = build_llm_task_script_args(script, params, task_log, "gpt_sovits_cpufast");
    assert_eq!(
        args,
        vec![
            "--base-model".to_string(),
            "gpt_sovits_cpufast".to_string(),
            "--script-path".to_string(),
            "/src-model/gpt_sovits_cpufast/streaming.py".to_string(),
            "--params-file".to_string(),
            "/d/streaming_1/streaming.params.json".to_string(),
            "--log-path".to_string(),
            "/log/task/streaming-1.log".to_string(),
            "--task-log-file".to_string(),
            "/log/task/streaming-1.log".to_string(),
        ]
    );
}
```

- [ ] **Step 2: 运行测试，确认通过（既有纯函数，应直接通过）**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test pipeline_params streaming_script_args_match_begin_llm_task_contract`
Expected: PASS（断言既有契约；若 `build_llm_task_script_args` 未通过 test_support 暴露给测试 crate，改为直接用 `kirine_client::service::pipeline::build_llm_task_script_args`——该函数为 `pub`，可直接访问）。

- [ ] **Step 3: 在 `pipeline/mod.rs` 加 `StreamingPipelineRequest`**

In `src-tauri/src/service/pipeline/mod.rs`，在 `VoiceDesignPipelineRequest` 之后新增：
```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamingPipelineRequest {
    pub task_id: i64,
}
```

- [ ] **Step 4: 实现 runner（路径 + invocation + 进程 + stdout reader + 分发 + cancel）**

在 `src-tauri/src/service/pipeline/streaming.rs` 顶部追加 import，并追加 runner 代码。文件最终 import 区为：

```rust
use std::{
    collections::HashMap,
    io::AsyncBufReadExt,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    process::{Child, Command},
    sync::{oneshot, watch},
};
use tracing::{error, info, warn};

use crate::{
    common::{
        local_paths::{resolve_local_log_dir, resolve_task_path, serialize_task_path},
        task_paths::{
            self, streaming_context_json_path, streaming_input_cache_path,
            streaming_message_audio_path, streaming_output_audio_dir, task_log_file_path,
            task_sample_dir,
        },
    },
    config::{BaseModel, HardwareType},
    hooks::streaming::AudioStreamEvent,
    service::{
        local::LocalService,
        models::{HistoryTaskType, StreamingSpeakerInput, TaskStatus, UpdateTaskStatusPayload},
        pipeline::{
            api::{
                PythonScriptInvocationSpec, PythonScriptRuntimeOptions, PythonScriptTaskArgs,
                PythonScriptTaskKind, StreamingArgs, StreamingSpeakerArg,
            },
            script_paths::{
                resolve_src_model_root, src_model_begin_llm_task_script_path,
                src_model_model_python_script_path, ScriptPlatform,
            },
            StreamingPipelineRequest, build_llm_task_script_args,
        },
    },
    utils::process::LoggedCommandResult,
    Result,
};
```

在纯函数区之后追加：

```rust
#[derive(Debug, Clone)]
pub(crate) struct ResolvedStreamingPaths {
    pub base_model: String,
    pub model_version: String,
    pub src_model_root: PathBuf,
    pub begin_llm_task_script_path: PathBuf,
    pub streaming_python_script_path: PathBuf,
    pub sample_root: PathBuf,
    pub context_json_path: PathBuf,
    pub input_cache_path: PathBuf,
    pub output_audio_dir: PathBuf,
    pub params_json_path: PathBuf,
}

pub(crate) fn resolve_streaming_paths(
    service: &LocalService,
    task_id: i64,
    base_model: &str,
    model_version: &str,
) -> Result<ResolvedStreamingPaths> {
    let src_model_root = resolve_src_model_root(service.app_dir())?;
    let begin_llm_task_script_path = src_model_begin_llm_task_script_path(&src_model_root);
    let streaming_python_script_path =
        src_model_model_python_script_path(&src_model_root, base_model, "streaming.py")?;
    let sample_root = task_sample_dir(
        Path::new(service.data_dir()),
        HistoryTaskType::StreamingSpeech,
        task_id,
    );
    let context_json_path = streaming_context_json_path(&sample_root);
    let input_cache_path = streaming_input_cache_path(&sample_root);
    let output_audio_dir = streaming_output_audio_dir(&sample_root);
    let params_json_path = sample_root.join("streaming.params.json");

    Ok(ResolvedStreamingPaths {
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        src_model_root,
        begin_llm_task_script_path,
        streaming_python_script_path,
        sample_root,
        context_json_path,
        input_cache_path,
        output_audio_dir,
        params_json_path,
    })
}

pub(crate) fn build_streaming_invocation(
    paths: &ResolvedStreamingPaths,
    base_model: &str,
    model_version: &str,
    device: HardwareType,
    speakers: Vec<StreamingSpeakerInput>,
) -> PythonScriptInvocationSpec {
    PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: base_model.to_string(),
        model_version: model_version.to_string(),
        kind: PythonScriptTaskKind::StreamingSpeech,
        runtime: PythonScriptRuntimeOptions {
            device: Some(device.runtime_arg().to_string()),
            logging_dir: None,
            attn_implementation: None,
        },
        args: PythonScriptTaskArgs::Streaming(StreamingArgs {
            context_file_path: paths.context_json_path.to_string_lossy().to_string(),
            input_cache_file_path: paths.input_cache_path.to_string_lossy().to_string(),
            output_audio_dir: paths.output_audio_dir.to_string_lossy().to_string(),
            speakers: speakers
                .into_iter()
                .map(|s| StreamingSpeakerArg {
                    name: s.name,
                    ref_audio_path: s.ref_audio_path,
                    ref_text: s.ref_text,
                })
                .collect(),
        }),
    }
}

/// 单条消息的前端 Channel + 完成信号。
pub(crate) struct MessageChannel {
    pub on_event: Channel<AudioStreamEvent>,
    pub done: oneshot::Sender<StreamMessageOutcome>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StreamMessageOutcome {
    pub cancelled: bool,
    pub errored: bool,
}

/// 流式会话附加态：存于 `ActiveTaskControl.streaming_extra`，runner 与
/// `send_streaming_message` 共享 `message_channels`。
#[derive(Debug, Clone, Default)]
pub(crate) struct StreamingSessionExtra {
    pub message_channels: Arc<RwLock<HashMap<String, MessageChannel>>>,
}

#[derive(Debug, Clone)]
pub(crate) struct LoadedStreamingDetail {
    pub base_model: String,
    pub model_version: String,
    pub device: HardwareType,
    pub speakers: Vec<StreamingSpeakerInput>,
}

/// 长期存活的流式会话 runner：spawn `begin_llm_task` -> streaming.py，逐行读 stdout
/// 分帧、按 contextId 分发到 `message_channels`；cancel 信号到达即 kill 子进程并
/// 通知所有 pending 消息。会话状态机：Running（spawn 后）-> Cancelled/Failed（退出）。
/// `detail` / `extra` 由 `start_streaming_session`（Task 5）从 DB / 会话注册表取后传入，
/// 本函数不调用 `load_streaming_task_detail` / `streaming_session_extra`，避免前向依赖。
pub(crate) async fn run_streaming_session(
    service: &LocalService,
    request: StreamingPipelineRequest,
    base_model: &str,
    detail: LoadedStreamingDetail,
    extra: Arc<StreamingSessionExtra>,
) -> Result<()> {
    let task_id = request.task_id;
    if detail.base_model.trim() != base_model {
        bail!(
            "streaming task base model mismatch: expected {}, got {}",
            base_model,
            detail.base_model
        );
    }
    let paths = resolve_streaming_paths(service, task_id, &detail.base_model, &detail.model_version)?;
    let invocation = build_streaming_invocation(
        &paths,
        &detail.base_model,
        &detail.model_version,
        detail.device,
        detail.speakers,
    );
    invocation.write_to_json_file(&paths.params_json_path)?;

    let runtime_config = service.runtime_config()?;
    let log_dir = resolve_local_log_dir(&runtime_config)?;
    let task_log_path = task_log_file_path(&log_dir, HistoryTaskType::StreamingSpeech, task_id);

    let mut cancel_rx =
        service.active_task_cancel_receiver(task_id, HistoryTaskType::StreamingSpeech)?;

    service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: TaskStatus::Running,
            duration_seconds: None,
        })
        .await?;

    let platform = ScriptPlatform::current();
    let mut shell_args = platform.shell_base_args();
    shell_args.push(paths.begin_llm_task_script_path.to_string_lossy().to_string());
    shell_args.extend(build_llm_task_script_args(
        &paths.streaming_python_script_path,
        &paths.params_json_path,
        &task_log_path,
        &detail.base_model,
    ));

    let mut child = Command::new(platform.shell_program())
        .args(&shell_args)
        .current_dir(&paths.src_model_root)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| "failed to spawn streaming begin_llm_task")?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let task_log_path_clone = task_log_path.clone();
    // stderr -> task-log-file（异步落盘）
    tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut file = match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&task_log_path_clone)
        {
            Ok(f) => f,
            Err(_) => return,
        };
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = writeln!(file, "{line}");
        }
    });

    let outcome = drive_streaming_stdout(
        &mut child,
        stdout,
        &extra,
        &mut cancel_rx,
    )
    .await;

    // 退出前清理所有 pending 消息（cancel / 进程退出）
    drain_pending_channels(&extra, outcome.is_cancelled());

    let final_status = match &outcome {
        StreamSessionOutcome::Cancelled => TaskStatus::Cancelled,
        StreamSessionOutcome::Failed => TaskStatus::Failed,
        StreamSessionOutcome::Exited => TaskStatus::Cancelled, // 进程自然退出视为会话终止
    };
    let _ = service
        .update_task_status_impl(UpdateTaskStatusPayload {
            task_id,
            status: final_status,
            duration_seconds: None,
        })
        .await;

    match outcome {
        StreamSessionOutcome::Cancelled | StreamSessionOutcome::Exited => Ok(()),
        StreamSessionOutcome::Failed => bail!("streaming session failed for task {task_id}"),
    }
}

#[derive(Debug, Clone, Copy)]
enum StreamSessionOutcome {
    Cancelled,
    Failed,
    Exited,
}

impl StreamSessionOutcome {
    fn is_cancelled(self) -> bool {
        matches!(self, StreamSessionOutcome::Cancelled)
    }
}

async fn drive_streaming_stdout(
    child: &mut Child,
    stdout: tokio::process::ChildStdout,
    extra: &StreamingSessionExtra,
    cancel_rx: &mut watch::Receiver<bool>,
) -> StreamSessionOutcome {
    let mut reader = BufReader::new(stdout).lines();
    let mut failed = false;
    loop {
        tokio::select! {
            biased;
            _ = cancel_rx.changed() => {
                if *cancel_rx.borrow() {
                    info!("streaming cancel signal received, killing child");
                    let _ = child.kill().await;
                    return StreamSessionOutcome::Cancelled;
                }
            }
            line = reader.next_line() => {
                match line {
                    Ok(Some(raw)) => {
                        match parse_streaming_frame(&raw) {
                            Ok(Some(frame)) => {
                                if let Some(event) = frame_to_event(&frame) {
                                    forward_event(extra, &frame.context_id, event);
                                }
                                if matches!(frame.payload, StreamingFramePayload::Error { .. }) {
                                    warn!(context_id = %frame.context_id, "streaming frame error");
                                }
                            }
                            Ok(None) => {}
                            Err(err) => {
                                warn!(error = %err, line = %raw, "ignoring unparseable streaming line");
                            }
                        }
                    }
                    Ok(None) => {
                        // stdout EOF：进程结束
                        break;
                    }
                    Err(err) => {
                        error!(error = %err, "streaming stdout read error");
                        failed = true;
                        break;
                    }
                }
            }
        }
    }

    // 等子进程退出以取状态
    let status = child.wait().await;
    if failed || status.map(|s| !s.success()).unwrap_or(true) {
        StreamSessionOutcome::Failed
    } else {
        StreamSessionOutcome::Exited
    }
}

fn forward_event(extra: &StreamingSessionExtra, context_id: &str, event: AudioStreamEvent) {
    let is_terminal = matches!(
        event,
        AudioStreamEvent::Finished | AudioStreamEvent::Error { .. }
    );
    let mut guard = match extra.message_channels.write() {
        Ok(g) => g,
        Err(_) => return,
    };
    if is_terminal {
        if let Some(mc) = guard.remove(context_id) {
            let errored = matches!(event, AudioStreamEvent::Error { .. });
            let _ = mc.on_event.send(event);
            let _ = mc.done.send(StreamMessageOutcome { cancelled: false, errored });
        }
    } else if let Some(mc) = guard.get_mut(context_id) {
        // Channel::send 取 &self，无需 Clone
        let _ = mc.on_event.send(event);
    }
}

fn drain_pending_channels(extra: &StreamingSessionExtra, cancelled: bool) {
    let mut guard = match extra.message_channels.write() {
        Ok(g) => g,
        Err(_) => return,
    };
    let pending = guard.drain().collect::<Vec<_>>();
    drop(guard);
    for (_, mc) in pending {
        let _ = mc.on_event.send(AudioStreamEvent::Error {
            message: if cancelled {
                "流式会话已终止".to_string()
            } else {
                "流式会话已结束".to_string()
            },
        });
        let _ = mc.done.send(StreamMessageOutcome { cancelled, errored: !cancelled });
    }
}
```

> 注：`tauri::ipc::Channel::send` 取 `&self`，故 `forward_event` 用 `get_mut`/`remove` 借用 `MessageChannel` 即可单发事件，无需 `Channel: Clone`。

- [ ] **Step 5: 编译校验 runner（无独立测试，规则4）**

Run: `cargo build --manifest-path src-tauri/Cargo.toml -p kirine-client`
Expected: 编译通过。`run_streaming_session` 仅依赖既有 `LocalService` 方法（`active_task_cancel_receiver` / `update_task_status_impl` / `runtime_config`）与 Task 2/4 内类型（`LoadedStreamingDetail` 已在本任务定义）；`detail` / `extra` 为参数，不引用 Task 5 的 `load_streaming_task_detail` / `streaming_session_extra`。runner 本体不进单测（规则4）。

- [ ] **Step 6: 运行 args 契约测试 + 全量编译**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test pipeline_params` && `cargo build --manifest-path src-tauri/Cargo.toml -p kirine-client`
Expected: pipeline_params PASS；编译通过。

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/service/pipeline/streaming.rs src-tauri/src/service/pipeline/mod.rs src-tauri/tests/pipeline_params.rs
git commit -m "feat(streaming): 长期进程 runner（stdout 分帧分发 + cancel kill）"
```

---

## Task 5: Entity + Service 层（create / send / cancel / sweep）+ Service trait + Remote stub

**Files:**
- Create: `src-tauri/src/service/local/entity/streaming_task.rs`
- Create: `src-tauri/src/service/local/streaming.rs`
- Modify: `src-tauri/src/service/local/entity/mod.rs`
- Modify: `src-tauri/src/service/local/mod.rs`（`mod streaming;` + trait impl + `ActiveTaskControl.streaming_extra` + `init_db` sweep + `start_streaming_session`）
- Modify: `src-tauri/src/service/mod.rs`（trait 签名）
- Modify: `src-tauri/src/service/remote/mod.rs`（3 stub）
- Modify: `src-tauri/src/test_support.rs`（sweep helper）
- Test: `src-tauri/tests/streaming_service.rs`

**Interfaces:**
- Produces: `streaming_task` entity；`LocalService::create_streaming_speech_task_impl` / `send_streaming_message_impl` / `cancel_streaming_task_impl` / `sweep_stale_streaming_sessions_impl` / `start_streaming_session` / `load_streaming_task_detail` / `streaming_session_extra`；`Service` trait 3 方法；`RemoteService` 3 stub。

- [ ] **Step 1: 写 sweep 测试（TDD）**

Create `src-tauri/tests/streaming_service.rs`:

```rust
use kirine_client::service::models::{HistoryTaskType, TaskStatus};
use kirine_client::service::Service;
use kirine_client::test_support::LocalServiceHarness;

#[tokio::test]
async fn sweep_marks_stale_running_streaming_sessions_cancelled() {
    let harness = LocalServiceHarness::new("streaming_sweep").await.expect("harness");
    harness
        .seed_history(1, HistoryTaskType::StreamingSpeech, TaskStatus::Running)
        .await
        .expect("seed running");
    harness
        .seed_history(2, HistoryTaskType::TextToSpeech, TaskStatus::Running)
        .await
        .expect("seed tts running");

    harness.sweep_stale_streaming_sessions().await.expect("sweep");

    let streaming = harness.get_history_record(1).await.expect("get streaming");
    assert_eq!(streaming.status, TaskStatus::Cancelled);
    // 非 streaming 的 Running 不应被清扫
    let tts = harness.get_history_record(2).await.expect("get tts");
    assert_eq!(tts.status, TaskStatus::Running);

    harness.shutdown().await.expect("shutdown");
}
```

- [ ] **Step 2: 运行测试，确认失败**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test streaming_service`
Expected: 编译失败（`sweep_stale_streaming_sessions` 不存在）。

- [ ] **Step 3: 写 entity**

Create `src-tauri/src/service/local/entity/streaming_task.rs`:

```rust
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "streaming_tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub history_id: i64,
    pub base_model: String,
    pub model_version: String,
    pub language: String,
    pub device: String,
    pub model_params_json: String,
    pub context_file_path: String,
    pub input_cache_file_path: String,
    pub output_audio_dir: String,
    pub message_count: i64,
    pub create_time: String,
    pub modify_time: String,
    pub deleted: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

Modify `src-tauri/src/service/local/entity/mod.rs`：在 `pub mod voice_design_task;` 之后加 `pub mod streaming_task;`。

- [ ] **Step 4: 写 service 层**

Create `src-tauri/src/service/local/streaming.rs`:

```rust
use std::path::Path;

use anyhow::bail;
use sea_orm::{
    ActiveModelTrait, ActiveValue::NotSet, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter,
    TransactionTrait,
};
use serde_json::Value;
use tracing::{error, info};

use crate::{
    common::{
        local_paths::{ensure_child_dir, serialize_task_path},
        task_paths::{
            ensure_task_sample_dir, streaming_context_json_path, streaming_input_cache_path,
            streaming_output_audio_dir,
        },
    },
    config::HardwareType,
    service::{
        local::entity::{
            streaming_task as streaming_task_entity, task_history as task_history_entity,
        },
        models::{
            CreateStreamingSpeechTaskPayload, HistoryTaskType, SendStreamingMessagePayload,
            StreamingContextJson, StreamingContextBasic, StreamingMessageEntry, StreamingSpeaker,
            StreamingSpeakerInput, StreamingSpeechTaskResult, TaskStatus,
            UpdateTaskStatusPayload,
        },
        pipeline::{
            streaming::{run_streaming_session, StreamingPipelineRequest},
            StreamingSessionExtra,
        },
        LocalService,
    },
    utils::time::now_string,
    Result,
};

impl LocalService {
    pub(crate) async fn create_streaming_speech_task_impl(
        &self,
        payload: CreateStreamingSpeechTaskPayload,
    ) -> Result<StreamingSpeechTaskResult> {
        let create_time = now_string()?;
        let base_model = payload.base_model.trim().to_string();
        let model_version = payload.model_version.trim().to_string();
        let device = payload.device;
        let selected_model_info = self
            .find_supported_model_variant(&base_model, &model_version)
            .await?;
        if !selected_model_info.supported_devices.contains(&device) {
            bail!(
                "模型 {} {} 不支持设备 {}，请切换为 {:?}",
                selected_model_info.model_name,
                selected_model_info.model_version,
                device,
                selected_model_info.supported_devices
            );
        }
        if payload.speakers.is_empty() {
            bail!("流式语音会话至少需要一个说话人");
        }

        let mut model_params = payload.model_params.clone();
        let title = super::build_task_title("流式语音", None, &create_time);

        let txn = self.orm().begin().await?;
        let task_history = task_history_entity::ActiveModel {
            id: NotSet,
            task_type: Set(HistoryTaskType::StreamingSpeech.as_str().to_string()),
            title: Set(title),
            speaker_id: Set(None),
            speaker_name_snapshot: Set("-".to_string()),
            status: Set(TaskStatus::Pending.as_str().to_string()),
            duration_seconds: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            finished_time: Set(None),
            device: Set(device.as_str().to_string()),
            deleted: Set(0),
        }
        .insert(&txn)
        .await?;
        let task_id = task_history.id;

        let sample_dir = ensure_task_sample_dir(
            Path::new(self.data_dir()),
            HistoryTaskType::StreamingSpeech,
            task_id,
        )?;
        super::copy_model_param_files(
            &base_model,
            HistoryTaskType::StreamingSpeech,
            &mut model_params,
            &sample_dir,
            Path::new(self.data_dir()),
            self.ui_config(),
        )?;

        let context_json_path = streaming_context_json_path(&sample_dir);
        let input_cache_path = streaming_input_cache_path(&sample_dir);
        let output_audio_dir = streaming_output_audio_dir(&sample_dir);
        std::fs::create_dir_all(&output_audio_dir)?;

        let context = StreamingContextJson {
            basic: StreamingContextBasic {
                task_id,
                base_model: base_model.clone(),
                model_version: model_version.clone(),
                device: device.as_str().to_string(),
                language: payload.language.as_str().to_string(),
                speakers: payload
                    .speakers
                    .iter()
                    .map(|s| StreamingSpeaker {
                        id: s.name.clone(),
                        name: s.name.clone(),
                        base_model: s.base_model.trim().to_string(),
                        model_version: s.model_version.clone(),
                        ref_audio_path: s.ref_audio_path.clone(),
                        ref_audio_name: s.ref_audio_name.clone(),
                        ref_text: s.ref_text.clone(),
                        description: s.description.clone(),
                    })
                    .collect(),
            },
            messages: Vec::new(),
        };
        let context_path_abs = context_json_path.clone();
        std::fs::write(&context_path_abs, serde_json::to_vec_pretty(&context)?)?;

        let serialized_context = serialize_task_path(Path::new(self.data_dir()), &context_json_path);
        let serialized_input = serialize_task_path(Path::new(self.data_dir()), &input_cache_path);
        let serialized_audio_dir = serialize_task_path(Path::new(self.data_dir()), &output_audio_dir);

        streaming_task_entity::Entity::insert(streaming_task_entity::ActiveModel {
            id: NotSet,
            history_id: Set(task_id),
            base_model: Set(base_model.clone()),
            model_version: Set(model_version.clone()),
            language: Set(payload.language.as_str().to_string()),
            device: Set(device.as_str().to_string()),
            model_params_json: Set(serde_json::to_string(&model_params)?),
            context_file_path: Set(serialized_context.clone()),
            input_cache_file_path: Set(serialized_input.clone()),
            output_audio_dir: Set(serialized_audio_dir.clone()),
            message_count: Set(0),
            create_time: Set(create_time.clone()),
            modify_time: Set(create_time.clone()),
            deleted: Set(0),
        })
        .exec(&txn)
        .await?;

        txn.commit().await?;

        // 注册会话附加态 + 拉起长期进程
        self.register_streaming_session(task_id);
        if let Err(err) = self.start_streaming_session(base_model.clone(), task_id) {
            error!(error = %err, task_id, "failed to start streaming session");
        }

        Ok(StreamingSpeechTaskResult {
            task_id,
            context_file_path: serialized_context,
            input_cache_file_path: serialized_input,
            output_audio_dir: serialized_audio_dir,
            status: TaskStatus::Pending,
            created_at: create_time,
        })
    }

    pub(crate) async fn send_streaming_message_impl(
        &self,
        payload: SendStreamingMessagePayload,
        on_event: tauri::ipc::Channel<crate::hooks::streaming::AudioStreamEvent>,
    ) -> Result<()> {
        let task_id = payload.task_id;
        let context_id = payload.context_id.clone();
        let history = task_history_entity::Entity::find_by_id(task_id)
            .filter(task_history_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话任务: {task_id}"))?;
        let status = history
            .status
            .parse::<TaskStatus>()
            .map_err(|e| anyhow::anyhow!(e))?;
        if status != TaskStatus::Running {
            bail!("流式会话未在运行（当前状态 {status}），无法发送消息");
        }
        if history.task_type != HistoryTaskType::StreamingSpeech.as_str() {
            bail!("任务 {task_id} 不是流式语音会话");
        }

        let detail = streaming_task_entity::Entity::find()
            .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
            .filter(streaming_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话详情: {task_id}"))?;

        let data_dir = Path::new(self.data_dir());
        let audio_dir = crate::common::local_paths::resolve_task_path(data_dir, &detail.output_audio_dir);
        let context_json_path =
            crate::common::local_paths::resolve_task_path(data_dir, &detail.context_file_path);
        let input_cache_path =
            crate::common::local_paths::resolve_task_path(data_dir, &detail.input_cache_file_path);
        let audio_path =
            crate::common::task_paths::streaming_message_audio_path(&audio_dir, &context_id);
        let serialized_audio_path = serialize_task_path(data_dir, &audio_path);

        // append context.json messages
        let ctx_bytes = std::fs::read(&context_json_path)?;
        let mut ctx: StreamingContextJson = serde_json::from_slice(&ctx_bytes)?;
        ctx.messages.push(StreamingMessageEntry {
            context_id: context_id.clone(),
            speaker_name: payload.speaker_name.clone(),
            text: payload.text.clone(),
            audio_path: serialized_audio_path,
        });
        std::fs::write(&context_json_path, serde_json::to_vec_pretty(&ctx)?)?;

        // append input.jsonl
        let entry_line = crate::service::pipeline::streaming::serialize_input_entry(
            &context_id,
            &payload.speaker_name,
            &payload.text,
            &audio_path.to_string_lossy(),
        );
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&input_cache_path)?;
        use std::io::Write;
        writeln!(file, "{entry_line}")?;

        // 注册 Channel + 等待终帧
        let extra = self
            .streaming_session_extra(task_id)?
            .ok_or_else(|| anyhow::anyhow!("流式会话进程未运行: {task_id}"))?;
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        {
            let mut guard = extra.message_channels.write().map_err(|_| anyhow::anyhow!("会话信道锁损坏"))?;
            guard.insert(context_id.clone(), crate::service::pipeline::streaming::MessageChannel::new(on_event, done_tx));
        }

        let outcome = done_rx
            .await
            .map_err(|_| anyhow::anyhow!("流式会话进程退出，未收到完成信号"))?;

        // 完成则自增 message_count
        if !outcome.cancelled && !outcome.errored {
            let _ = streaming_task_entity::Entity::update_many()
                .col_expr(
                    streaming_task_entity::Column::MessageCount,
                    sea_orm::sea_query::Expr::col(streaming_task_entity::Column::MessageCount).add(1),
                )
                .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
                .exec(self.orm())
                .await;
        }
        if outcome.errored {
            bail!("流式生成失败（contextId={context_id}）");
        }
        Ok(())
    }

    pub(crate) async fn cancel_streaming_task_impl(&self, task_id: i64) -> Result<bool> {
        // 复用既有取消信号路径（watch::Sender -> runner kill child）
        self.request_active_task_cancel(task_id, HistoryTaskType::StreamingSpeech)
    }

    /// 启动清扫：将上次应用退出后残留的 Running 流式会话标记为 Cancelled。
    pub(crate) async fn sweep_stale_streaming_sessions_impl(&self) -> Result<()> {
        let stale = task_history_entity::Entity::find()
            .filter(task_history_entity::Column::TaskType.eq(HistoryTaskType::StreamingSpeech.as_str()))
            .filter(task_history_entity::Column::Status.eq(TaskStatus::Running.as_str()))
            .filter(task_history_entity::Column::Deleted.eq(0))
            .all(self.orm())
            .await?;
        if stale.is_empty() {
            return Ok(());
        }
        info!(count = stale.len(), "sweeping stale streaming sessions");
        let now = now_string()?;
        for record in stale {
            let _ = task_history_entity::Entity::update_many()
                .col_expr(task_history_entity::Column::Status, sea_orm::sea_query::Expr::value(TaskStatus::Cancelled.as_str()))
                .col_expr(task_history_entity::Column::ModifyTime, sea_orm::sea_query::Expr::value(now.clone()))
                .col_expr(task_history_entity::Column::FinishedTime, sea_orm::sea_query::Expr::value(now.clone()))
                .filter(task_history_entity::Column::Id.eq(record.id))
                .exec(self.orm())
                .await;
        }
        Ok(())
    }
}
```

- [ ] **Step 5: 接入 `service/local/mod.rs`**

In `src-tauri/src/service/local/mod.rs`:

(a) 在 `mod voice_design;` 之后加 `mod streaming;`。

(b) `ActiveTaskControl` 增字段（替换 Task 4 占位）。把
```rust
struct ActiveTaskControl {
    task_type: HistoryTaskType,
    cancel_tx: watch::Sender<bool>,
    _cancel_rx_guard: watch::Receiver<bool>,
}
```
改为
```rust
struct ActiveTaskControl {
    task_type: HistoryTaskType,
    cancel_tx: watch::Sender<bool>,
    _cancel_rx_guard: watch::Receiver<bool>,
    streaming_extra: Option<Arc<crate::service::pipeline::streaming::StreamingSessionExtra>>,
}
```
（`use std::sync::Arc;` 已在文件顶部。）

`register_active_task_control` 保持原有签名；新会话用 `register_streaming_session` 单独注入 `streaming_extra`。

(c) 新增辅助方法（Task 4 经预检重构后无占位需替换）。在 `impl LocalService { ... }` 内（`request_active_task_cancel` 之后）新增：

```rust
    pub(crate) fn register_streaming_session(&self, task_id: i64) {
        let extra = Arc::new(crate::service::pipeline::streaming::StreamingSessionExtra::default());
        if let Ok(mut controls) = self.active_task_controls.write() {
            let cancel_tx = {
                let (cancel_tx, cancel_rx_guard) = watch::channel(false);
                controls.insert(
                    task_id,
                    ActiveTaskControl {
                        task_type: HistoryTaskType::StreamingSpeech,
                        cancel_tx,
                        _cancel_rx_guard: cancel_rx_guard,
                        streaming_extra: Some(extra.clone()),
                    },
                );
            };
            let _ = cancel_tx;
        }
    }

    pub(crate) fn streaming_session_extra(
        &self,
        task_id: i64,
    ) -> Result<Option<Arc<crate::service::pipeline::streaming::StreamingSessionExtra>>> {
        let controls = self
            .active_task_controls
            .read()
            .map_err(|_| anyhow::anyhow!("无法读取运行中任务句柄"))?;
        Ok(controls.get(&task_id).and_then(|c| c.streaming_extra.clone()))
    }

    pub(crate) async fn load_streaming_task_detail(
        &self,
        task_id: i64,
    ) -> Result<crate::service::pipeline::streaming::LoadedStreamingDetail> {
        use crate::service::local::entity::streaming_task as streaming_task_entity;
        let detail = streaming_task_entity::Entity::find()
            .filter(streaming_task_entity::Column::HistoryId.eq(task_id))
            .filter(streaming_task_entity::Column::Deleted.eq(0))
            .one(self.orm())
            .await?
            .ok_or_else(|| anyhow::anyhow!("未找到流式会话详情: {task_id}"))?;
        let speakers: Vec<crate::service::models::StreamingSpeakerInput> =
            serde_json::from_str::<crate::service::pipeline::streaming::StreamingContextJson>(
                &std::fs::read_to_string(
                    crate::common::local_paths::resolve_task_path(
                        Path::new(self.data_dir()),
                        &detail.context_file_path,
                    ),
                )?,
            )?
            .basic
            .speakers
            .into_iter()
            .map(|s| crate::service::models::StreamingSpeakerInput {
                name: s.name,
                base_model: s.base_model.parse().unwrap_or(crate::config::BaseModel::default()),
                model_version: s.model_version,
                ref_audio_path: s.ref_audio_path,
                ref_audio_name: s.ref_audio_name,
                ref_text: s.ref_text,
                description: s.description,
            })
            .collect();
        Ok(crate::service::pipeline::streaming::LoadedStreamingDetail {
            base_model: detail.base_model,
            model_version: detail.model_version,
            device: detail.device.parse().unwrap_or(HardwareType::Cpu),
            speakers,
        })
    }

    pub(crate) fn start_streaming_session(
        &self,
        base_model: crate::config::BaseModel,
        task_id: i64,
    ) -> Result<()> {
        let service = self.clone();
        tauri::async_runtime::spawn(async move {
            let result = async {
                let detail = service.load_streaming_task_detail(task_id).await?;
                let extra = service
                    .streaming_session_extra(task_id)?
                    .ok_or_else(|| anyhow::anyhow!("streaming session extra not registered for task {task_id}"))?;
                run_streaming_session(
                    &service,
                    StreamingPipelineRequest { task_id },
                    &base_model,
                    detail,
                    extra,
                )
                .await
            }
            .await;
            service.unregister_active_task_control(task_id);
            if let Err(err) = result {
                tracing::error!(error = %err, "local streaming session failed");
            }
        });
        Ok(())
    }
```

> `BaseModel = String`（`config/mod.rs:34`）。`start_streaming_session` 保留 `base_model` 参数，与其它 4 个 `start_*_inference` 同形；`run_streaming_session` 用其做 `detail.base_model` 一致性校验（仿 `run_*_pipeline` 的 mismatch 检查）。`model_version` / `device` / `speakers` 仍从 `load_streaming_task_detail` 取。模型环境准备（`prepare_*_model_env` 对齐：模型已下载校验 + Torch 运行时）为后续接入点，届时复用传入的 `base_model` + `detail.model_version`。

(d) `init_db` 末尾（`sync_supported_models` 之后）调用 sweep：
```rust
        supported_models::sync_supported_models(orm).await.map_err(|e| {
            anyhow::anyhow!("failed to sync model config catalog into local database in {}: {}", data_dir.display(), e)
        })?;

        // 清扫上次应用退出后残留的流式会话 Running 行
        if let Err(err) = sweep_stale_streaming_sessions_impl_on_orm(orm).await {
            tracing::warn!(error = %err, "failed to sweep stale streaming sessions during init");
        }
```
新增独立函数（不依赖 `&self`，仅用 `orm`）：
```rust
async fn sweep_stale_streaming_sessions_impl_on_orm(orm: &DatabaseConnection) -> Result<()> {
    use crate::service::local::entity::task_history as task_history_entity;
    use sea_orm::sea_query::Expr;
    let stale = task_history_entity::Entity::find()
        .filter(task_history_entity::Column::TaskType.eq(HistoryTaskType::StreamingSpeech.as_str()))
        .filter(task_history_entity::Column::Status.eq(TaskStatus::Running.as_str()))
        .filter(task_history_entity::Column::Deleted.eq(0))
        .all(orm)
        .await?;
    if stale.is_empty() { return Ok(()); }
    let now = crate::utils::time::now_string()?;
    for record in stale {
        let _ = task_history_entity::Entity::update_many()
            .col_expr(task_history_entity::Column::Status, Expr::value(TaskStatus::Cancelled.as_str()))
            .col_expr(task_history_entity::Column::ModifyTime, Expr::value(now.clone()))
            .col_expr(task_history_entity::Column::FinishedTime, Expr::value(now.clone()))
            .filter(task_history_entity::Column::Id.eq(record.id))
            .exec(orm).await;
    }
    Ok(())
}
```
（`LocalService::sweep_stale_streaming_sessions_impl` 内部可直接委托给该自由函数以复用逻辑；测试通过 harness helper 调用 `&self` 版本。）

(e) `Service` trait impl 块内新增 3 方法委托：
```rust
    async fn create_streaming_speech_task(
        &self,
        payload: crate::service::models::CreateStreamingSpeechTaskPayload,
    ) -> Result<crate::service::models::StreamingSpeechTaskResult> {
        self.create_streaming_speech_task_impl(payload).await
    }

    async fn send_streaming_message(
        &self,
        payload: crate::service::models::SendStreamingMessagePayload,
        on_event: tauri::ipc::Channel<crate::hooks::streaming::AudioStreamEvent>,
    ) -> Result<()> {
        self.send_streaming_message_impl(payload, on_event).await
    }

    async fn cancel_streaming_task(&self, task_id: i64) -> Result<bool> {
        self.cancel_streaming_task_impl(task_id).await
    }
```

- [ ] **Step 6: `Service` trait 签名 + Remote stub**

In `src-tauri/src/service/mod.rs`：`Service` trait 末尾（`create_voice_design_task` 之后）加：
```rust
    async fn create_streaming_speech_task(
        &self,
        payload: crate::service::models::CreateStreamingSpeechTaskPayload,
    ) -> Result<crate::service::models::StreamingSpeechTaskResult>;
    async fn send_streaming_message(
        &self,
        payload: crate::service::models::SendStreamingMessagePayload,
        on_event: tauri::ipc::Channel<crate::hooks::streaming::AudioStreamEvent>,
    ) -> Result<()>;
    async fn cancel_streaming_task(&self, task_id: i64) -> Result<bool>;
```
并在文件顶部 `use crate::service::models::{...}` 中补 `CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload, StreamingSpeechTaskResult`。

In `src-tauri/src/service/remote/mod.rs`：`impl Service for RemoteService` 末尾加：
```rust
    async fn create_streaming_speech_task(
        &self,
        _payload: crate::service::models::CreateStreamingSpeechTaskPayload,
    ) -> Result<crate::service::models::StreamingSpeechTaskResult> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn send_streaming_message(
        &self,
        _payload: crate::service::models::SendStreamingMessagePayload,
        _on_event: tauri::ipc::Channel<crate::hooks::streaming::AudioStreamEvent>,
    ) -> Result<()> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }

    async fn cancel_streaming_task(&self, _task_id: i64) -> Result<bool> {
        anyhow::bail!("远程存储模式暂不支持流式语音会话")
    }
```

- [ ] **Step 7: `MessageChannel::new` 构造 + harness sweep helper**

In `src-tauri/src/service/pipeline/streaming.rs`，给 `MessageChannel` 加构造：
```rust
impl MessageChannel {
    pub(crate) fn new(
        on_event: Channel<AudioStreamEvent>,
        done: oneshot::Sender<StreamMessageOutcome>,
    ) -> Self {
        Self { on_event, done }
    }
}
```

In `src-tauri/src/test_support.rs`，`impl LocalServiceHarness` 内新增：
```rust
    pub async fn sweep_stale_streaming_sessions(&self) -> Result<()> {
        // test_support 在 crate 内，可直接调用 pub(crate) 的真实实现
        self.service.sweep_stale_streaming_sessions_impl().await
    }
```
> 该 helper 直接调用真实 `LocalService::sweep_stale_streaming_sessions_impl`（`test_support` 在 crate 内可访 `pub(crate)`），测试覆盖真实清扫逻辑。

- [ ] **Step 8: 运行 sweep 测试 + 全量编译**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test streaming_service` && `cargo build --manifest-path src-tauri/Cargo.toml -p kirine-client`
Expected: sweep 测试 PASS；编译通过。

- [ ] **Step 9: 提交**

```bash
git add src-tauri/src/service/local/entity/streaming_task.rs src-tauri/src/service/local/entity/mod.rs src-tauri/src/service/local/streaming.rs src-tauri/src/service/local/mod.rs src-tauri/src/service/mod.rs src-tauri/src/service/remote/mod.rs src-tauri/src/service/pipeline/streaming.rs src-tauri/src/test_support.rs src-tauri/tests/streaming_service.rs
git commit -m "feat(streaming): entity + service 层（create/send/cancel/sweep）+ Service trait"
```

---

## Task 6: Hooks 命令 + 注册

**Files:**
- Modify: `src-tauri/src/hooks/streaming.rs`（新增 3 命令）
- Modify: `src-tauri/src/hooks/mod.rs`（`load_hooks` 注册）

**Interfaces:**
- Produces: `create_streaming_speech_task` / `send_streaming_message` / `cancel_streaming_task` 三个 `#[tauri::command]`。

- [ ] **Step 1: 写 hooks 命令**

In `src-tauri/src/hooks/streaming.rs` 顶部加 `use tauri::{ipc::Channel, State};` 与 `use crate::service::{models::{CreateStreamingSpeechTaskPayload, SendStreamingMessagePayload}, ServiceState};`。在 `stream_audio_placeholder` 之后追加：

```rust
#[tauri::command]
pub async fn create_streaming_speech_task(
    payload: CreateStreamingSpeechTaskPayload,
    state: State<'_, ServiceState>,
) -> std::result::Result<crate::service::models::StreamingSpeechTaskResult, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .create_streaming_speech_task(payload)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn send_streaming_message(
    payload: SendStreamingMessagePayload,
    on_event: Channel<AudioStreamEvent>,
    state: State<'_, ServiceState>,
) -> std::result::Result<(), String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .send_streaming_message(payload, on_event)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn cancel_streaming_task(
    task_id: i64,
    state: State<'_, ServiceState>,
) -> std::result::Result<bool, String> {
    state
        .0
        .service()
        .map_err(|err| err.to_string())?
        .cancel_streaming_task(task_id)
        .await
        .map_err(|err| err.to_string())
}
```

- [ ] **Step 2: 注册命令**

In `src-tauri/src/hooks/mod.rs`，`load_hooks` 的 `generate_handler!` 中，`streaming::stream_audio_placeholder,` 之后加：
```rust
        streaming::create_streaming_speech_task,
        streaming::send_streaming_message,
        streaming::cancel_streaming_task,
```

- [ ] **Step 3: 编译**

Run: `cargo build --manifest-path src-tauri/Cargo.toml -p kirine-client`
Expected: 编译通过。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/hooks/streaming.rs src-tauri/src/hooks/mod.rs
git commit -m "feat(streaming): 注册 create/send/cancel 三个 hooks 命令"
```

---

## Task 7: 前端接入（store / hook / 组件 / 视图）

**Files:**
- Modify: `src/types/streaming.ts`（`StreamingChatMessage` 增 `synthText`）
- Modify: `src/stores/streamingSpeech.ts`（`sendMessage` 异步化 + 会话管理 + `terminateSession`）
- Modify: `src/hooks/useStreamableAudioPlayer.ts`（`startStreaming` 改调 `send_streaming_message`）
- Modify: `src/components/common/StreamableAudioPlayer.vue`（透传 `speakerName` / `synthText`）
- Modify: `src/views/StreamingSpeechView.vue`（透传 + 终止按钮）

**Interfaces:**
- Produces: 前端 `sendMessage` 在首条消息时 `invoke('create_streaming_speech_task')` 拿 `taskId` 回填；`useStreamableAudioPlayer.startStreaming(taskId, contextId, speakerName, text)` 调 `send_streaming_message`；视图「终止会话」按钮调 `cancel_streaming_task`。

- [ ] **Step 1: `StreamingChatMessage` 增 `synthText`**

Modify `src/types/streaming.ts`，在 `StreamingChatMessage` 内 `text: string;` 之后加：
```ts
  synthText: string;   // 该轮待合成的文本（assistant 消息携带，传给 send_streaming_message）
```

- [ ] **Step 2: store 改造**

Modify `src/stores/streamingSpeech.ts`:

顶部加 `import { invoke } from '@tauri-apps/api/core';` 与类型 import：
```ts
import type { StreamingSpeechTaskResult } from '@/types/domain';
```
（若 `domain.ts` 尚无 `StreamingSpeechTaskResult`，新增：
```ts
export interface StreamingSpeechTaskResult {
  taskId: number;
  contextFilePath: string;
  inputCacheFilePath: string;
  outputAudioDir: string;
  status: string;
  createdAt: string;
}
export interface SendStreamingMessagePayload {
  taskId: number;
  contextId: string;
  speakerName: string;
  text: string;
}
export interface CreateStreamingSpeechTaskPayload {
  baseModel: string;
  modelVersion: string;
  device: string;
  language: import('@/enums/language').AppLanguage;
  modelParams: Record<string, unknown>;
  speakers: { name: string; baseModel: string; modelVersion?: string; refAudioPath: string; refAudioName: string; refText: string; description?: string }[];
}
```
）

在 store 内新增 state：
```ts
  const activeTaskId = ref<number | null>(null);
  const isStartingSession = ref(false);
```

替换 `sendMessage` 为异步：
```ts
  const sendMessage = async (text: string, speakerId: string | null) => {
    const trimmed = text.trim();
    if (trimmed.length === 0) {
      return;
    }
    const speaker = getSpeaker(speakerId);

    // 首条消息：建会话拿 taskId
    if (activeTaskId.value === null) {
      if (speakers.value.length === 0) {
        return;
      }
      isStartingSession.value = true;
      try {
        const result = await invoke<StreamingSpeechTaskResult>('create_streaming_speech_task', {
          payload: {
            baseModel: sessionConfig.baseModel,
            modelVersion: sessionConfig.modelVersion,
            device: sessionConfig.device,
            language: sessionConfig.language,
            modelParams: sessionConfig.modelParams,
            speakers: speakers.value.map(s => ({
              name: s.name,
              baseModel: s.baseModel,
              modelVersion: s.modelVersion,
              refAudioPath: s.refAudioPath,
              refAudioName: s.refAudioName,
              refText: s.refText,
              description: s.description
            }))
          }
        });
        activeTaskId.value = result.taskId;
      } finally {
        isStartingSession.value = false;
      }
    }

    const taskId = activeTaskId.value;
    if (taskId === null) {
      return;
    }

    const userMessage: StreamingChatMessage = {
      id: nextMessageId(),
      role: 'user',
      text: trimmed,
      synthText: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId,
      contextId: '',
      status: 'completed'
    };
    messages.value = [...messages.value, userMessage];

    const assistantId = nextMessageId();
    const assistantMessage: StreamingChatMessage = {
      id: assistantId,
      role: 'assistant',
      text: '',
      synthText: trimmed,
      speakerId,
      speakerName: speaker?.name,
      taskId,
      contextId: assistantId,
      status: 'streaming'
    };
    messages.value = [...messages.value, assistantMessage];
  };

  const terminateSession = async () => {
    const taskId = activeTaskId.value;
    if (taskId === null) {
      return;
    }
    try {
      await invoke('cancel_streaming_task', { taskId });
    } finally {
      activeTaskId.value = null;
      messages.value = messages.value.map(m => (m.status === 'streaming' ? { ...m, status: 'error' } : m));
    }
  };
```

`clearMessages` 末尾加 `activeTaskId.value = null;`。

return 中追加暴露：`activeTaskId, isStartingSession, terminateSession`。

- [ ] **Step 3: `useStreamableAudioPlayer` 改调新命令**

Modify `src/hooks/useStreamableAudioPlayer.ts`，把 `startStreaming` 改为：
```ts
  const startStreaming = async (taskId: number, contextId: string, speakerName: string, text: string) => {
    accumulated = [];
    hasData.value = false;
    streamComplete.value = false;
    isStreaming.value = true;
    destroyAudioElement();
    releaseObjectUrl();
    pathUrl = null;

    const channel = new Channel<AudioStreamEvent>();
    channel.onmessage = (message: AudioStreamEvent) => {
      switch (message.type) {
        case 'started':
          break;
        case 'chunk':
          accumulated.push(...message.bytes);
          if (!hasData.value) {
            hasData.value = true;
          }
          break;
        case 'finished':
          streamComplete.value = true;
          isStreaming.value = false;
          break;
        case 'error':
          isStreaming.value = false;
          options.onStreamError?.(message.message);
          break;
      }
    };

    try {
      await invoke('send_streaming_message', {
        payload: { taskId, contextId, speakerName, text },
        onEvent: channel
      });
    } catch (error) {
      isStreaming.value = false;
      options.onStreamError?.(error instanceof Error ? error.message : String(error));
    }
  };
```

- [ ] **Step 4: `StreamableAudioPlayer` 透传 props**

Modify `src/components/common/StreamableAudioPlayer.vue`。`Props` 增字段：
```ts
interface Props {
  mode: 'stream' | 'path';
  taskId?: number;
  contextId?: string;
  audioPath?: string;
  speakerName?: string;
  synthText?: string;
}
```
`withDefaults` 补 `speakerName: '', synthText: ''`。

`mode === 'stream'` 的 watch 改为：
```ts
  watch(
    () => [props.taskId, props.contextId, props.speakerName, props.synthText] as const,
    ([taskId, contextId, speakerName, synthText]) => {
      if (taskId != null) {
        void startStreaming(taskId, contextId, speakerName, synthText);
      }
    },
    { immediate: true },
  );
```

- [ ] **Step 5: 视图透传 + 终止按钮**

Modify `src/views/StreamingSpeechView.vue`。

assistant 消息的 `StreamableAudioPlayer` 改为：
```vue
<StreamableAudioPlayer
  mode="stream"
  :task-id="message.taskId"
  :context-id="message.contextId"
  :speaker-name="message.speakerName ?? ''"
  :synth-text="message.synthText"
/>
```

头部按钮区在「清空对话」之前加「终止会话」：
```vue
<BaseButton
  tone="ghost"
  size="sm"
  :disabled="store.activeTaskId === null"
  @click="store.terminateSession"
>
  <StopCircleIcon class="h-4 w-4" aria-hidden="true" />
  <span>终止会话</span>
</BaseButton>
```
import 增 `StopCircleIcon`：
```ts
import { Cog6ToothIcon, PaperAirplaneIcon, StopCircleIcon, TrashIcon } from '@heroicons/vue/24/outline';
```

- [ ] **Step 6: 类型检查 + 构建**

Run: `npm run build`（或 `pnpm build` / `yarn build`，按项目实际）
Expected: 类型检查通过、构建成功。

- [ ] **Step 7: 提交**

```bash
git add src/types/streaming.ts src/stores/streamingSpeech.ts src/hooks/useStreamableAudioPlayer.ts src/components/common/StreamableAudioPlayer.vue src/views/StreamingSpeechView.vue src/types/domain.ts
git commit -m "feat(streaming): 前端接入 create/send/cancel 流式命令"
```

---

## Task 8: 手动验证（待脚本侧就绪）

> 本期 Rust + 前端完工，但真实 `streaming.py` 与 `begin_llm_task` stdout 透传改造不在范围，故端到端流式播放待脚本侧就绪后验证。以下为脚本侧就绪后的验证清单。

- [ ] **Step 1: 启动应用，进入「流式语音」页**

Run: `npm run tauri dev`
Expected: 侧边栏「流式语音」入口可点开聊天页。

- [ ] **Step 2: 配置抽屉添加说话人 + 基础配置，发送首条消息**

操作：打开配置抽屉 -> 添加一个语音克隆式说话人（参考音频 + 参考文本）-> 关闭抽屉 -> 输入文本回车。
Expected: `create_streaming_speech_task` 被调用，DB 出现一行 `task_history`(streaming-speech, pending->running) + `streaming_tasks` 行；会话目录下生成 `context.json`（含 basic + 空 messages）。

- [ ] **Step 3: 验证流式播放**

Expected: assistant 气泡出现 `StreamableAudioPlayer`，`send_streaming_message` 经 Channel 收到 started/chunk/finished，可实时播放；`context.json` messages 追加该条（含 audioPath）；`input.jsonl` 追加一行；`output_audio_dir` 生成 `<contextId>.wav`；`message_count` 自增。

- [ ] **Step 4: 验证终止会话**

点击「终止会话」。
Expected: `cancel_streaming_task` -> 进程被 kill；`task_history.status` -> cancelled；前端 `activeTaskId` 清空，streaming 消息置 error。

- [ ] **Step 5: 验证应用重启清扫**

发送消息后会话 Running -> 关闭应用 -> 重新启动。
Expected: 启动后该会话 `status` 被清扫为 cancelled（sweep）；历史列表可见该会话行。

- [ ] **Step 6: 回归既有流水线**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: 全部测试 PASS（含新增 streaming_schema / streaming_service / pipeline_params streaming 用例）。

---

## Self-Review 记录

- **Spec 覆盖**：§2 DB -> Task 1；§3 文件契约 -> Task 2（ser-de）+ Task 5（写 context.json/input.jsonl）；§4 分帧协议 -> Task 2（parse_streaming_frame）+ Task 4（runner 分发）；§5 Rust 模块 -> Task 1-6；§6 生命周期+取消 -> Task 4（cancel kill）+ Task 5（sweep/状态机）；§7 前端 -> Task 7；§8 测试 -> 各任务 TDD + Task 8 手动；§9 不在范围 -> 已在 Global Constraints 声明。无遗漏。
- **占位符扫描**：Task 4 经预检重构后无占位方法（`detail`/`extra` 以参数传入 `run_streaming_session`）；无 TBD/TODO。
- **类型一致**：`StreamingFrame` / `StreamingContextJson` / `StreamingSessionExtra` / `MessageChannel` / `LoadedStreamingDetail` / `StreamMessageOutcome` 在 Task 2/4 定义、Task 4/5 消费，命名一致；`build_llm_task_script_args` 复用既有 `pub` 纯函数。
- **已确认/已解决**：`BaseModel = String`（`config/mod.rs:34`），`as_str`/`Default`/`parse` 均可用；`run_streaming_session` 保留 `base_model` 参数（与其它 4 个 `start_*_inference` 同形），用于 `detail.base_model` 一致性校验；`Channel::send` 取 `&self`，`forward_event` 无需 `Clone`。**剩余留意点**：`base64` 依赖需在 `Cargo.toml` 确认（Task 2 Step 4）；`oneshot`/`watch` 已在既有依赖中；模型环境准备（`prepare_*_model_env`）为后续接入点。
