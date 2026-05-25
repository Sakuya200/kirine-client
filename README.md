# Kirine Client 用户手册

Kirine Client 是 Kirine（桐音）音频工作台的桌面客户端，支持本地文本转语音、声音克隆、模型训练、说话人管理和历史任务管理。

本项目的目标是实现一个通用的音频合成模型UI以及调度层实现，提供开箱即用的多模型音频合成功能，未来将引入更多的音频合成相关功能，b站功能介绍视频地址：https://www.bilibili.com/video/BV1MwLy6yEzU

---

## 1. 前置环境准备

在首次使用 Kirine Client 之前，请先安装以下程序并确保它们已加入系统 PATH：

1. **Python 3.12.x**：用于本地模型的运行环境初始化与任务执行，建议统一使用 3.12.x 版本。
2. **Git**：用于部分模型的资源获取流程，特别是 GPT-SoVITS-CPUFast。

## 2. 使用前须知

1. 当前以 Windows 10/11 本地模式运行。
2. 首次安装模型、首次推理或首次训练通常会较慢，这是正常现象。
3. 每项任务可以单独选择使用 CPU 或 CUDA（GPU）运行；若所选设备与当前模型运行环境不一致，提交前会出现确认弹窗，确认后应用会自动切换，但本次任务启动时间会明显增加。
4. 模型训练建议使用 GPU；CPU 可以运行，但速度会明显下降。

## 3. 当前模型支持

| 基础模型 | 版本 | 文本转语音 | 声音克隆 | 模型训练 | 设备支持 |
| --- | --- | --- | --- | --- | --- |
| Qwen3-TTS | 1.7B | ✓ | ✓ | ✓ | CPU / CUDA |
| Qwen3-TTS | 0.6B | ✓ | ✓ | ✓ | CPU / CUDA |
| VoxCPM2 | 2B | ✓ | ✓ | ✓ | CPU / CUDA |
| MOSS-TTS Local | 1.7B | ✓ | ✓ | ✓ | CPU / CUDA |
| GPT-SoVITS-CPUFast | V1 | ✓ | ✓ | — | CPU |
| GPT-SoVITS-CPUFast | V2 / V2Pro / V2ProPlus | ✓（实验） | ✓（实验） | — | CPU |

## 4. 快速上手

1. 启动后先打开**设置**页，确认数据、日志、模型目录是否符合当前机器的目录规划。
2. 打开**模型管理**页，安装要使用的模型。
3. **文本转语音**：选择模型、说话人、语言，输入文本后提交。
4. **声音克隆**：上传参考音频，填写目标台词后提交；参考音频支持 `wav`、`mp3`、`flac`、`ogg`。
5. **模型训练**：先导入样本（支持单样本和批量数据集两种方式），再填写说话人名称和参数后启动训练。
6. 所有任务的状态、结果和导出操作可以在**历史任务**页统一管理，也可以从历史记录直接回填参数重新提交。

## 5. 功能页面说明

### 5.1 模型管理

查看当前支持的模型及其安装状态，可执行安装、重装或卸载。模型安装完成后，对应功能会自动在任务页启用。

### 5.2 文本转语音

选择模型、版本、设备类型、语言和输出格式，可选择说话人，输入文本后提交任务。任务结果可在页面右侧结果卡片实时查看，也会出现在历史任务中。

### 5.3 声音克隆

选择模型、版本、设备类型、语言和输出格式，上传参考音频，可选填写参考文本，输入目标台词后提交。

### 5.4 模型训练

选择语言、模型和设备类型，填写说话人名称和描述，通过"单样本（音频 + 台词）"或"样本集（音频压缩包 + 标注文件）"的方式导入训练数据后启动。

标注文件支持格式：`jsonl`、`xlsx`、`xls`。

### 5.5 说话人管理

查看、搜索、编辑和删除本地说话人，支持按语言和状态筛选。

### 5.6 历史任务

统一查看所有任务的状态、详情和结果，支持试听、导出音频，以及从历史记录回填参数重新发起任务。

## 6. GPT-SoVITS-CPUFast 特别说明

当前默认仅保障 V1 使用体验。若要使用 V2 / V2Pro / V2ProPlus，找到以下文件：

```
src-model\base-models\gpt_sovits_cpufast\GPT_SoVITS\text\chinese2.py
```

将 `g2pw` 的导入和实例化改为 `onnx_api`：

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

这是对上游实验版本的临时兼容方案，后续可能随项目演进而调整。

## 7. 已支持模型项目指引

1. Qwen3-TTS
   - 项目入口（GitHub）：https://github.com/QwenLM/Qwen3-TTS
   - 模型仓库（Hugging Face）：https://huggingface.co/Qwen
2. VoxCPM2
   - 模型仓库（Hugging Face）：https://huggingface.co/openbmb/VoxCPM2
3. MOSS-TTS Local
   - 模型仓库（Hugging Face）：https://huggingface.co/OpenMOSS-Team
4. GPT-SoVITS-CPUFast
   - 项目仓库（GitHub）：https://github.com/baicai-1145/GPT-SoVITS-CPUFast

## 8. 常见问题

**首次运行慢**：首次调用模型时会自动创建 Python 虚拟环境并安装依赖，属于正常现象，后续调用不会重复这个过程。

**任务长时间无进展**：检查页面通知栏是否有错误提示，或查看配置中 `log_dir` 下对应任务的日志文件。若为首次安装、首次推理或切换了设备类型，启动耗时增加是正常的。

**flash-attn 相关报错**：Windows 环境下通常缺少稳定官方支持，建议保持默认的 `sdpa`。

---

## 开发者文档

本节面向需要在本地构建或调试 Kirine Client 的开发者。

### D1. 开发环境要求

除上文用户侧的 Python 3.12.x 和 Git 外，还需要安装：

1. **Node.js**：推荐 LTS 版本，用于前端构建。
2. **Rust 工具链**：通过 rustup 安装，Tauri 2 要求稳定版。
3. **Tauri 依赖**：Windows 上需要 Microsoft C++ Build Tools 和 WebView2，参考 [Tauri 官方前置文档](https://v2.tauri.app/start/prerequisites/)。

### D2. 准备打包资源（必做）

Tauri 在本地构建和开发模式下都会校验 `bundle.resources` 中列出的文件是否存在于 `src-tauri/resources/`。**若以下文件缺失，`npm run tauri dev` 和 `npm run tauri build` 均会报错终止。**

需要手工准备的文件如下：

```
src-tauri/resources/
├── config.toml                          # 可以复制仓库中的文件
├── ffmpeg-8.0.1-essentials_build.zip    # ffmpeg Windows 构建包
├── sox-14.4.2-win32.zip                 # SoX Windows 构建包
└── src-model-runtime.zip                # Python 模型运行时打包
```

获取方式：

- `ffmpeg-8.0.1-essentials_build.zip`：从 https://www.gyan.dev/ffmpeg/builds/ 下载 essentials build，版本号必须与文件名一致。
- `sox-14.4.2-win32.zip`：从 https://sourceforge.net/projects/sox/files/sox/ 下载对应版本的 Windows 包。
- `src-model-runtime.zip`：是仓库 `src-model/` 目录的打包产物。在开发阶段应用会自动识别 workspace 根目录下的 `src-model/`（即仓库 clone 下来的目录），因此本地开发时该 zip 文件缺失不影响运行时功能，但 Tauri 在启动/构建前会检查文件存在性。可以先创建一个空 zip 文件占位，或联系项目维护者获取正式的运行时包。

### D3. 克隆与初始化

```bash
git clone <repo_url>
cd kirine-client
npm install
```

完成后，按 D2 节说明准备好 `src-tauri/resources/` 下的文件。

### D4. 启动开发环境

```bash
npm run tauri dev
```

会同时启动 Vite 前端开发服务器和 Tauri 桌面端，前端热更新实时生效，Rust 端修改后会自动重新编译并重启。

### D5. 构建

```bash
npm run tauri build
```

会先执行 `vue-tsc --noEmit` 类型检查，再通过 Vite 构建前端产物，最终由 Tauri 打包为 NSIS 安装包。产物输出到 `src-tauri/target/release/bundle/`。

### D6. 仅验证编译

如果只需要检查代码是否编译通过而不需要完整启动：

```bash
# Rust 侧
cd src-tauri
cargo check

# 前端类型检查
npx vue-tsc --noEmit
```

### D7. 配置文件说明

项目根目录的 `config.toml` 是运行时配置文件，应用启动时从当前目录查找。

```toml
[basic]
mode = "local"
data_dir = 'D:\Project\temp\kirine-client\data'
log_dir = 'D:\Project\temp\kirine-client\logs'
model_dir = 'D:\Project\temp\kirine-client\models'

[training]
attn_implementation = "sdpa"
```

- `data_dir`：任务数据、样本、SQLite 数据库与生成音频的存储目录。
- `log_dir`：应用日志与任务日志目录，调试时优先查这里。
- `model_dir`：本地模型权重的存储目录。
- `training.attn_implementation`：注意力实现，可选 `sdpa`（默认）、`flash_attention_2`、`eager`。

### D8. 日志与调试

- **Rust 后端日志**：写入 `log_dir` 下的日志文件，任务执行日志按任务 ID 单独存放。
- **前端 / Tauri 日志**：开发模式下直接输出到终端；生产模式下同样写入 `log_dir`。
- **Python 脚本日志**：每次任务执行时，脚本的标准输出会被重定向到任务专属日志文件，路径格式为 `log_dir/<task_id>/`。
- **模型运行时查找**：Rust 启动时会依次尝试 `<workspace>/src-model`、`<app_dir>/src-model`、`<app_dir>/lib/src-model`，本地开发通常命中第一个路径（仓库目录下的 `src-model/`）。
