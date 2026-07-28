# 模型适配器开发指南

本指南面向需要在 `src-model/` 下**新增或维护模型适配器**的开发者，描述适配器与前端表单、Rust 调度层、Python 运行时之间的契约。

> 运行时总体结构（脚本分工、conda_env/venv 探测、平台脚本动作）请先阅读 [README.md](./README.md)。本指南只聚焦“如何让一个新模型被应用识别并能跑通四类任务”。

---

## 1. 适配器在整体架构中的位置

Kirine Client 的模型链路分为三层，适配器位于最底层：

```
┌──────────────────────────────────────────────────────────────┐
│ 前端 (Vue 3)                                                   │
│  GenericTaskParamsForm.vue  ← 按 params-config.json 渲染表单    │
│  收集到的参数值打包为 model_params_json                          │
└───────────────────────────────┬──────────────────────────────┘
                                 │  Tauri command
┌────────────────────────────────▼──────────────────────────────┐
│ Rust 后端                                                       │
│  启动时扫描 configs/*.json → 同步模型元数据与表单定义             │
│  任务执行时：拼装 --params-file（含 model_params_json）          │
│           → 调用 begin_llm_task.{ps1,sh}                       │
└───────────────────────────────┬──────────────────────────────┘
                                 │  --base-model --script-path --params-file
┌────────────────────────────────▼──────────────────────────────┐
│ Python 适配器 (src-model/<base_model>/)                         │
│  tts.py / voice_clone.py / voice_design.py / training.py       │
│   └─ params.py    ← 把 ParamsEntity 转成上游可消费的结构          │
│       └─ params_entity.py  ← 解析 --params-file JSON             │
│   └─ common.py    ← 路径解析 + subprocess 调上游 infer.py/train.py│
└──────────────────────────────────────────────────────────────┘
```

**核心契约**：前端表单字段的 `name` → Rust 写入 `model_params_json` 的同名 key → Python 适配器通过 `params.model_param_xxx(name)` 读取。三层之间不硬编码字段，全部以两个 JSON 配置文件为契约源。

---

## 2. 适配器目录结构约定

以参考实现 `irodori_tts_v3` 为例，一个标准适配器目录包含：

| 文件 | 职责 | 必需 |
| --- | --- | --- |
| `configs/model-config.json` | 模型元数据（版本、功能、设备、语言、预置说话人） | ✅ |
| `configs/params-config.json` | 前端任务参数表单定义 | ✅ |
| `params_entity.py` | 解析 `--params-file` JSON，提供 `ParamsEntity` | ✅ |
| `params.py` | 把 `ParamsEntity` 转成适配器专用 dataclass | ✅ |
| `tts.py` | 文本转语音入口 | 支持 TTS 时 |
| `voice_clone.py` | 声音克隆入口 | 支持克隆时 |
| `voice_design.py` | 音色设计入口 | 支持音色设计时 |
| `training.py` | 模型训练入口 | 支持训练时 |
| `common.py` | 路径解析、设备归一化、subprocess 调上游脚本 | 推荐 |
| `download.py` | 克隆上游仓库 / 下载权重（`downloadType: Custom` 时） | 视情况 |
| `requirements.txt` | 基础 Python 依赖 | ✅ |
| `requirements-torch.txt` | Torch 运行时依赖（CPU/CUDA 由脚本切换） | ✅ |
| `requirements-post-torch.txt` | 依赖 torch 的运行时依赖（如 deepspeed），由 init_task_runtime 在 torch 装完后安装 | 可选 |
| `__init__.py` | 包标识（部分适配器兼作 TTS 入口） | ✅ |

> 各适配器因上游差异略有不同（见 [§8 各适配器实现差异](#8-各适配器实现差异)），新增适配器建议遵循上述 irodori 结构。

---

## 3. 两个配置文件

应用启动时，Rust 扫描所有 `<adapter>/configs/` 下的配置文件并同步到数据库与前端。两个文件均使用 camelCase 键名。

### 3.1 `configs/model-config.json` — 模型元数据

对应 Rust `ModelInfo` 结构（`src-tauri/src/service/models.rs`）。顶层为 `{ "models": [...], "speakers": [...] }`。

**`models[]` 字段：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `baseModel` | string | 适配器目录名，作为全局唯一标识（如 `irodori_tts_v3`） |
| `modelName` | string | 展示名（如 `Irodori-TTS-V3`） |
| `modelVersion` | string | 版本号（如 `500M`、`V1`）；同一 `baseModel` 可声明多个版本 |
| `downloadType` | `"HF-Like"` \| `"Custom"` | `HF-Like`：按 `requiredModelRepoIdList` 从 Hugging Face 拉取；`Custom`：交由适配器 `download.py` 自行处理 |
| `requiredModelNameList` | string[] | 下载后落地的本地目录名清单 |
| `requiredModelRepoIdList` | string[] | Hugging Face repo id 清单（`HF-Like` 时使用） |
| `supportedDevices` | (`"cpu"` \| `"cuda"`)[] | 支持的设备类型 |
| `supportedFeatureList` | Feature[] | 支持的任务功能，取值见下 |
| `supportedLanguages` | Language[] | 支持的语言，取值见下 |

**`supportedFeatureList` 取值**（对应 `HistoryTaskType`）：

- `text-to-speech` / `voice-clone` / `voice-design` / `model-training`

**`supportedLanguages` 取值**（对应 `AppLanguage`）：

- `chinese` / `english` / `japanese` / `korean`

**`speakers[]` 字段**（预置说话人，可为空数组）：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 说话人名称 |
| `baseModel` | string | 归属适配器 |
| `description` | string | 描述 |

示例（多版本 + 预置说话人，摘自 `qwen3_tts`）：

```json
{
  "models": [
    {
      "baseModel": "qwen3_tts",
      "modelName": "Qwen3-TTS",
      "modelVersion": "1.7B",
      "downloadType": "HF-Like",
      "requiredModelNameList": ["Qwen3-TTS-12Hz-1.7B-Base", "Qwen3-TTS-Tokenizer-12Hz"],
      "requiredModelRepoIdList": ["Qwen/Qwen3-TTS-12Hz-1.7B-Base", "Qwen/Qwen3-TTS-Tokenizer-12Hz"],
      "supportedDevices": ["cpu", "cuda"],
      "supportedFeatureList": ["text-to-speech", "voice-clone", "model-training", "voice-design"],
      "supportedLanguages": ["chinese", "english", "japanese", "korean"]
    }
  ],
  "speakers": [
    { "name": "Vivian", "baseModel": "qwen3_tts", "description": "Bright young female voice. Chinese." }
  ]
}
```

### 3.2 `configs/params-config.json` — 前端任务参数表单

对应 Rust `UiConfigCatalog`（`src-tauri/src/config/ui_config.rs`）。文件本身是一个**数组**，每个元素描述一个 `(task, base-model)` 组合的参数表单。Rust 按 `base-model:task` 去重，重复条目会告警并跳过。

**数组元素字段：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `task` | Task | `text-to-speech` / `voice-clone` / `voice-design` / `model-training` |
| `base-model` | string | 对应 `model-config.json` 的 `baseModel` |
| `params[]` | ParamDefinition[] | 参数字段定义列表 |

**`ParamDefinition` 字段：**

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `name` | string | 字段名（camelCase）。**这是贯穿三层的契约 key**，最终成为 `model_params_json` 中的键名 |
| `type` | `"number"` \| `"string"` \| `"boolean"` | 值类型（`UiParamType`） |
| `componentType` | ComponentType | 前端渲染组件（见 [§4](#4-前端支持的组件类型)） |
| `componentProps` | object | 透传给组件的属性（见下） |
| `required` | boolean | 是否必填 |
| `defaultValue` | any | 默认值（可为 `null`） |
| `description` | string | 字段说明，前端作为辅助文本展示 |

**`componentProps` 通用字段：**

| 字段 | 适用组件 | 说明 |
| --- | --- | --- |
| `label` | 全部 | 字段标签 |
| `helpText` | 全部 | 覆盖 `description` 作为辅助文本 |
| `placeholder` | input-* / textarea / select | 占位文本 |
| `min` / `max` / `step` | input-number | 数值范围与步进 |
| `nullable` | input-number | 为 `true` 时空值变 `null`，否则空值回退为 `0` |
| `inputMode` | input-number | 浏览器 inputmode |
| `rows` | textarea | 行数，默认 3 |
| `text` / `textOn` / `textOff` | switch | 开关文案 |
| `options` | select | `{label, value}[]` 选项列表 |
| `visibleWhen` | 全部 | 条件渲染：`{ "field": "<其他字段name>", "equals": <值> }` |
| `dialogTitle` / `buttonText` / `clearButtonText` / `extensions` | input-audio-file / input-text-file | 文件选择对话框配置；`extensions` 为扩展名白名单 |

> `visibleWhen` 的匹配规则：当前表单中 `visibleWhen.field` 的值经 `JSON.stringify` 后与 `visibleWhen.equals` 相等才渲染。例如 LoRA 相关字段 `{ "visibleWhen": { "field": "trainingMode", "equals": "lora" } }` 只在微调模式选 LoRA 时出现。

示例（摘自 `irodori_tts_v3` 的 `model-training`，含 select / input-number / switch / visibleWhen）：

```json
{
  "task": "model-training",
  "base-model": "irodori_tts_v3",
  "params": [
    {
      "name": "trainingMode",
      "type": "string",
      "componentType": "select",
      "componentProps": {
        "label": "微调模式",
        "options": [
          { "label": "Speaker Inversion（说话人嵌入）", "value": "speaker_inversion" },
          { "label": "LoRA 微调", "value": "lora" }
        ]
      },
      "required": true,
      "defaultValue": "speaker_inversion",
      "description": "微调模式。"
    },
    {
      "name": "loraR",
      "type": "number",
      "componentType": "input-number",
      "componentProps": { "label": "LoRA 秩", "min": 1, "step": 1, "visibleWhen": { "field": "trainingMode", "equals": "lora" } },
      "required": true,
      "defaultValue": 16,
      "description": "LoRA 秩（--lora-r）。仅在 LoRA 模式下生效。"
    }
  ]
}
```

---

## 4. 前端支持的组件类型

前端由 `src/components/form/GenericTaskParamsForm.vue` 根据 `componentType` 分发到对应的 `UiParam*` 组件。当前支持 **7 种**组件类型（对应 Rust `UiComponentType`，全部已在用）：

| `componentType` | 前端组件 | `type` 建议 | 渲染效果 | 当前用量 |
| --- | --- | --- | --- | --- |
| `input-number` | `UiParamInputField`（type=number） | `number` / `string` | 数字输入框，支持 min/max/step/nullable | 84 |
| `input-text` | `UiParamInputField`（type=text） | `string` | 单行文本输入框 | 3 |
| `textarea` | `UiParamTextareaField` | `string` | 多行文本，支持 rows | 2 |
| `select` | `UiParamSelectField` | `string` | 下拉选择，需配 `options` | 11 |
| `switch` | `UiParamSwitchField` | `boolean` | 开关，支持 text/textOn/textOff | 12 |
| `input-audio-file` | `UiParamAudioFileField` | `string` | 音频文件选择，支持 extensions 白名单 | 5 |
| `input-text-file` | `UiParamTextFileField` | `string` | 文本文件选择，支持 extensions 白名单 | 1 |

**实现要点：**

1. `input-number` 与 `input-text` 共用 `UiParamInputField`，按 `componentType` 切换 `<input type>`。
2. 空值处理：`input-number` 在 `nullable: true` 时空值提交 `null`；否则空值回退为 `0`（`type: number`）或原字符串（`type: string`）。需要“留空=不限”语义的字段（如随机种子、最大步数）务必设 `nullable: true` 并把 `defaultValue` 设为 `null`。
3. `select` 的 `options[].value` 可以是字符串、数字或布尔；前端按原始类型回传。
4. 文件类组件（`input-audio-file` / `input-text-file`）的 `dialogTitle` / `buttonText` / `clearButtonText` / `extensions` 走 `componentProps` 的扩展字段（Rust 侧 `ComponentProps.extra` 透传）。
5. 当 `visibleParams` 过滤后为空时，表单渲染 `UiParamEmptyState` 空状态。

---

## 5. Python 适配器实现流程

### 5.1 `params_entity.py` — 参数载体（契约层）

本文件定义 `--params-file` 的 JSON 结构，是 Rust 与 Python 之间的**稳定契约**。各适配器的 `params_entity.py` 内容基本一致，核心是 `ParamsEntity.from_file(path)`：

```python
@dataclass(frozen=True)
class ParamsEntity:
    version: str
    base_model: str
    model_version: str
    kind: TaskKind                 # Training / TextToSpeech / VoiceClone / VoiceDesign
    runtime: RuntimeOptions        # device / logging_dir / attn_implementation
    args: TrainingArgs | TextToSpeechArgs | VoiceCloneArgs | VoiceDesignArgs
```

**`--params-file` JSON 结构：**

```json
{
  "version": "1.0.0",
  "base_model": "irodori_tts_v3",
  "model_version": "500M",
  "kind": "TextToSpeech",
  "runtime": { "device": "cuda", "logging_dir": "...", "attn_implementation": "sdpa" },
  "args": {
    "TextToSpeech": {
      "common": {
        "model_root_path": "<model_dir>",
        "speaker_dir_name": "<speaker_id>",
        "model_params_json": { "numSteps": 40, "cfgScaleText": "3.0", "seed": null }
      },
      "text": "...",
      "language": "chinese",
      "speaker": "Vivian",
      "output_path": "<output.wav>"
    }
  }
}
```

关键点：

- `kind` 决定 `args` 内层 key（`args.TextToSpeech` / `args.VoiceClone` / `args.VoiceDesign` / `args.Training`），且为 PascalCase 字符串。
- `common.model_params_json` 就是前端表单收集到的参数字典，**key 与 `params-config.json` 的 `name` 一一对应**。
- `ParamsEntity` 提供取值辅助方法，`params.py` 通过它们读取表单值：
  - `params.model_param(key, default)` — 原始值
  - `params.model_param_str(key, default)` — 字符串
  - `params.model_param_int(key, default)` — 整数
  - `params.model_param_bool(key, default)` — 布尔（兼容 `"true"`/`1`/`"on"` 等字符串形式）
- 任务参数通过 `params.tts_args()` / `voice_clone_args()` / `voice_design_args()` / `training_args()` 取得，内部会校验 `kind` 是否匹配，类型不符会抛错。

### 5.2 `params.py` — 适配器专用映射

把 `ParamsEntity` 转成上游脚本需要的、强类型的 dataclass，并解析适配器私有路径（基座权重、训练产物等）。这是**适配器改动最频繁**的文件。

典型模式（摘自 `irodori_tts_v3`）：

```python
def load_tts_params(path: str | Path) -> IrodoriTtsParams:
    params = ParamsEntity.from_file(path)      # 解析 --params-file
    args = params.tts_args()                   # 取 TTS 任务参数 + kind 校验

    text = (args.text or "").strip()
    if not text:
        raise ValueError("Text cannot be empty.")

    artifact = resolve_speaker_artifact(       # 解析训练产物（ref_embed / lora_adapter）
        args.common.model_root_path, args.common.speaker_dir_name
    )
    speaker_kind, speaker_path = (None, None)
    if artifact is not None:
        speaker_kind, speaker_path = artifact

    return IrodoriTtsParams(
        text=text,
        output_path=args.output_path,
        base_checkpoint=base_checkpoint(),     # 基座权重路径
        speaker_kind=speaker_kind,
        speaker_path=speaker_path,
        sampling=_load_sampling(params, with_caption=False),  # 读 model_params_json
        device=normalize_device(params.runtime.device),
    )
```

`_load_sampling` 内部用 `params.model_param_int("numSteps", 40)`、`params.model_param_str("modelPrecision", "fp32")` 等读取前端表单值——**这里的 key 必须与 `params-config.json` 的 `name` 完全一致**，否则取不到值只能走默认。

### 5.3 入口脚本 — `tts.py` / `voice_clone.py` / `voice_design.py` / `training.py`

四个入口脚本职责单一：解析 `--params-file` → 调 `params.py` 的 `load_xxx_params` → 拼装上游 CLI 参数 → 调 `common.py` 的 `run_infer` / `run_train`。

统一契约：

- 只接受一个必填参数 `--params-file`，指向 Rust 写入的 JSON 文件。
- 不直接读模型权重，权重路径由 `params.py` + `common.py` 解析。
- 通过 `print(..., flush=True)` 输出进度，stdout/stderr 由 `begin_llm_task` 重定向到任务日志。
- 失败时抛异常或 `raise SystemExit`，退出码非 0 即标记任务失败。

```python
# tts.py 骨架
def main(argv=None):
    cli_args = parse_args(argv)                 # --params-file
    params = load_tts_params(cli_args.params_file)
    print(f"[irodori_tts] TTS text_len={len(params.text)} device={params.device}", flush=True)
    run_infer(build_infer_args(params))         # 调上游 infer.py

if __name__ == "__main__":
    main()
```

> 入口脚本名是 Rust 调度时的 `--script-path` 目标。`model-training` 用 `training.py`，`text-to-speech` 用 `tts.py`，以此类推。`dots_tts` 历史上用 `__init__.py` 作为 TTS 入口，新适配器建议统一用 `tts.py`。

### 5.4 `common.py` — 路径解析与上游调用

集中放置适配器私有逻辑，避免入口脚本重复：

1. **路径解析**：基座 / 音色设计权重路径经 `__file__` 推导，不依赖运行时参数；训练产物（说话人）路径由 `model_root_path` + `speaker_dir_name` 拼接。
2. **训练产物识别**：`resolve_speaker_artifact` 返回 `(kind, path)`，区分 Speaker Inversion（`*.speaker.safetensors` → `ref_embed`）与 LoRA（`checkpoint_final/` 目录 → `lora_adapter`）。
3. **设备归一化**：`normalize_device` 把 `cuda:*` / `cpu` 归一为上游 CLI 接受的 `cuda` / `cpu`。
4. **上游 subprocess**：以 `sys.executable` 运行上游 `infer.py` / `train.py`，`cwd` 设为上游仓库根目录，使上游包经 `sys.path[0]` 自动可导入（无需 editable install），并流式转发输出。

### 5.5 `download.py`

仅在 `model-config.json` 的 `downloadType` 为 `Custom` 时由下载脚本调用。职责是克隆上游仓库、下载权重、预热 tokenizer/codec 等离线资产。`downloadType: HF-Like` 的适配器（如 `qwen3_tts`）不需要此文件，由运行时脚本按 `requiredModelRepoIdList` 直接从 Hugging Face 拉取。

### 5.6 `requirements.txt` / `requirements-torch.txt` / `requirements-post-torch.txt`

- `requirements.txt`：基础 Python 依赖（transformers、safetensors、librosa 等），由 `init_task_runtime` 在创建 venv/conda_env 时安装（此时 torch 尚未装入，故此文件不得包含依赖 torch 的包）。
- `requirements-torch.txt`：Torch 运行时依赖，由 `ensure_torch_runtime` 在 CPU/CUDA 切换时按 `index-url` 安装对应轮子。
- `requirements-post-torch.txt`：可选；依赖 torch 的运行时依赖（构建需要 torch 已就绪），由 `init_task_runtime` 在 torch 安装完成后安装。文件不存在则跳过，其他适配器不受影响。

三个文件共同保证不同适配器的依赖互不污染（每个适配器独立 venv/conda_env）。

---

## 6. 完整调用链路

以一次 TTS 任务为例：

```
前端 GenericTaskParamsForm 收集 { numSteps: 40, cfgScaleText: "3.0", ... }
   │
   ▼  Tauri command
Rust 拼装 --params-file（含 args.TextToSpeech.common.model_params_json）
   │
   ▼  Pipeline 调用
begin_llm_task.{ps1,sh}
   --base-model irodori_tts_v3 --script-path tts.py
   --params-file <tmp.json> --task-log-file <log>
   │  探测 conda_env/python.exe，否则回退 venv/Scripts/python
   │  注入 -X utf8 -X faulthandler -u，重定向输出到 task-log
   ▼
tts.py main()
   ├─ load_tts_params(--params-file)
   │    ├─ ParamsEntity.from_file → 解析 JSON
   │    ├─ params.tts_args() → 取 text / output_path / common
   │    └─ _load_sampling(params) → 读 model_params_json 的 numSteps 等
   ├─ build_infer_args(params) → 拼上游 --checkpoint / --text / --num-steps ...
   └─ run_infer(args)
        └─ subprocess: sys.executable infer.py --checkpoint ... (cwd=上游仓库根)
```

训练产物→TTS 的衔接：训练时 Rust 把产物写到 `output_model_path = <model_dir>/<speaker_id>`；TTS 时 Rust 给出 `model_root_path = <model_dir>` + `speaker_dir_name = <speaker_id>`，`common.py` 拼起来定位产物，自动选择 `--ref-embed` / `--lora-adapter` / `--no-ref`。

---

## 7. 新增适配器检查清单

1. **创建目录** `src-model/<base_model>/`（`base_model` 用 snake_case，作为唯一标识）。
2. **挂载为 Git 子模块**（若是独立仓库）或直接纳入主仓库；确认 `.gitignore` 忽略 `conda_env/`、`venv/`、`__pycache__/`。
3. **写 `configs/model-config.json`**：声明 `baseModel` / 版本 / `downloadType` / 功能 / 设备 / 语言 / 预置说话人。
4. **写 `configs/params-config.json`**：为每个支持的功能定义参数表单，字段 `name` 用 camelCase 并记住它将贯穿到 Python。
5. **写 `params_entity.py`**：可直接从 `irodori_tts_v3` 复制，按需调整任务参数字段。
6. **写 `params.py`**：定义适配器专用 dataclass 与 `load_xxx_params`，用 `model_param_xxx(name)` 读取表单值，`name` 与 `params-config.json` 对齐。
7. **写入口脚本**：为每个支持的功能写 `tts.py` / `voice_clone.py` / `voice_design.py` / `training.py`，统一 `--params-file` 入参。
8. **写 `common.py`**：路径解析、设备归一化、subprocess 调上游。
9. **写 `download.py`**（仅 `Custom` 下载类型）。
10. **写 `requirements.txt` + `requirements-torch.txt`**。
11. **本地联调**：`npm run tauri dev` 启动后，在模型管理页安装新模型，确认四类任务能跑通；日志在 `log_dir/<task_id>/`。
12. **类型检查**：`src-model/` 下有 `pyrightconfig.json`，可用 pyright 验证 Python 侧类型。

---

## 8. 各适配器实现差异

| 适配器 | TTS 入口 | 声音克隆 | 音色设计 | 训练 | 下载 | 备注 |
| --- | --- | --- | --- | --- | --- | --- |
| `irodori_tts_v3` | `tts.py` | `voice_clone.py` | `voice_design.py` | `training.py` | `download.py` | 参考实现，四类任务齐全 |
| `dots_tts` | `__init__.py` | `voice_clone.py` | — | `training.py` + `training_common.py` + `dataset.py` | `download.py` | TTS 入口为 `__init__.py`；训练拆共享逻辑 |
| `qwen3_tts` | `tts.py` | `voice_clone.py` | `voice_design.py` | `training.py` + `training_common.py` + `training_full.py` | HF-Like（无 `download.py`） | 含 `encode_audio.py` 训练前预处理 |
| `vox_cpm2` | `tts.py` | `voice_clone.py` | `voice_design.py` | `training.py` + `train_voxcpm_finetune.py` | HF-Like（无 `download.py`） | 本地维护训练入口副本 |
| `moss_tts_local` | `tts.py` | `voice_clone.py` | — | `training.py` | `download.py` | v0.11.2 重构为标准脚本调用模式 |
| `gpt_sovits_cpufast` | `tts.py` | `voice_clone.py` | — | — | `download.py` + `inference_bridge.py` | 仅 CPU；无训练；多版本（V1/V2/V2Pro/V2ProPlus） |

新增适配器建议以 `irodori_tts_v3` 为模板，按上游能力裁剪入口脚本。
