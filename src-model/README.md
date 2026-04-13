# src-model

本目录存放本地训练所需的 Python 代码与平台脚本。

## 目录约定

- `src/encode_audio.py`: 训练前的音频编码预处理实现。
- `src/training.py`: 模型训练实现。
- `src/tts.py`: 文本转语音推理实现。
- `src/voice_clone.py`: 声音克隆推理实现。
- `src/ffmpeg.py`: 基于 `ffmpy` 的音频转码实现，用于封装音频格式转换等 ffmpeg 命令。
- `scripts/windows/init_task_runtime.ps1`: Windows 运行时初始化脚本，负责创建或复用 `venv`、安装 torch / requirements / modelscope，并校验当前 CPU 或 CUDA 运行时可用。
- `scripts/windows/download_models.ps1`: Windows 基础模型下载脚本，只负责下载 Qwen3-TTS 基础模型与 tokenizer。
- `scripts/unix/init_task_runtime.sh`: Unix-like 运行时初始化脚本。
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
- `download-models`: 使用 `modelscope download` 下载 Qwen3-TTS 基础模型与 tokenizer。
- `encode`: Rust 直接调用 `encode_audio.py`，需要 `--input-jsonl`、`--output-jsonl`、`--tokenizer-model-path`、`--device`。
- `train`: Rust 直接调用 `training.py`，需要 `--train-jsonl`、`--output-model-path`、`--init-model-path`、`--batch-size`、`--num-epochs`、`--speaker-name`、`--device`。
- `tts`: Rust 直接调用 `tts.py`。
- `voice-clone`: Rust 直接调用 `voice_clone.py`。
- `transcode`: Rust 直接调用 `ffmpeg.py`，由 `ffmpy` 驱动 `ffmpeg` 完成格式转换。

## 一次性初始化标记

- `init-task-runtime` 与 `download-models` 都成功后，Rust 才会把 `config.toml` 中的 `training.prepared_hardware_types` 写入当前硬件类型。
- 后续训练或声音克隆如果发现当前硬件类型已经在 `prepared_hardware_types` 中，并且 `venv` 与 `base-models/` 路径齐全，就会跳过这两个准备阶段。
- 如果用户手动删除了 `venv` 或 `base-models/`，需要先把 `training.prepared_hardware_types` 中对应的硬件类型移除，否则系统会误判为环境已准备完成。

## 模型运行配置

- `config.toml` 的 `[training]` 段支持 `attn_implementation`，默认值为 `sdpa`。
- 当前该配置统一作用于 Qwen 模型加载链路，包括文本转语音、声音克隆、训练、编码预处理。
- 建议可选值为 `sdpa`、`flash_attention_2`、`eager`；设置页会以下拉方式写入该字段。
