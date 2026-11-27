# Kirine Client

桌面端 AI 语音合成客户端：**Tauri 2（Rust）+ Vue 3（TypeScript）**，按 `src-model/` 下的 Git 子模块适配器调度本地 Python 模型运行时。

> 当前状态（2026-11-22，分支/版本 `v0.15.0`）：Local 模式完整可用；Remote 模式仍是占位实现（`client::ApiClient` 所有方法 `bail("HTTP 调用尚未接入")`），流式语音仅 Local 支持。

## 1. 工具与工作方式

### MCP：code-review-graph（代码探索优先）

本仓库有代码知识图谱（`.code-review-graph/`，约 250 文件 / 3600 节点 / 1.8 万边）。**探索代码时先用图工具，再退回 Grep/Glob/Read**：

| 场景 | 工具 |
| --- | --- |
| 找调用者/被调用者/引用/导入/测试覆盖 | `query_graph_tool`（`callers_of` / `callees_of` / `references_to` / `importers_of` / `tests_for`） |
| 改动的风险与影响面 | `detect_changes_tool`、`get_impact_radius_tool`、`get_affected_flows_tool` |
| 需要源码片段做审查 | `get_review_context_tool` |
| 按名称/语义找符号 | `semantic_search_nodes_tool` |
| 架构与模块边界 | `get_architecture_overview_tool`、`list_communities_tool` |
| 执行流 | `list_flows_tool`、`get_flow_tool` |

- 图由 hooks 在文件变更后自动更新；结构与调用关系以图为准，字符串/日志/配置键等文本仍用 Grep。
- GitHub MCP（`github` / `github-mcp`）已配置但当前 Authorization 头被 401 拒绝；需要 GitHub 操作时改用 `git` / `gh` CLI。

### Skills

- `superpowers:*`：`brainstorming`（新功能/新方案前）、`writing-plans` / `executing-plans`（多步任务）、`systematic-debugging`（任何 bug/测试失败前）、`test-driven-development`、`requesting-code-review` / `receiving-code-review`、`verification-before-completion`、`using-git-worktrees`、`finishing-a-development-branch`。
- `claude-md-management:*`：本文件的审计与更新。
- 常用命令：`/code-review`、`/simplify`、`/security-review`、`/init`、`/run`。
- 项目 `.claude/settings.local.json` 已放行 code-review-graph 图构建/检索、`git submodule *`、GitHub 文件读取等权限，并启用 `code-review-graph` MCP server。

## 2. 技术栈

| 层 | 技术 / 版本 | 说明 |
| --- | --- | --- |
| 前端 | Vue 3.5、TypeScript 5.6（strict）、Vite 6 | `@` 别名指向 `src/`；Vite 端口固定 3333 |
| 路由 / 状态 | Vue Router 5（hash 模式）、Pinia 3 | 10 个主路由 + 重定向 + 404 |
| UI | TailwindCSS 3.4、Headless UI、Heroicons | 组件集中在 `src/components/` |
| 国际化 | vue-i18n 11（Composition API） | `legacy:false`，`fallbackLocale: zh-CN` |
| 桌面壳 | Tauri 2.11 + dialog/opener 插件 | 前端 `invoke<T>`、`Channel`、`listen` |
| 后端 | Rust 2021、Tokio、anyhow、serde | `src-tauri/` 单 crate |
| 数据库 | SeaORM 1.1 + sea-orm-migration、SQLite | 本地 `data_dir/app.db`，schema version **31** |
| HTTP（预留） | reqwest、`utils::HttpClient` | Remote 模式基础设施，尚未启用 |
| Python | 3.12，每模型独立 `venv` 或 `conda_env` | 见 `src-model/README.md` |
| 外部工具 | ffmpeg 8.1.2 shared build、SoX 14.4.2 | 随包分发，不写系统 PATH |

## 3. 系统架构

```text
Vue 3（views / components / stores / hooks / locales）
   │  invoke<T>(command, payload)      Tauri 命令（驼峰 payload）
   │  Channel<InvokeResponseBody>      流式音频/控制事件
   │  listen('history:task-updated')   任务状态失效事件
   ▼
src-tauri/src/hooks/                  命令边界：参数校验、AppError 转换
   ▼
service/Service trait ── ServiceImpl::Local / Remote（按 config.toml [basic].mode 分发）
   │                        9 个业务域接口：speaker/model/history/generated_audio/
   │                        tts/training/voice_clone/voice_design/streaming
   ├── local/            SeaORM SQLite + 任务表 + 运行句柄
   └── remote/           ApiClient 占位（流式全部 bail）
   ▼
service/pipeline/                    一次性任务：参数文件 → 包装脚本 → Python
   ▼
src-model/scripts/{windows,unix}/    init / ensure_torch / download / begin_llm_task / transcode
   ▼
src-model/<base_model>/              适配器：configs/*.json + params*.py + 任务入口 .py
```

四条贯穿全局的设计：

1. **配置驱动**：模型元数据、功能矩阵、语言、设备与前端参数表单全部来自适配器的 `configs/model-config.json` 与 `configs/params-config.json`。Rust 启动时扫描并同步进数据库；不在 Rust/前端硬编码模型能力。
2. **接口隔离**：`Service` trait 只保留生命周期 + 9 个域访问方法；Local/Remote 分别实现同一组域接口，hooks 通过 `service().xxx_service()` 调用。
3. **参数文件契约**：Rust 将 `PythonScriptInvocationSpec` 原子写入 `--params-file` JSON，由 `begin_llm_task.{ps1,sh}` 选择 Python 运行时并执行 `<task>.py`；三层只以 JSON 键为契约。
4. **单一音频来源**：`GeneratedAudioSource`（一次性任务用 `historyId`，流式用 `historyId + messageId`）是读取、路径定位、另存为的唯一索引，不依赖活动会话或 WebView 本地文件 URL。

## 4. 目录结构

```text
kirine-client/
├── package.json                 # 前端脚本：dev/build/preview/tauri
├── config.toml                  # 运行时配置（basic/remote/training/streaming）
├── db/
│   ├── tables.sql               # SQLite schema 真值（迁移后的最终态）
│   └── tables_pgsql.sql         # PostgreSQL 镜像，结构须一一对应
├── docs/
│   ├── remote-api.yaml          # Remote API OpenAPI 契约（手工维护）
│   ├── memory/                  # 历史上下文资料（非权威，代码为准）
│   └── superpowers/             # 本地-only，不纳入 git
├── src/                         # Vue 前端
│   ├── components/
│   │   ├── common/              # BaseDialog、BaseListbox、播放器、分页、通知等
│   │   ├── form/                # GenericTaskParamsForm、各任务详情表单
│   │   ├── history/             # 历史详情对话框
│   │   ├── speaker/             # 说话人头像
│   │   ├── streaming/           # 流式聊天、配置抽屉、说话人表单
│   │   └── ui/                  # 7 种配置驱动参数组件
│   ├── enums/                   # task/status/settings/language/taskType 等
│   ├── hooks/                   # 分页、轮询恢复、任务事件、音频播放、设备守卫
│   ├── locales/{zh-CN,en-US}/   # 按模块拆分的词条
│   ├── routers/                 # 路由表（streaming 路径硬编码）
│   ├── stores/                  # models/speakers/ui/uiConfig/streamingSpeech
│   ├── types/                   # domain.ts / streaming.ts / uiConfig.ts
│   ├── utils/                   # 音频下载、文件名、参数转换
│   └── views/                   # 10 个页面（4 任务 + 流式 + 模型/说话人/历史/设置/404）
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs               # 启动：WebView2 检测、cwd 锚定、init_service
│   │   ├── hooks/               # 36 个 Tauri 命令（model/speaker/task/streaming/settings）
│   │   ├── config/              # EnvConfig、模型目录、UI 配置、日志
│   │   ├── client/              # ApiClient（远端占位）+ 路径/实体
│   │   ├── common/              # local_paths、task_paths
│   │   ├── migration/           # SeaORM 迁移，schema 31
│   │   ├── service/
│   │   │   ├── models.rs        # 全部业务枚举/结构体/分页/音频来源
│   │   │   ├── local/           # entity + 各域实现 + 流式会话句柄
│   │   │   ├── remote/          # 各域占位实现
│   │   │   └── pipeline/        # 参数文件、路径解析、任务管线、流式 runner
│   │   ├── utils/               # 进程、PATH、音频、文件、时间、HTTP
│   │   └── test_support.rs      # tests/ 的公共重导出桥
│   ├── tests/                   # 集成测试（17 个文件，不用内联 #[cfg(test)]）
│   ├── resources/               # 打包资源：config.toml、ffmpeg/sox/src-model zip
│   ├── windows/prepare-dependencies/  # NSIS 安装钩子
│   └── scripts/make-portable.ps1     # 绿色免安装包
└── src-model/
    ├── README.md                # 运行时总览（venv/conda、脚本动作、职责边界）
    ├── ADAPTER_DEVELOPMENT.md   # 适配器开发契约（新增模型必读）
    ├── base-models/             # 基础权重/离线资产
    ├── scripts/{windows,unix}/  # 平台脚本
    └── <adapter>/               # 8 个 Git 子模块，见 §6.1
```

## 5. 核心功能逻辑链路

### 5.1 启动与初始化

`lib.rs::run` → WebView2 检测（缺失弹原生框）→ 发布构建将 cwd 锚定到 exe 目录 → `init`：

1. 读取 `config.toml`（相对 `.` / `..` 解析），写入 `EnvConfigState`。
2. 扫描 `src-model/*/configs/*.json`，写入 `UiConfigState`。
3. 初始化 tracing 日志到 `log_dir`。
4. `init_service(config)` 按 `[basic].mode` 构造 `LocalService` 或 `RemoteService`。
5. Local：建目录 → 打开 `data_dir/app.db` → 跑迁移 → 同步模型元数据 → 清扫残留的 Running 流式会话（置 Cancelled）。
6. 注册 `history:task-updated` 事件通道；为 `data_dir` 追加 asset 协议递归授权。
7. 应用退出 `RunEvent::Exit` → `LocalService::close()`：取消所有运行任务 + 按登记的流式 PID 整树强杀。

### 5.2 模型管理

**列表/设备**：`list_model_infos`（分页）→ 读取 DB 中的模型元数据；`get_device_type` → `ensure_torch_runtime --query-device-type` 探测当前模型环境，Rust 只解析脚本输出。

**安装**：`install_model(modelId, device)` → 校验 `device ∈ supportedDevices` → `validate_and_init`（`init_task_runtime` 创建/复用 `conda_env` 或 `venv`、装基础依赖）→ 若 `downloaded` 则跳过 → `HF-Like` 用 `download_models` 按 repo 清单拉取；`Custom` 调适配器 `download.py` → 校验产物 → 标记 `downloaded`。卸载清理模型产物与可重建运行时。

**当前设备**：`model_info.current_device` 持久化用户选择；单设备模型 sync 时自动回填，多设备初始为空；非法/失效值归一为 `None`。单设备自动选中，多设备须选后才能安装。

### 5.3 一次性任务：TTS / 声音克隆 / 音色设计

三者共用同一执行骨架，仅参数结构与入口脚本不同。

```text
前端 View 收集参数（通用表单 + 任务详情）
  → invoke create_xxx_task(payload)
LocalService：
  1. 校验模型存在、device ∈ supportedDevices、说话人 Ready 且 baseModel 匹配
  2. 事务写 task_history(Pending) + tts/voice_clone/voice_design_tasks
  3. 复制模板/参考文件到任务 sample 目录，序列化输出路径
  4. emit history:task-updated(created)
  5. spawn Pipeline → 立即返回 taskId
Pipeline：
  mark Running → 读取任务参数 → ensure_torch_runtime（必要时切 CPU/CUDA）
  → 校验模型已安装 → 构造 PythonScriptInvocationSpec
  → 原子写 params.json → begin_llm_task → <tts|voice_clone|voice_design>.py
  → 产出临时 WAV → transcode_audio 转目标格式 → Completed / Failed / Cancelled
前端：
  事件驱动刷新 + 30s 低频轮询兜底；结果音频经 get_generated_audio_path 取绝对路径，
  convertFileSrc 走 asset 协议零拷贝播放；导出统一 save_generated_audio_as。
```

- 设备不一致时，前端 `useTaskDeviceTypeGuard` 先弹确认；确认后由 Pipeline 的 `ensure_torch_runtime` 实际切换。
- 任务取消走 `watch::Sender<bool>` → `run_logged_shell_script_cancellable`；取消后子进程链整体回收。

### 5.4 模型训练

1. 创建：校验设备 → 事务写 `speakers`（status=Training）+ `task_history` + `model_training_tasks`。
2. 数据准备（大文件拷贝/解压，调度到阻塞线程池）：
   - 单样本：音频 + 台词；
   - 数据集：ZIP + 标注文件（`jsonl` / `xlsx` / `xls`）；
   - 产物：任务 sample 目录下的音频副本、`input.jsonl`、参考音频。
3. Pipeline：`ensure_torch_runtime` → 训练音频规范化（并发 spawn `transcode_audio`，并发数与失败率阈值见 `config.toml [training]`）→ 构造训练参数（epochs/batch/accumulation/lr 等）→ `training.py`。
4. 产物写到 `<model_dir>/<speaker_id>/`；成功后说话人置 Ready、更新样本数；失败置 Failed。

### 5.5 流式语音（会话级长期进程）

与一次性任务不同：**一个 task 一个常驻进程**，多条消息按 `contextId` 复用。

```text
create_streaming_speech_task
  → 校验模型/设备/说话人（一会话至多一个 trained 说话人）
  → 事务写 task_history + streaming_tasks
  → 复制模型参数文件、写初始 context.json
  → register_streaming_session + spawn 长期 runner
  → 返回 taskId 与三类路径

runner（run_streaming_session）
  → begin_llm_task → streaming.py
  → bind 127.0.0.1 随机端口 + 首帧 AUTH 令牌校验
  → reader/writer task 分帧读写；drive 循环 select!(frame, cancel, 2s 探活)
  → 进程退出 / cancel → 置 Cancelled 或 Failed；终态清理所有 Channel

send_streaming_message
  → 校验 Running → 先注册 Channel（防终帧早到）
  → context_lock 串行 read-modify-write context.json + append input.jsonl（审计）
  → 经 input_tx 推 INPUT 0x10 帧 → 等待 done 终帧
  → 帧间静默超时（默认 180s，config.toml [streaming]），可配置超时后是否终止会话
  → chunk 帧经 Channel Raw → 前端收 ArrayBuffer；started/finished/error 为 JSON
```

**说话人热更新**：`update_streaming_speakers` 全量提交 → 校验 → 头像 ingest → 替换 `context.json basic.speakers`（`id = name`，**name 即身份，禁止改名**）→ 进程在则发 0x11 帧让 Python `apply_speakers_update` 重建表。运行中新增/更换 `trained` 被拒（checkpoint 启动时加载定死），删除放行；`voice-clone` 可增删改。

**历史回放**：`get_streaming_replay_snapshot(historyId)` 恢复消息与配置，不依赖活动进程；音频仍按 `GeneratedAudioSource::StreamingSpeech { historyId, messageId }` 定位。

### 5.6 说话人管理

- `speakers` 表：名称、基础模型、描述、状态（Ready/Training/Disabled）、来源（local/preset/remote）、`avatar BLOB + avatar_content_type`。
- 创建/更新/删除；`import_model_as_speaker` 按模型目录中的训练产物导入。
- 头像：记录头像经 `get_speaker_avatar` 取字节；流式会话头像在创建任务时复制进 sample 目录，经 `get_streaming_speaker_avatar_path` 走 asset 协议零拷贝，缺失时回退记录头像/首字占位。
- 流式说话人类别：`voice-clone`（前端本地参考音频）、`trained`（Ready 说话人，`speakerDirName = speaker.id`）、`preset`（预留）。

### 5.7 历史任务与结果音频

- `list_history_records` 只返回轻量 `HistoryRecordSummary`；详情与日志按需拉取：`get_history_record_detail`、`get_history_task_log`（默认 64KB tail）。
- 任务状态刷新：后端 service 层发 `history:task-updated`（created/updated/deleted/cancelled），前端模块级单例监听 + 500ms 防抖扇出；各任务页另保留 30s 低频轮询兜底。
- 取消：`cancel_history_task(historyId)` → 按任务类型找到运行句柄 → 发取消信号。
- 回放：历史详情跳转携带 `replayTaskId`，对应 View 回填参数重新发起任务。
- 结果音频：`get_generated_audio_path` 返回绝对路径（零拷贝播放）；`get_generated_audio` 保留字节路径；`save_generated_audio_as` 用同一 `GeneratedAudioSource` 导出。

### 5.8 设置与国际化

- `config.toml` 分四段：`[basic]`（mode/data/log/model/language）、`[remote]`（api_url/api_token）、`[training]`（attn/normalization）、`[streaming]`（静默超时/终止会话）。
- `save_settings_config`：校验路径 → 迁移数据/日志/模型目录 → 持久化 → 返回 `restart_required`；`save_ui_language` 独立保存，避免触发目录迁移。
- `get_ui_config` 返回启动时加载的配置驱动表单目录。
- UI 语言（`zh-CN`/`en-US`）与语音语言（`AppLanguage`: 中/英/日/韩）是两个独立概念，禁止复用。

## 6. 关键契约与原理

### 6.1 模型适配器

`src-model/` 下 9 个 Git 子模块（克隆须 `--recurse-submodules`）：

| 适配器 | 版本 | TTS | 克隆 | 训练 | 音色设计 | 流式 | 设备 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `irodori_tts_v3` | 500M | ✓ | ✓ | ✓ | ✓ | — | CPU/CUDA |
| `dots_tts` | 2B | ✓ | ✓ | ✓ | — | — | CPU/CUDA |
| `qwen3_tts` | 1.7B / 0.6B | ✓ | ✓ | ✓ | ✓ | ✓* | CPU/CUDA |
| `vox_cpm2` | 2B | ✓ | ✓ | ✓ | ✓ | — | CPU/CUDA |
| `moss_tts_local` | 1.7B | ✓ | ✓ | ✓ | — | — | CPU/CUDA |
| `moss_tts_realtime` | 1.7B | — | — | — | — | ✓ | CPU/CUDA |
| `gpt_sovits_cpufast` | V1/V2/V2Pro/V2ProPlus | ✓ | ✓ | — | — | — | CPU |
| `index_tts` | 2.0 / 2.5 | ✓ | ✓ | — | — | — | CPU/CUDA |
| `ming_tts` | 0.5B | ✓ | ✓ | — | ✓ | — | CUDA |

- 特性/设备/语言以各适配器 `configs/model-config.json` 为准（应用启动扫描），上表只是当前快照。`✓*` = qwen3_tts 流式为分句模拟（`qwen_tts` 公开 API 仅整段合成），非 token 级真流式。
- 新增/维护适配器必读 `src-model/ADAPTER_DEVELOPMENT.md`：目录约定、两个 JSON schema、参数实体、入口脚本、下载方式、检查清单。
- 每个适配器自带 `requirements.txt`、`requirements-torch.txt`、可选 `requirements-post-torch.txt`；`ensure_torch_runtime` 只重装 torch 文件，不污染基础依赖。

### 6.2 参数文件（Rust ↔ Python）

`PythonScriptInvocationSpec` 写入任务 sample 目录（先写 `.tmp` 再 rename）：

```json
{
  "version": "1.0.0",
  "base_model": "irodori_tts_v3",
  "model_version": "500M",
  "kind": "TextToSpeech",              // Training / TextToSpeech / VoiceClone / VoiceDesign / StreamingSpeech
  "runtime": { "device": "cuda:0", "logging_dir": "...", "attn_implementation": "sdpa" },
  "args": { "TextToSpeech": { ... } }  // 键为 PascalCase；流式 kind=StreamingSpeech 但 args 键是 "Streaming"
}
```

- 字段为 snake_case；前端表单值放在 `common.model_params_json`，其 key 必须与 `params-config.json` 的 `name` 完全一致。
- Python 侧 `params_entity.py::ParamsEntity.from_file` 解析；`params.py` 转换；入口脚本只接受 `--params-file`，通过 `print(..., flush=True)` 输出进度，非 0 退出即失败。
- 参数文件会被复制到任务 sample 目录留档；模板/参考音频同理。

### 6.3 流式帧协议（环回 TCP，二进制）

帧格式：`[u32 LE len(kind+payload)][kind u8][payload]`，`MAX_FRAME_PAYLOAD = 16MB`。Rust 与 Python 的编解码逐字节对齐（`streaming_transport.rs` ↔ `streaming_transport.py`）。

| kind | 方向 | payload |
| --- | --- | --- |
| `0x01` AUTH | Python → Rust | `{"token"}`，模型加载前连接并校验 |
| `0x02` CONTROL | Python → Rust | JSON `{type:"started"/"finished"/"error"/"session_ready", contextId, message?}` |
| `0x03` CHUNK | Python → Rust | `[u16 LE idLen][contextId][PCM16 bytes]`；首帧前置 44 字节 WAV 哨兵头（data size=0xFFFFFFFF） |
| `0x10` INPUT | Rust → Python | JSON `{contextId, speakerName, text, audioPath}` |
| `0x11` SPEAKERS_UPDATE | Rust → Python | 全量说话人快照 JSON `{"speakers":[...]}` |

- 音频全程无 base64 / JSON 包裹；Rust → 前端用 `Channel<InvokeResponseBody>`，chunk 为 `Raw`，JS 收 `ArrayBuffer`。
- 说话人更新用全量快照：幂等、丢帧可自愈、与写盘同源。
- `contextId` 由前端生成，多路复用同一会话进程；`context.json` 的 read-modify-write 由每会话 `context_lock` 串行化。

### 6.4 数据模型

- 表：`app_meta`、`speakers`、`task_history`、`model_info`、`tts_tasks`、`model_training_tasks`、`voice_clone_tasks`、`voice_design_tasks`、`streaming_tasks`。
- `task_history` 统一承载 5 种任务；子表通过 `history_id` 关联；`streaming_tasks` 另有唯一索引。
- 时间字段统一 `create_time / modify_time`（前端 `createTime / modifyTime`，string），由后端生成；`finished_time` 可空。
- 迁移在 `src-tauri/src/migration/`，当前 14 个迁移、schema 版本 31；`db/tables.sql` + `db/tables_pgsql.sql` 是参考真值。

### 6.5 错误码

- `AppError { code?: string, message, params }`，camelCase；Rust 侧 `AppError::coded(...).with_param(...).into_anyhow()`，hooks 边界 `AppError::from_anyhow`。
- code 常量在 `src-tauri/src/error.rs::codes`，前端词条在 `src/locales/<lang>/errors.ts`，两侧必须同步。
- 未错误码化的报错仍以 `String` 透传；英文界面显示中文原文属预期。

## 7. 开发规范

### 7.1 命令

```bash
npm install
npm run tauri dev                 # Vite + Tauri 桌面端
npm run build                     # vue-tsc --noEmit + vite build
npm run tauri build               # NSIS 安装包
powershell -ExecutionPolicy Bypass -File src-tauri/scripts/make-portable.ps1
cd src-tauri && cargo check       # Rust 编译检查
cd src-tauri && cargo clippy
cd src-tauri && cargo test        # 集成测试
npx vue-tsc --noEmit              # 前端类型检查
```

- `dev` / `build` 前必须准备 `src-tauri/resources/`：`config.toml`、`ffmpeg-8.1.2.zip`、`sox-14.4.2-win32.zip`、`src-model-runtime.zip`（本地开发可先用空 zip 占位，workspace 的 `src-model/` 会被优先识别）。
- 日志在 `log_dir`；任务日志按任务类型 + id 写入（`tts-/voice-clone-/voice-design-/training-/streaming-<id>.log`），安装阶段另有 `install-*` 日志。

### 7.2 必须同步的契约

1. **表结构**：改迁移/建表 → 同时更新 `db/tables.sql` 与 `db/tables_pgsql.sql`（列、索引、表一一对应）。
2. **任务类型**：Rust `HistoryTaskType` 增删 → 同步 `src/enums/task.ts` 枚举 + 两个 `Record`（路由、文本 key）；TypeScript 穷尽检查会报错。
3. **远端 API**：改 `client::ApiClient` 方法/payload/分页/鉴权 → 同步 `docs/remote-api.yaml`（path、operationId、schema、`info.version`）。
4. **错误码**：新增 `codes` 常量 → 同步两组 `locales/*/errors.ts`。
5. **模型能力**：新增 feature 只在模型真正支持时声明；未就绪时不借用其它任务的 `params-config` 填充（见 §7.3）。
6. **流式说话人字段**：`context.json`、Rust `StreamingSpeaker*`、Python `streaming.py`、前端 `types/streaming.ts` 四处字段名须一致（camelCase）。
7. **模型打包清单**：每次新增模型适配器（`src-model/` 下新子模块）后，须同步更新 `src-model/scripts/windows/package_src_model.ps1` 的 `modelDirectories` 列表，否则打包产物（`src-model-runtime.zip`）缺该模型脚本。
8. **README 模型文档**：每次完成新模型支持后，须同步更新 `README.md` 与 `README.en-US.md`：§3「当前模型支持」表加行，§6「已支持模型项目指引」加项目/模型仓库链接（两个语言版本保持一致）。

### 7.3 代码与目录规范

- **测试放 `src-tauri/tests/`**，不用内联 `#[cfg(test)]`；需要测 `pub(crate)` 项时先提升为 `pub`，再经 `src/test_support.rs` 重导出。
- **不要全局跑 `cargo fmt`**：src-tauri HEAD 不是 rustfmt 干净状态，会改约 30 个无关文件；以 `cargo check/clippy/test` 验证，手动保持周边风格。
- **不擅自删预留字段**：有明确未来用途的字段/参数保留；警告可用 `#[allow(dead_code)]` + 注释。
- **不为未实现特性借用配置**：模型筛选与参数表单使用自身 `HistoryTaskType`；空着等真实实现。
- **时间字段**：一律 `createTime / modifyTime`（string），后端生成、前端只读。
- **流式说话人身份**：`name` 即 `context.json` 的 id，禁止改名；运行中 `trained` 仅可删不可加/换。
- **子进程 PATH**：Rust 拼接用 `utils::process::join_path_entries`，禁用 `std::env::join_paths`（Windows 引号语义会破坏 PATH）；PowerShell 解析系统工具优先绝对路径。
- **脚本边界**：模型初始化、调用、硬件探测逻辑归脚本层；不要为性能在 Rust 侧缓存设备判断或直连替代脚本链。
- **IO 阻塞**：大文件拷贝/解压/训练数据准备走 `utils::blocking::run_blocking`，不占 async worker。

### 7.4 i18n

- 新增页面/组件同时写 `zh-CN` 与 `en-US` 词条，命名空间 `<模块>.<区域>.<名称>`。
- 含中文的枚举文本表以 `_KEY` 结尾，值为 i18n key；技术术语表保持原文。
- `defineProps` 默认值不能引用 `t()`；渲染处 `?? t(...)` 回退。
- 消息中字面 `{}` / `@` / `|` 需转义。
- 适配器 `params-config.json` 的 label/description 不在 i18n 范围。

### 7.5 提交

- 按已批准计划/批次产出的 commit，完成后立即 `git reset --soft HEAD~1` 撤回，改动留给开发者检查；不要 `--hard`。
- 用户明确要求的提交（发布、PR）不适用。
