# src-model

本目录存放 Kirine Client 的本地模型运行时，包括 Python 业务脚本、基础模型资产、平台脚本与测试。当前运行时按基础模型拆分目录，并由 Rust 侧在任务执行时按 `base_model + model_version` 组合调度。

## 目录结构

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

1. `qwen3_tts`
2. `vox_cpm2`
3. `moss_tts_local`
4. `gpt_sovits_cpufast`

## 模型运行时概览

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
2. 平台脚本负责初始化/复用每个模型目录下的 `venv`、安装依赖、校验或切换 torch 运行时、下载基础模型、转码与打包。
3. Python 脚本负责编码、训练、文本转语音和声音克隆等业务逻辑。
4. 音频转码由平台脚本统一处理，并直接驱动系统中的 `ffmpeg`。

## 平台脚本动作

本地模型任务按固定阶段执行，当前主要脚本如下：

1. `init_task_runtime.ps1` / `init_task_runtime.sh`
  - 创建或复用指定 `--base-model` 的 `venv`
  - 安装模型目录中的 `requirements.txt`
  - 为后续任务准备基础运行环境
2. `ensure_torch_runtime.ps1` / `ensure_torch_runtime.sh`
  - 校验当前模型环境中的 torch 是否满足 CPU 或 CUDA 任务要求
  - 按需要切换 torch 运行时版本
  - 当前还支持只读查询模式，用于返回当前模型环境的设备类型，输出约定为 `DEVICE_TYPE|cpu` 或 `DEVICE_TYPE|cuda`
3. `download_models.ps1` / `download_models.sh`
  - 下载基础模型资源与相关离线资产
4. `transcode_audio.ps1` / `transcode_audio.sh`
  - 统一音频格式转换
5. `package_src_model.ps1` / 对应 Unix 打包脚本
  - 打包 `src-model` 运行时资产

## 当前运行时行为

1. 每个基础模型维护独立 `requirements.txt` 与 `venv/`，避免不同模型链路互相污染。
2. 任务设备类型已经改为任务级参数，而不是全局设置。
3. 任务创建前，前端会先调用后端 `get_device_type`，后端再通过 `ensure_torch_runtime` 的查询模式探测当前模型环境里的 torch 设备类型。
4. 如果当前模型环境与任务选择的设备类型不一致，用户确认后才会继续执行，并由后续运行时阶段自动切换 torch 环境。
5. `task_history.device` 是任务最终执行设备的记录来源，历史详情会展示该字段。

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
2. 某些模型下载或准备流程依赖 Git，请确保系统 PATH 中可用。
3. 如需验证 Python 侧行为，优先运行 `tests/` 下的窄范围测试，避免直接触发重量级训练任务。
4. 若仅需验证前后端联调，应优先使用仓库根目录的 `npm run tauri dev` 与 `src-tauri/` 下的 `cargo check`。
