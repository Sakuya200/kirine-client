# src-model

本目录存放 Kirine Client 的本地模型运行时，包括 Python 业务脚本、基础模型资产、平台脚本与测试。当前运行时按基础模型拆分目录，并由 Rust 侧在任务执行时按 `base_model + model_version` 组合调度。

各基础模型的业务实现以 **Git 子模块**形式挂载在 `src-model/` 下（独立 adapter 仓库），克隆主仓库时需带 `--recurse-submodules`。

## 目录结构

- `irodori_tts_v3/`：Irodori-TTS-V3 运行时目录，包含训练、推理、声音克隆、音色设计与下载脚本。
- `dots_tts/`：Dots.TTS 运行时目录，包含训练、推理、声音克隆、数据集与训练辅助脚本。
- `qwen3_tts/`：Qwen3-TTS 运行时目录，包含训练、推理、声音克隆、音频编码与共享训练辅助脚本。
- `vox_cpm2/`：VoxCPM2 运行时目录，包含训练、推理、声音克隆与本地化训练入口脚本。
- `moss_tts_local/`：MOSS-TTS Local 运行时目录，包含训练、推理、声音克隆和微调辅助脚本。
- `gpt_sovits_cpufast/`：GPT-SoVITS-CPUFast 运行时目录，包含推理、声音克隆及相关参数脚本。
- `base-models/`：基础模型权重与离线资产目录。
- `scripts/windows/`：Windows 平台脚本，负责初始化运行时、下载模型、校验 torch 环境、音频转码与打包。
- `scripts/unix/`：Unix-like 平台脚本，职责与 Windows 版本对应。
- `tests/`：Python 侧测试，覆盖参数实体、训练配置与部分训练链路行为。

## 模型元数据来源

应用不会在 Rust 侧硬编码支持模型。当前模型目录下的 `configs/model-config.json` 与 `configs/params-config.json` 会在应用启动时被扫描，并同步为：

1. 模型基础信息
2. 支持功能列表
3. 支持设备列表 `supportedDevices`
4. 预置说话人信息
5. 前端任务参数表单定义

当前已接入基础模型：

1. `irodori_tts_v3`
2. `dots_tts`
3. `qwen3_tts`
4. `vox_cpm2`
5. `moss_tts_local`
6. `gpt_sovits_cpufast`

## 模型运行时概览

### irodori_tts_v3

- `tts.py`：文本转语音推理（RF Euler 流匹配采样）。
- `voice_clone.py`：声音克隆推理。
- `voice_design.py`：音色设计推理（使用独立的 VoiceDesign 权重）。
- `training.py`：模型训练入口。
- `download.py`：克隆上游 `Aratako/Irodori-TTS` 并下载基座 / 音色设计权重，预热 codec 与文本 tokenizer。
- 上游项目：https://github.com/Aratako/Irodori-TTS

### dots_tts

- `__init__.py`：文本转语音推理入口。
- `voice_clone.py`：声音克隆推理。
- `training.py`：训练外层入口。
- `training_common.py`：训练共享逻辑（Dataset / DataLoader / Optimizer / Scheduler）。
- `dataset.py`：训练数据集（JSONL manifest）。
- `download.py`：下载 `rednote-hilab/dots.tts` 系列权重。
- 上游项目：https://github.com/rednote-hilab/dots.tts

### qwen3_tts

- `encode_audio.py`：训练前音频编码预处理。
- `training.py`：训练外层入口，当前统一走全量微调实现。
- `training_common.py` / `training_full.py`：训练共享逻辑。
- `tts.py`：文本转语音推理。
- `voice_clone.py`：声音克隆推理。

### vox_cpm2

- `training.py`：训练入口，负责参数解析、配置生成与训练后收尾。
- `train_voxcpm_finetune.py`：本地维护的 VoxCPM2 训练入口副本。
- `tts.py`：文本转语音推理。
- `voice_clone.py`：声音克隆推理。

### moss_tts_local

- `training.py`：模型训练入口。
- `tts.py`：文本转语音推理。
- `voice_clone.py`：声音克隆推理。
- `finetuning/`：与训练相关的辅助实现。

### gpt_sovits_cpufast

- `tts.py`：文本转语音推理。
- `voice_clone.py`：声音克隆推理。
- `download.py` / `inference_bridge.py`：下载与推理桥接逻辑。
- 当前仅支持 CPU 运行环境。

## Rust、平台脚本与 Python 的职责边界

当前链路分工如下：

1. Rust 负责任务状态流转、数据库记录、参数组织、路径解析、错误包装与 Tauri command 暴露。
2. 平台脚本负责初始化/复用每个模型目录下的 Python 运行时（`venv` 或 `conda_env`）、安装依赖、校验或切换 torch 运行时、下载基础模型、转码与打包。
3. Python 脚本负责编码、训练、文本转语音和声音克隆等业务逻辑。
4. 音频转码由平台脚本统一处理，并直接驱动系统中的 `ffmpeg`。

## 平台脚本动作

本地模型任务按固定阶段执行，当前主要脚本如下：

1. `init_task_runtime.ps1` / `init_task_runtime.sh`

- 探测系统 PATH 是否存在 `conda`：存在则为指定 `--base-model` 创建 `conda_env`（`conda create --prefix <model>/conda_env python=3.12`），否则创建标准 `venv`
- 安装模型目录中的 `requirements.txt`
- 为后续任务准备基础运行环境

1. `ensure_torch_runtime.ps1` / `ensure_torch_runtime.sh`

- 校验当前模型环境中的 torch 是否满足 CPU 或 CUDA 任务要求
- 按需要结合模型目录中的 `requirements-torch.txt` 切换 torch 运行时版本
- 当前还支持只读查询模式，用于返回当前模型环境的设备类型，输出约定为 `DEVICE_TYPE|cpu` 或 `DEVICE_TYPE|cuda`

1. `download_models.ps1` / `download_models.sh`

- 下载基础模型资源与相关离线资产

1. `transcode_audio.ps1` / `transcode_audio.sh`

- 统一音频格式转换

1. `begin_llm_task.ps1`（Windows）/ `begin_llm_task.sh`（Unix，规划中）

- LLM 任务执行的统一包装器，由 Rust Pipeline 调用
- 解析 `--base-model` / `--script-path` / `--params-file` / `--log-path` / `--task-log-file`
- 优先探测 `<model>/conda_env/python(.exe)`，存在则用 conda env，否则回退 `<model>/venv`
- 为 Python 注入 `-X utf8 -X faulthandler -u`，将 stdout/stderr 追加写入 `--task-log-file`

1. `package_src_model.ps1` / 对应 Unix 打包脚本

- 打包 `src-model` 运行时资产

## 当前运行时行为

1. 每个基础模型维护独立 `requirements.txt`、`requirements-torch.txt` 与 Python 运行时（`venv/` 或 `conda_env/`），避免不同模型链路互相污染。
2. 运行时类型由系统是否安装 Conda 决定：PATH 中可检测到 `conda` 时优先创建 `conda_env`，否则创建标准 `venv`。后续所有阶段（torch 校验、下载、任务执行）均先探测 `conda_env` 的 python，存在则使用，否则回退 `venv`。Windows 下 conda env 通过 `conda run --prefix <conda_env> python --` 调用。
3. `requirements.txt` 只负责基础 Python 依赖；`requirements-torch.txt` 负责模型自定义的 Torch 运行时依赖，实际 CPU/CUDA 轮子来源仍由运行时脚本通过 `index-url` 选择。
4. 任务设备类型已经改为任务级参数，而不是全局设置。
5. 任务创建前，前端会先调用后端 `get_device_type`，后端再通过 `ensure_torch_runtime` 的查询模式探测当前模型环境里的 torch 设备类型。
6. 如果当前模型环境与任务选择的设备类型不一致，用户确认后才会继续执行，并由后续运行时阶段自动切换 torch 环境。
7. `task_history.device` 是任务最终执行设备的记录来源，历史详情会展示该字段。
8. LLM 任务统一由 `begin_llm_task` 包装器脚本驱动 Python，而非直接调用 venv python，以便支持 conda 环境选择与统一日志写入。

## 配置说明

当前 `config.toml` 中与运行时直接相关的训练配置只保留：

```toml
[training]
attn_implementation = "sdpa"
```

说明：

1. 全局 `hardware_type` 已移除，不再通过设置页统一指定所有任务的硬件类型。
2. `attn_implementation` 仍由设置页维护，可选值通常包括 `sdpa`、`flash_attention_2`、`eager`。
3. 具体参数是否生效以及生效范围，由各模型目录中的实现决定。

## 依赖与测试说明

1. 推荐统一使用 Python 3.12.x。
2. 若安装了 Miniconda / Anaconda 且 `conda` 在 PATH 中，运行时会优先使用 conda 环境，否则使用标准 `venv`。两者均不需要额外配置即可工作。
3. 某些模型下载或准备流程依赖 Git，请确保系统 PATH 中可用；各模型实现以 Git 子模块形式挂载，克隆主仓库时需带 `--recurse-submodules`。
4. 如需验证 Python 侧行为，优先运行 `tests/` 下的窄范围测试，避免直接触发重量级训练任务。
5. 若仅需验证前后端联调，应优先使用仓库根目录的 `npm run tauri dev` 与 `src-tauri/` 下的 `cargo check`。
