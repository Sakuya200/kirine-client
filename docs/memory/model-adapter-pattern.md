---
name: model-adapter-pattern
description: 配置驱动的模型适配器、特性矩阵、参数文件执行、设备支持
metadata:
  node_type: memory
  type: project
  originSessionId: 477b1f78-1d15-4316-9e8d-45edad057e8b
---

# 模型适配器架构

> 状态截至 2026-09-01 · 分支 `v.0.12.0`

## 模型体系
项目采用 **Git 子模块 + 配置驱动** 的模型适配器架构。每个模型作为独立 Git 子模块存在于 `src-model/` 目录，通过标准化的命令行接口（`--params-file`）与后端 Pipeline 通信。

## 支持的模型（7 个）

| 模型 | 版本 | 子模块路径 | TTS | 声音克隆 | 微调 | 音色设计 | 设备 | 说明 |
|------|------|-----------|-----|---------|------|---------|------|------|
| **irodori_tts_v3** | 500M | `src-model/irodori_tts_v3` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | 上游 Aratako/Irodori-TTS，RF Euler 流匹配 |
| **dots_tts** | 2B | `src-model/dots_tts` | ✅ | ✅ | ✅ | ❌ | CPU/CUDA | 上游 rednote-hilab/dots.tts (Qwen2 backbone) |
| **qwen3_tts** | 1.7B/0.6B | `src-model/qwen3_tts` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | Qwen3 系列 TTS 模型 |
| **vox_cpm2** | 2B | `src-model/vox_cpm2` | ✅ | ✅ | ✅ | ✅ | CPU/CUDA | 支持音色设计 (Voice Design) |
| **moss_tts_local** | 1.7B | `src-model/moss_tts_local` | ✅ | ✅ | ✅ | ❌ | CPU/CUDA | MOSS-TTS Local，标准脚本调用模式（`tts.py`/`voice_clone.py`/`training.py` + `--params-file`） |
| **moss_tts_realtime** | 1.7B | `src-model/moss_tts_realtime` | ❌ | ❌ | ❌ | ❌ | CUDA/CPU | MOSS-TTS-Realtime，**会话级流式 `streaming.py` 首例**（仅 `streaming-speech`），上游 OpenMOSS/MOSS-TTS |
| **gpt_sovits_cpufast** | V1/V2/V2Pro/V2ProPlus | `src-model/gpt_sovits_cpufast` | ✅ | ✅ | ❌ | ❌ | CPU | CPU 优化的 GPT-SoVITS，固定上游 main `d3e5875`（G2PW pth 权重已纳入下载清单） |

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

### 模型当前设备选择 (currentDevice)
`model_info.current_device`（schema 29）存储用户在模型管理页为每模型选择的当前设备，决定 `install_model`/重装使用的设备（不再默认 Cpu）。
- 单设备模型（如 gpt_sovits_cpufast 仅 cpu）：sync 时自动回填唯一设备，前端下拉只读。
- 多设备模型（cpu+cuda）：初始留空，用户须主动选择后才能点安装（按钮禁用兜底）。
- 选择跨 sync 持久化：已有值仍属 `supportedDevices` 则保留，失效才纠偏。详见 [[tech-stack-backend]] `model_info.current_device 列` 与 [[data-flow-and-types]]。

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

### 依赖文件与安装顺序（init_task_runtime）

每个适配器在 `<model_root>/` 下维护三类 requirements 文件，`init_task_runtime` 按固定顺序装入 venv/conda_env：

1. `requirements.txt`：基础 Python 依赖（transformers/safetensors/librosa 等），建 venv 后最先装。**不得包含依赖 torch 的包**（此时 torch 尚未装入）。
2. `requirements-torch.txt`：Torch 运行时依赖（torch/torchvision/torchaudio/torchcodec），按 CPU/CUDA `--index-url` 从 pytorch wheel 索引安装。
3. `requirements-post-torch.txt`（可选）：依赖 torch 的运行时依赖（构建需 torch 已就绪），在 torch 装完后安装。文件不存在则跳过，其他适配器不受影响。

`ensure_torch_runtime`（任务时切换 torch CPU/CUDA）只重装 `requirements-torch.txt`，不触碰基础/post-torch 依赖。torch 已初始化时 `init_task_runtime` 走早退路径，整体跳过依赖安装。

## irodori_tts_v3 模型详解

- 上游: GitHub `Aratako/Irodori-TTS`，权重 HF `Aratako/Irodori-TTS-500M-v3`（基座）+ `Aratako/Irodori-TTS-600M-v3-VoiceDesign`（音色设计）
- 流匹配采样: RF Euler，默认 40 步；CFG 分文本 (`cfgScaleText` 3.0) 与说话人 (`cfgScaleSpeaker` 5.0) 两路
- 额外预热 HF 缓存: `Aratako/Semantic-DACVAE-Japanese-32dim`、`llm-jp/llm-jp-3-150m`
- 入口脚本: `tts.py` / `voice_clone.py` / `voice_design.py` / `training.py` / `download.py`，`infer.py` 从克隆仓库根目录运行（经 `sys.path[0]` 导入 `irodori_tts` 包，不做 editable install）

## moss_tts_realtime 模型详解

- 上游: GitHub `OpenMOSS/MOSS-TTS`（克隆到 `base-models/moss_tts_realtime/`，cwd=`src-model/`，零改动上游）；`mossttsrealtime` 包位于仓库根的 `moss_tts_realtime/` 子目录，脚本不在仓库根运行（`sys.path[0]`=脚本目录），故运行前由 `common.ensure_package_on_path()` 把包目录注入 `sys.path`。权重 HF `OpenMOSS-Team/MOSS-TTS-Realtime`（基座）+ `OpenMOSS-Team/MOSS-Audio-Tokenizer`（codec）。
- `supportedDevices: ["cuda", "cpu"]`（CUDA=生产路径 bf16/fp16+sdpa；CPU=本地无 GPU 测试 fp32+eager，慢仅验证流程）；`supportedFeatureList: ["streaming-speech"]`（无 TTS/克隆/设计/微调）。
- **CPU 加载须显式 `.to(device, dtype)` 统一 dtype**：基座 checkpoint 以 `bfloat16` 存储（`config.dtype=bfloat16`），且 `language_config.dtype=bfloat16` 传播到 Qwen3 `language_model` 子模型。transformers 5.0 加载时（`core_model_loading.py:1174`）对每参数取 `empty_param.dtype != 目标 dtype` 即回退到子模型 init dtype，故 CPU 上 `from_pretrained(torch_dtype=float32).to(device)` 会留下 **混合 dtype**：`embed_tokens` float32、`language_model` bfloat16。forward 时 float32 embed 输出撞 bf16 权重，报 `expected m1 and m2 to have the same dtype, but got: float != struct c10::BFloat16`（streaming.py `load_model_and_codec` 已改 `.to(torch_device, dtype)` 修复；CUDA 因目标 bf16 与子 config bf16 一致本无此问题）。
- **首个实现会话级 `streaming.py` 契约的适配器**：自写会话循环（读 `streaming.params.json` + `context.json` basic.speakers，模型加载前 `connect_session_socket` 连 Rust 环回 Socket 并 AUTH，随后阻塞 `read_frame` 收 INPUT 帧，按到达顺序按 `contextId` 合成，经 Socket 二进制帧回传 started/chunk/finished/error，不经 stdout、不轮询文件）。用法对齐上游 `example_multiturn_stream_to_tts.py`（`MossTTSRealtimeStreamingSession`/`MossTTSRealtimeInference`/`AudioStreamDecoder`）。多轮语义取最简：每消息独立合成（voice prompt + 文本 -> 音频），轮间不保 KV cache、不采集 user 音频。
- **chunk 字节格式**：首 chunk = 44 字节 WAV 头（data size 哨兵 `0xFFFFFFFF`）+ PCM16，后续 chunk = PCM16，前端 `useStreamableAudioPlayer` 累加成单个 `audio/wav` Blob 整播。
- **不提供微调**：上游 deepspeed 仅用于微调（`finetuning/sft.py` 的 ZeRO-3 可选路径）且在 Windows 构建安装受阻（pip build isolation 临时环境无 torch 触发 "Unable to pre-compile ops without torch installed"），故本适配器移除 `model-training` 能力与 `training.py`，仅保留推理。如需用已训练说话人，可外部产出 checkpoint 后按下条加载。
- **trained 说话人回接流式**：外部产出的 `<model_root_path>/<speaker_dir_name>/checkpoint_final/` 经 `StreamingSpeakerForm` trained 类别从 `list_speaker_infos`(status=Ready) 选择；streaming.py 加载该 checkpoint 合成。一会话至多一个 trained 说话人。
- 入口脚本: `streaming.py` / `download.py` / `common.py` / `params.py` / `params_entity.py`（含 `StreamingSpeech` TaskKind，显式映射 `TaskKind -> args 嵌套键`，因 kind="StreamingSpeech" 而 args 键="Streaming"）。
- 适配器纯逻辑单测: `src-model/moss_tts_realtime/tests/`（pytest，覆盖 configs/common/params_entity/params/wav_frames）。
- **打包与许可**：已纳入 Windows src-model 打包脚本 `scripts/windows/package_src_model.ps1`（`modelDirectories` 含 `moss_tts_realtime`）；子模块带 Apache 2.0 `LICENSE`（上游 OpenMOSS 许可）。

## 关联记忆
- [[project-overview]] - 项目全貌
- [[tech-stack-backend]] - Rust 后端 Pipeline 架构
- [[tech-stack-frontend]] - 前端参数表单
- [[data-flow-and-types]] - 数据流与类型
