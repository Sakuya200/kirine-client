# src-model

本目录存放本地训练所需的 Python 代码与平台脚本。

## 目录约定

- `src/encode_audio.py`: 训练前的音频编码预处理实现。
- `src/training.py`: 模型训练实现。
- `src/tts.py`: 文本转语音推理实现。
- `src/voice_clone.py`: 声音克隆推理实现。
- `src/ffmpeg.py`: 基于 `ffmpy` 的音频转码实现，用于封装音频格式转换等 ffmpeg 命令。
- `scripts/windows/init_task_runtime.ps1`: Windows 运行时初始化脚本，负责创建或复用 `venv`、安装 torch / requirements / modelscope，并校验当前 CPU 或 CUDA 运行时可用。
- `scripts/windows/toggle_qlora_dependencies.ps1`: Windows QLoRA 依赖切换脚本，按启用/禁用安装或卸载 `peft`、`bitsandbytes`。
- `scripts/windows/download_models.ps1`: Windows 基础模型下载脚本，只负责下载 Qwen3-TTS 基础模型与 tokenizer。
- `scripts/unix/init_task_runtime.sh`: Unix-like 运行时初始化脚本。
- `scripts/unix/toggle_qlora_dependencies.sh`: Unix-like QLoRA 依赖切换脚本。
- `scripts/unix/download_models.sh`: Unix-like 基础模型下载脚本。
- `base-models/`: 通过 `modelscope` 下载的 Qwen3-TTS 基础模型与 tokenizer 本地缓存目录。

## Rust 与脚本职责边界

- Rust 负责训练任务状态流转、路径解析、参数组织、错误包装。
- Rust 直接调用运行时准备脚本，以及 `src/` 下的 Python 业务脚本。
- 平台脚本只负责运行时准备和基础模型下载。
- Python 文件负责编码、训练、文本转语音、声音克隆与转码业务逻辑。

## 脚本动作

本地训练按固定阶段执行：

- `init-task-runtime`: 创建或复用 `venv`，根据模式选择 CPU 或 NVIDIA GPU / CUDA 版 torch，安装 `requirements.txt` 与 `modelscope`，并校验 torch 运行时。
- `toggle-qlora-dependencies`: 由设置页保存动作直接触发，按 `enable` 或 `disable` 安装/卸载 QLoRA 相关依赖。
- `download-models`: 使用 `modelscope download` 下载 Qwen3-TTS 基础模型与 tokenizer。
- `encode`: Rust 直接调用 `encode_audio.py`，需要 `--input-jsonl`、`--output-jsonl`、`--tokenizer-model-path`、`--device`。
- `train`: Rust 直接调用 `training.py`，需要 `--train-jsonl`、`--output-model-path`、`--init-model-path`、`--batch-size`、`--num-epochs`、`--speaker-name`、`--device`。
- `train`: QLoRA 只支持显式启用或禁用。启用时必须先通过切换脚本安装 `peft` 与 `bitsandbytes`，禁用时固定走全量微调。
- `tts`: Rust 直接调用 `tts.py`。
- `voice-clone`: Rust 直接调用 `voice_clone.py`。
- `transcode`: Rust 直接调用 `ffmpeg.py`，由 `ffmpy` 驱动 `ffmpeg` 完成格式转换。

## 模型运行配置

- `config.toml` 的 `[training]` 段支持 `hardware_type` 与 `attn_implementation`，默认值分别为 `cuda` 与 `sdpa`。
- `config.toml` 的 `[training]` 段还支持 `qlora_mode`、`qlora_rank`、`qlora_alpha`、`qlora_dropout`、`qlora_quant_type`、`qlora_double_quant`，并由设置页统一维护。
- 当前这些配置统一作用于 Qwen 模型加载链路，包括文本转语音、声音克隆、训练、编码预处理。
- 建议可选值为 `sdpa`、`flash_attention_2`、`eager`；设置页会以下拉方式写入该字段。
- `hardware_type` 是全局设置项，功能页不会再按任务覆盖；切换硬件后，新任务统一使用该配置。
- `qlora_mode` 只支持 `enabled`、`disabled`：`enabled` 会在保存设置时立即同步依赖并要求 CUDA 环境；`disabled` 会卸载相关依赖并固定走全量微调。
- QLoRA 目标模块集合按当前 Qwen3-TTS 模型结构固定写在 `src/qwen3_tts/training.py` 中，不通过设置页暴露，避免不同模型结构下误配。
