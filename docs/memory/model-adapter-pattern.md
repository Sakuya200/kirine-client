---
name: model-adapter-pattern
description: 配置驱动的模型适配器、特性矩阵、参数文件执行、设备支持
metadata:
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# 模型适配器架构

> 状态截至 2026-07-21 · 分支 `v.0.12.0`

## 模型体系
项目采用 **Git 子模块 + 配置驱动** 的模型适配器架构。每个模型作为独立 Git 子模块存在于 `src-model/` 目录，通过标准化的命令行接口（`--params-file`）与后端 Pipeline 通信。

## 支持的模型（6 个）

| 模型 | 版本 | 子模块路径 | TTS | 声音克隆 | 微调 | 音色设计 | 设备 | 说明 |
|------|------|-----------|-----|---------|------|---------|------|------|
| **irodori_tts_v3** | 500M | `src-model/irodori_tts_v3` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | 上游 Aratako/Irodori-TTS，RF Euler 流匹配 |
| **dots_tts** | 2B | `src-model/dots_tts` | ✅ | ✅ | ✅ | ❌ | CPU/CUDA | 上游 rednote-hilab/dots.tts (Qwen2 backbone) |
| **qwen3_tts** | 1.7B/0.6B | `src-model/qwen3_tts` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | Qwen3 系列 TTS 模型 |
| **vox_cpm2** | 2B | `src-model/vox_cpm2` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | 支持音色设计 (Voice Design) |
| **moss_tts_local** | 1.7B | `src-model/moss_tts_local` | ✅ | ✅ | ✅ | ❌ | CPU/CUDA | MOSS-TTS Local，标准脚本调用模式（`tts.py`/`voice_clone.py`/`training.py` + `--params-file`） |
| **gpt_sovits_cpufast** | V1/V2/V2Pro/V2ProPlus | `src-model/gpt_sovits_cpufast` | ✅ | ✅ | ❌ | ❌ | CPU | CPU 优化的 GPT-SoVITS，V2+ 为实验性 |

> 特性矩阵、设备支持与支持语言以各子模块 `configs/model-config.json` 的 `supportedFeatureList` / `supportedDevices` / `supportedLanguages` 为准（应用启动时扫描）。

## 模型适配器标准接口

> 📖 适配器开发规范详见 `src-model/ADAPTER_DEVELOPMENT.md`（覆盖目录结构、`--params-file` 契约、model-config/params-config schema、下载方式等）。

每个模型子模块通过以下统一命令行模式与 Rust 后端交互：

```bash
python <model>/<task>.py --params-file /path/to/params.json
```

### 各模型的任务入口脚本

**dots_tts**:
| 任务 | 脚本 | 说明 |
|------|------|------|
| TTS | `__init__.py` (main) | `generate()` -> audio WAV |
| Voice Clone | `voice_clone.py` (main) | `generate_with_speaker_audio()` -> audio WAV |
| Training | `training.py` (main) | JSONL manifest -> fine-tune via HuggingFace Accelerate |

**通用模式**:
| 任务 | 典型脚本 |
|------|----------|
| TTS | `__init__.py` 或 `tts.py` |
| Voice Clone | `voice_clone.py` |
| Voice Design | `voice_design.py` |
| Training | `training.py` |
| Download | `download.py` |

### 参数文件 (params JSON)
每个模型子模块包含 `configs/params-config.json`，定义参数表单 schema。后端通过 `PythonScriptInvocationSpec` 将用户输入序列化为 JSON 参数文件，传递给 Python 脚本。

### 模型配置 (model JSON)
每个模型子模块包含 `configs/model-config.json`，定义：
- 模型名称、版本、描述
- 设备支持 (CPU/CUDA/MPS)
- 特性矩阵 (supported_features: tts/voice_clone/voice_design/training)
- **支持语言 (supportedLanguages)**：`Vec<AppLanguage>`，驱动前端各任务页"输出语言"下拉。缺省值 `[chinese, english, japanese]`（`SupportedModelDefinition` 的 `#[serde(default)]`）。
- 下载方式 (Git LFS / HTTP 等)

> 说话人对象的 `languages` 字段已移除；模型级 `supportedLanguages` 取而代之。`AppLanguage` 枚举含 Chinese/English/Japanese/**Korean** 四种（`service/models.rs` + `src/enums/language.ts`）。

## dots_tts 模型详解

### 目录结构
```
src-model/dots_tts/
├── __init__.py           # TTS 入口 + main()
├── voice_clone.py        # 声音克隆入口
├── training.py           # 微调入口 (SingleSample training)
├── common.py             # 公共: load_runtime(), save_generated_audio()
├── training_common.py    # 训练公共: Dataset/DataLoader/Optimizer/Scheduler
├── dataset.py            # DotsTtsDataset (JSONL manifest -> DataLoader)
├── params.py             # 参数加载与校验 (load_tts_params/load_voice_clone_params)
├── params_entity.py      # 参数实体类型定义
├── download.py           # 模型下载
├── configs/
│   ├── model-config.json
│   └── params-config.json
└── requirements.txt
```

### 技术特点
- **Backbone**: Qwen2 LLM，支持 gradient checkpointing 节省显存
- **训练框架**: HuggingFace Accelerate + AdamW + Cosine warmup scheduler
- **精度**: GPU 用 bfloat16，CPU 用 float32
- **音频格式**: WAV 输出，通过 `scipy.io.wavfile.write` 写入
- **设备感知**: 通过 `--device` 参数区分 CPU/CUDA 训练路径

### 关键参数
**TTS**: `text`, `num_steps` (default 10), `guidance_scale` (1.2), `speaker_scale` (1.5), `language`
**Voice Clone**: `text`, `speaker_audio_path`, `num_steps` (10), `guidance_scale` (1.2), `speaker_scale` (1.2)
**Training**: `train_manifest` (JSONL), `num_epochs`, `batch_size`, `learning_rate` (2e-5), `weight_decay` (0.1), `warmup_steps`/`warmup_ratio`, `gradient_accumulation_steps`, `mixed_precision`

## Python 运行时环境（venv / conda_env）

每个基础模型在 `<model_root>/` 下维护独立的 Python 运行时，避免不同模型链路互相污染：

- **venv**: `<model_root>/venv`（默认，`init_task_runtime` 通过 `python -m venv` 创建，Python 3.12）
- **conda_env**: `<model_root>/conda_env`（当系统 PATH 检测到 conda CLI 时优先使用）

环境选择策略（`scripts/{windows,unix}/init_task_runtime.*` 与 `common.ps1`）：
1. `init_task_runtime` 探测 conda：Windows 用 `Get-CondaExecutable`（优先 `*.exe`），Unix 用 `command -v conda`。
2. 若检测到 conda -> `conda create -y --prefix <model_root>/conda_env python=3.12`；否则走 venv。
3. 后续 `ensure_torch_runtime` / `download_models` / `begin_llm_task` 均先探测 `conda_env/python(.exe)` 是否存在，存在则用 conda env，否则回退 venv。
4. Windows 下 conda env 的 python 通过 `conda run --prefix <conda_env> python --` 调用；若 conda CLI 不在 PATH 但 env 已存在，则直接调用 env 内 `python.exe`。

任务执行统一由包装器脚本 `begin_llm_task.{ps1,sh}`（见 [[tech-stack-backend]] Pipeline 执行模型）驱动：解析 `--base-model/--script-path/--params-file/--log-path/--task-log-file`，选 python，注入 `-X utf8 -X faulthandler -u`，stdout/stderr 追加写入 task-log-file。Windows 与 Unix 包装器均已存在。

## irodori_tts_v3 模型详解

- 上游: GitHub `Aratako/Irodori-TTS`，权重 HF `Aratako/Irodori-TTS-500M-v3`（基座）+ `Aratako/Irodori-TTS-600M-v3-VoiceDesign`（音色设计）
- 流匹配采样: RF Euler，默认 40 步；CFG 分文本 (`cfgScaleText` 3.0) 与说话人 (`cfgScaleSpeaker` 5.0) 两路
- 额外预热 HF 缓存: `Aratako/Semantic-DACVAE-Japanese-32dim`、`llm-jp/llm-jp-3-150m`
- 入口脚本: `tts.py` / `voice_clone.py` / `voice_design.py` / `training.py` / `download.py`，`infer.py` 从克隆仓库根目录运行（经 `sys.path[0]` 导入 `irodori_tts` 包，不做 editable install）

## 关联记忆
- [[project-overview]] - 项目全貌
- [[tech-stack-backend]] - Rust 后端 Pipeline 架构
- [[tech-stack-frontend]] - 前端参数表单
- [[data-flow-and-types]] - 数据流与类型
