# src-model

本目录存放桌面端本地模型运行时，包括 Python 业务脚本、基础模型资源、平台初始化脚本与测试。当前运行时按模型拆分目录，主要包含 `qwen3_tts/` 与 `vox_cpm2/` 两条链路。

## 目录结构

- `qwen3_tts/`: Qwen3-TTS 运行时目录，包含训练、推理、声音克隆、音频编码与共享训练辅助脚本。
- `vox_cpm2/`: VoxCPM2 运行时目录，包含训练、推理、声音克隆、转码与本地化训练入口脚本。
- `base-models/`: 本地基础模型资源目录，当前包含 Qwen3-TTS 各变体、Tokenizer 与 VoxCPM2 基础模型。
- `scripts/windows/`: Windows 平台初始化、模型下载、LoRA 依赖切换与打包脚本。
- `scripts/unix/`: Unix-like 平台初始化、模型下载、LoRA 依赖切换与打包脚本。
- `tests/`: Python 侧测试，覆盖 ffmpeg、训练配置与 VoxCPM2 训练/运行时解析等行为。
- `vendor/`: 预留给第三方源码或离线资产的目录；当前运行时不再依赖其动态拉取 VoxCPM 训练源码。

## 模型运行时

- `qwen3_tts/` 主要脚本：
  - `encode_audio.py`: 训练前音频编码预处理。
  - `training.py`: 训练入口；内部再分发到 `training_full.py`、`training_lora.py` 与 `training_common.py`。
  - `tts.py`: 文本转语音推理。
  - `voice_clone.py`: 声音克隆推理。
  - `ffmpeg.py`: 基于 `ffmpy` 的音频转码封装。
- `vox_cpm2/` 主要脚本：
  - `training.py`: 训练外层入口；负责参数解析、训练配置生成与 checkpoint/runtime metadata 收尾。
  - `train_voxcpm_finetune.py`: 本地化维护的 VoxCPM 官方训练入口副本。运行时优先使用该脚本，不再通过 git clone 或 GitHub 压缩包动态拉取上游源码。
  - `tts.py`: 文本转语音推理。
  - `voice_clone.py`: 声音克隆推理。
  - `ffmpeg.py`: 音频转码辅助脚本。
- 当前 VoxCPM2 的离线前提是运行环境中已安装 `voxcpm` 及其训练依赖；训练脚本本身已随仓库分发。

## Rust 与脚本职责边界

- Rust 负责训练任务状态流转、路径解析、参数组织、错误包装。
- Rust 直接调用平台脚本，以及模型目录下的 Python 业务脚本。
- 平台脚本负责运行时准备、依赖安装、基础模型下载与打包辅助。
- Python 脚本负责编码、训练、文本转语音、声音克隆与转码等业务逻辑。

## 脚本动作

本地模型任务按固定阶段执行：

- `init-task-runtime`: 通过 `scripts/windows/init_task_runtime.ps1` 或 `scripts/unix/init_task_runtime.sh` 创建或复用指定 `--base-model` 的 `venv`，安装该模型目录的 `requirements.txt`，并校验 torch 运行时。脚本支持 `--base-model`、`--requirements-file`、`--cpu-mode`、`--task-log-file` 等参数。
- `toggle-lora-dependencies`: 通过平台脚本按 `enable` 或 `disable` 安装/卸载 LoRA 相关依赖。
- `download-models`: 通过平台脚本下载基础模型资源；当前主要用于 Qwen3-TTS 链路。
- `package-src-model`: 通过平台脚本打包 `src-model` 运行时资产。
- `encode`: Rust 直接调用 `qwen3_tts/encode_audio.py`。
- `train`: Rust 直接调用模型目录下的 `training.py`。Qwen3-TTS 与 VoxCPM2 训练入口相同命名，但内部实现不同。
- `tts`: Rust 直接调用模型目录下的 `tts.py`。
- `voice-clone`: Rust 直接调用模型目录下的 `voice_clone.py`。
- `transcode`: Rust 直接调用模型目录下的 `ffmpeg.py`，由 `ffmpy` 驱动 `ffmpeg` 完成格式转换。

## 模型运行配置

- `config.toml` 的 `[training]` 段支持 `hardware_type` 与 `attn_implementation`，并由设置页统一维护。
- `[training]` 段还支持 `lora_mode`、`lora_rank`、`lora_alpha`、`lora_dropout`。
- 这些配置会影响模型训练与推理链路；具体生效范围由各模型目录下的实现决定。
- 建议可选值为 `sdpa`、`flash_attention_2`、`eager`；设置页会以下拉方式写入该字段。
- `hardware_type` 是全局设置项，功能页不会再按任务覆盖；切换硬件后，新任务统一使用该配置。
- `lora_mode` 只支持 `enabled`、`disabled`：`enabled` 会在保存设置时立即同步依赖；`disabled` 会卸载相关依赖并固定走全量微调。

## 依赖与离线说明

- 每个模型目录维护自己的 `requirements.txt` 与 `venv/`。
- `qwen3_tts/requirements.txt` 与 `vox_cpm2/requirements.txt` 独立安装，避免不同模型链路互相污染。
- 如需验证 Python 侧行为，优先运行 `tests/` 下的窄范围测试，避免直接触发重量级模型训练。
