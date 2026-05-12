# Kirine Client 用户手册

Kirine Client 是 Kirine（桐音）音频工作台的桌面客户端，面向本地文本转语音、声音克隆、模型训练和任务管理场景。

当前支持模型和预置说话人以 [supported_models.json](supported_models.json) 为准，应用启动时会自动同步到本地数据库。

## 1. 使用前须知

1. 当前以 Windows 10/11 本地模式为主。
2. 项目仍在快速迭代，版本间可能不完全兼容。
3. 首次安装模型、首次推理或首次训练通常会较慢（依赖准备和模型检查）。
4. 训练任务建议使用 GPU；CPU 可运行但速度会明显下降。

## 2. 当前模型支持矩阵

| 基础模型 | 版本 | 文本转语音 | 声音克隆 | 模型训练 | 状态说明 |
| --- | --- | --- | --- | --- | --- |
| qwen3_tts | 1.7B | 支持 | 支持 | 支持 | 稳定可用 |
| qwen3_tts | 0.6B | 支持 | 支持 | 支持 | 稳定可用 |
| vox_cpm2 | 2B | 支持 | 支持 | 支持 | 稳定可用（支持 LoRA 参数链路） |
| moss_tts_local | 1.7B | 支持 | 支持 | 支持 | 稳定可用 |
| gpt_sovits_cpufast | V1 | 支持 | 支持 | 不支持 | 推荐使用版本 |
| gpt_sovits_cpufast | V2 / V2Pro / V2ProPlus | 支持（实验） | 支持（实验） | 不支持 | 默认不保障，需手工适配 |

## 3. 快速上手

1. 打开设置页，确认 `data_dir`、`log_dir`、`model_dir`。
2. 打开模型管理页，安装目标基础模型。
3. 文本转语音和声音克隆先确认已有可用说话人。
4. 模型训练先准备样本，再在训练页创建任务。
5. 在历史任务页统一查看结果、导出音频或复用参数。

## 4. 功能页面说明

### 4.1 模型管理

1. 查看当前支持模型与版本。
2. 安装/卸载模型。
3. 安装完成后对应功能自动在任务页可用。

### 4.2 文本转语音

1. 选择基础模型、模型版本、说话人。
2. 选择语言与输出格式。
3. 输入文本并提交任务。
4. 在历史任务查看结果。

### 4.3 声音克隆

1. 选择基础模型、模型版本、语言、输出格式。
2. 上传参考音频并填写参考文本。
3. 输入目标文本并提交。

参考音频格式：`wav`、`mp3`、`flac`、`ogg`。

### 4.4 模型训练

1. 选择语言、基础模型、模型版本。
2. 填写模型名称、描述和训练参数。
3. 导入样本并显式指定参考音频（`refAudioPath`）。
4. 启动训练并在历史任务页跟踪状态。

样本导入格式：
1. 音频：`wav`、`mp3`、`flac`、`ogg`
2. 标注：`jsonl`、`xlsx`、`xls`

### 4.5 说话人管理

1. 查看说话人数量、状态和样本统计。
2. 按关键词、语言、状态筛选。
3. 编辑名称和描述。
4. 删除不再使用的用户说话人。

### 4.6 历史任务

1. 统一查看训练、文本转语音、声音克隆任务。
2. 查看详情、试听结果、导出文件。
3. 回填历史参数再次执行。

## 5. GPT-SoVITS-CPUFast 特别说明

### 5.1 额外依赖要求

使用 GPT-SoVITS-CPUFast 前，请确保系统已安装 Git（并在 PATH 中可用），因为该模型采用自定义下载流程，会自动执行仓库克隆。

### 5.2 关于 V2 及以上版本

由于 GPT-SoVITS-CPUFast 上游仍在持续开发，Kirine 当前默认仅保障 V1 使用体验。V2 / V2Pro / V2ProPlus 作为实验能力，如果想使用，可以依照下面的方法：

首先，找到模型目录的chinese2.py脚本，正常情况下路径为：src-model\base-models\gpt_sovits_cpufast\GPT_SoVITS/text/chinese2.py

找到该脚本后，修改两行代码（第34行开始）：
```python
if is_g2pw:
    # print("当前使用g2pw进行拼音推理")
    # from text.g2pw.torch_api import G2PWTorchConverter --这一行改成下面的代码，修改原因是原作者未来考虑使用torch_api来实现相关推理流程，但是目前模型下载到的G2PW依旧是onnx实现，不改的话V2及以上版本执行任务时会因为找不到pth格式的权重报错，目前临时切回onnx_api可以解决这个问题，随着项目推进，未来可能会直接兼容
    from text.g2pw.onnx_api import G2PWOnnxConverter
    from text.g2pw.pronunciation import correct_pronunciation, get_phrase_pronunciation

    parent_directory = os.path.dirname(current_file_path)
    # g2pw = G2PWTorchConverter( --这一行改成下面的代码
    g2pw = G2PWOnnxConverter(
        model_dir="GPT_SoVITS/text/G2PWModel",
        style="pinyin",
        model_source=os.environ.get("bert_path", "GPT_SoVITS/pretrained_models/chinese-roberta-wwm-ext-large"),
        enable_non_tradional_chinese=True,
    )
```

## 6. 已支持模型项目指引

以下为 Kirine 当前支持模型对应的项目/模型仓库入口：

1. Qwen3-TTS
   1) 项目入口（GitHub）：https://github.com/QwenLM/Qwen3-TTS
   2) 模型仓库（Hugging Face）：https://huggingface.co/Qwen
2. VoxCPM2
   1) 模型仓库（Hugging Face）：https://huggingface.co/openbmb/VoxCPM2
3. MOSS-TTS Local
   1) 模型仓库（Hugging Face）：https://huggingface.co/OpenMOSS-Team
4. GPT-SoVITS-CPUFast
   1) 项目仓库（GitHub）：https://github.com/baicai-1145/GPT-SoVITS-CPUFast

## 7. 配置说明

主配置文件见 [config.toml](config.toml)。

```toml
[basic]
mode = "local"
data_dir = 'D:\Project\temp\kirine-client\data'
log_dir = 'D:\Project\temp\kirine-client\logs'
model_dir = 'D:\Project\temp\kirine-client\models'

[training]
hardware_type = "cpu"
attn_implementation = "sdpa"
```

字段说明：
1. `data_dir`：任务数据、样本、数据库目录。
2. `log_dir`：应用与任务日志目录。
3. `model_dir`：本地模型存储目录。
4. `hardware_type`：运行硬件类型（`cpu` / `gpu`）。
5. `attn_implementation`：注意力实现（默认 `sdpa`）。

## 8. 常见问题

### 8.1 首次运行慢

首次运行或首次调用模型会进行依赖准备和模型检查，属于正常现象。

### 8.2 任务长时间未完成

1. 先查看页面通知和任务状态是否持续刷新。
2. 确认是否为首次安装/首次推理/首次训练。
3. 若修改过目录配置，请重启应用后重试。
4. 查看 `log_dir` 下任务日志排查。

### 8.3 flash-attn 能否直接使用

Windows 环境下通常缺少稳定官方支持，默认建议继续使用 `sdpa`。

## 9. 开发命令

```bash
npm install
npm run dev
npm run tauri dev
npm run build
```
