# Kirine Client 用户手册

Kirine Client 是 Kirine（桐音）音频工作台的桌面客户端，支持本地文本转语音、声音克隆、音色设计、模型训练、流式语音会话、说话人管理和历史任务管理。

本项目的目标是实现一个通用的音频合成模型UI以及调度层实现，提供开箱即用的多模型音频合成功能，未来将引入更多的音频合成相关功能，b站功能介绍视频地址：https://www.bilibili.com/video/BV1MwLy6yEzU

---

## 1. 前置环境准备

在首次使用 Kirine Client 之前，请先安装以下程序并确保它们已加入系统 PATH：

1. **Python 3.12.x**：用于本地模型的运行环境初始化与任务执行，建议统一使用 3.12.x 版本。
2. **Git**：用于部分模型的资源获取流程，特别是 GPT-SoVITS-CPUFast。
3. **Conda（可选）**：Miniconda 或 Anaconda。若系统 PATH 中可检测到 `conda` 命令，应用会优先为每个模型创建独立的 conda 环境（`<模型目录>/conda_env`），否则回退到标准 `venv`。不安装 Conda 也能正常使用全部功能。

> 国内用户建议先阅读第 8 节《国内网络环境配置建议》，为 pip / conda / git 配置镜像源与代理，可显著提升依赖安装与模型资源下载的成功率与速度。

## 2. 使用前须知

1. 当前以 Windows 10/11 本地模式运行。
2. 首次安装模型、首次推理或首次训练通常会较慢，这是正常现象。
3. 每项任务可以单独选择使用 CPU 或 CUDA（GPU）运行；若所选设备与当前模型运行环境不一致，提交前会出现确认弹窗，确认后应用会自动切换，但本次任务启动时间会明显增加。
4. 模型训练建议使用 GPU；CPU 可以运行，但速度会明显下降。
5. 当前默认可用的是 **Local 模式**（本地 SQLite + 本地 Python Runtime）；Remote 模式仍在开发中，尚未接入真实 HTTP 调用。

## 3. 当前模型支持

| 基础模型 | 版本 | 文本转语音 | 声音克隆 | 模型训练 | 音色设计 | 流式语音 | 设备支持 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Irodori-TTS-V3 | 500M | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| Dots.TTS | 2B | ✓ | ✓ | ✓ | — | — | CPU / CUDA |
| Qwen3-TTS | 1.7B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| Qwen3-TTS | 0.6B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| VoxCPM2 | 2B | ✓ | ✓ | ✓ | ✓ | — | CPU / CUDA |
| MOSS-TTS Local | 1.7B | ✓ | ✓ | ✓ | — | — | CPU / CUDA |
| MOSS-TTS Realtime | 1.7B | — | — | — | — | ✓ | CPU / CUDA |
| GPT-SoVITS-CPUFast | V1 | ✓ | ✓ | — | — | — | CPU |
| GPT-SoVITS-CPUFast | V2 / V2Pro / V2ProPlus | ✓ | ✓ | — | — | — | CPU |

## 4. 快速上手

1. 启动后先打开**设置**页，确认数据、日志、模型目录是否符合当前机器的目录规划。
2. 打开**模型管理**页，先为模型选择当前设备（单设备模型会自动回填），再安装要使用的模型。
3. **文本转语音**：选择模型、说话人、语言，输入文本后提交。
4. **声音克隆**：上传参考音频，填写目标台词后提交；参考音频支持 `wav`、`mp3`、`flac`、`ogg`。
5. **音色设计**：输入音色 prompt 与目标台词，生成目标风格语音。
6. **模型训练**：先导入样本（支持单样本和批量数据集两种方式），再填写说话人名称和参数后启动训练。
7. **流式语音**：进入流式语音页，创建会话后以聊天方式连续发送消息，实时接收音频 chunk 回放；说话人可使用参考音频克隆，或选择已训练说话人。

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

查看、搜索、编辑和删除本地说话人，支持按状态筛选，并可从本地模型目录导入说话人。

### 5.6 历史任务

统一查看所有任务（含流式语音会话）的状态、详情和结果，支持试听、导出音频，以及从历史记录回填参数重新发起任务。

### 5.7 流式语音

流式语音采用会话级长期进程：一条会话可连续发送多条消息，按 `contextId` 分发音频流。页面支持参考音频语音克隆与已训练说话人（从 Ready 说话人选择）两类输入。历史会话会恢复消息和配置，单条消息的试听与导出均通过历史任务及消息 ID 定位对应的已生成音频。

## 6. 已支持模型项目指引

1. Irodori-TTS-V3
   - 项目入口（GitHub）：https://github.com/Aratako/Irodori-TTS
   - 模型仓库（Hugging Face）：https://huggingface.co/Aratako
2. Dots.TTS
   - 项目入口（GitHub）：https://github.com/rednote-hilab/dots.tts
   - 模型仓库（Hugging Face）：https://huggingface.co/rednote-hilab
3. Qwen3-TTS
   - 项目入口（GitHub）：https://github.com/QwenLM/Qwen3-TTS
   - 模型仓库（Hugging Face）：https://huggingface.co/Qwen
4. VoxCPM2
   - 模型仓库（Hugging Face）：https://huggingface.co/openbmb/VoxCPM2
5. MOSS-TTS Local
   - 模型仓库（Hugging Face）：https://huggingface.co/OpenMOSS-Team
6. MOSS-TTS Realtime
   - 项目入口（GitHub）：https://github.com/OpenMOSS/MOSS-TTS
   - 模型仓库（Hugging Face）：https://huggingface.co/OpenMOSS-Team
7. GPT-SoVITS-CPUFast
   - 项目仓库（GitHub）：https://github.com/baicai-1145/GPT-SoVITS-CPUFast

## 7. 常见问题

**首次运行慢**：首次调用模型时会自动创建 Python 虚拟环境并安装依赖，属于正常现象，后续调用不会重复这个过程。

**任务长时间无进展**：检查页面通知栏是否有错误提示，或查看配置中 `log_dir` 下对应任务的日志文件。若为首次安装、首次推理或切换了设备类型，启动耗时增加是正常的。

**flash-attn 相关报错**：Windows 环境下通常缺少稳定官方支持，建议保持默认的 `sdpa`。

**切到 Remote 模式后报错**：当前版本 Remote 模式尚未接通真实 API 调用，建议继续使用 Local 模式。

## 8. 国内网络环境配置建议

Kirine Client 在首次安装模型、首次推理时会从 PyPI 下载 Python 依赖，从 GitHub 克隆部分模型源码，从 Hugging Face 下载模型权重。这些资源在国内网络下经常出现超时或失败，建议提前为 **pip**、**conda**、**git** 配置国内镜像源，必要时再为 GitHub / Hugging Face 配置代理。

> 以下配置在终端中执行一次即可全局生效（写入用户配置文件）。命令行中的 `#` 注释无需输入。

### 8.1 pip 镜像源

将 pip 默认源切换为清华 TUNA 镜像，并提升可信主机超时时间：

```bash
# 写入用户级 pip 配置（Windows / macOS / Linux 通用）
pip config set global.index-url https://pypi.tuna.tsinghua.edu.cn/simple
pip config set global.trusted-host pypi.tuna.tsinghua.edu.cn
pip config set global.timeout 120
```

备选镜像（任选其一，替换上面的 `index-url` 即可）：

- 阿里云：`https://mirrors.aliyun.com/pypi/simple/`
- 中科大：`https://pypi.mirrors.ustc.edu.cn/simple/`
- 腾讯云：`https://mirrors.cloud.tencent.com/pypi/simple/`

### 8.2 conda 镜像源

若使用 Conda 管理环境，建议配置清华镜像。执行后会在 `~/.condarc`（Windows 为 `C:\Users\<用户名>\.condarc`）中写入配置：

```bash
# 逐条执行
conda config --add channels https://mirrors.tuna.tsinghua.edu.cn/anaconda/pkgs/main
conda config --add channels https://mirrors.tuna.tsinghua.edu.cn/anaconda/pkgs/free
conda config --add channels https://mirrors.tuna.tsinghua.edu.cn/anaconda/cloud/conda-forge
conda config --set show_channel_urls yes
```

如需恢复官方源：`conda config --remove-key channels`（或直接删除 `.condarc`）。

### 8.3 git 镜像与代理

GitHub 克隆速度慢或失败时，有两种常用方案。

**方案 A：使用 GitHub 镜像加速（无需代理）**

将 `github.com` 替换为镜像域名即可，例如：

```bash
# 原始
git clone https://github.com/rednote-hilab/dots.tts
# 使用镜像（任选其一，可用性随时间变化）
git clone https://ghproxy.com/https://github.com/rednote-hilab/dots.tts
git clone https://mirror.ghproxy.com/https://github.com/rednote-hilab/dots.tts
```

**方案 B：为 git 配置 HTTP / HTTPS 代理（需自备代理端口）**

如果你本地有可用的代理客户端（如监听 `127.0.0.1:7890`），可为 git 单独设置代理：

```bash
# 设置代理（把 7890 换成你的实际端口）
git config --global http.proxy http://127.0.0.1:7890
git config --global https.proxy http://127.0.0.1:7890

# 取消代理
git config --global --unset http.proxy
git config --global --unset https.proxy
```

### 8.4 Hugging Face 下载加速

模型权重通过 `huggingface_hub` 下载，国内可通过 `hf-mirror.com` 镜像加速。在启动 Kirine Client **之前**设置环境变量，应用及其子进程会继承该变量：

**Windows（永久生效，写入用户环境变量）：**

```powershell
[Environment]::SetEnvironmentVariable("HF_ENDPOINT", "https://hf-mirror.com", "User")
```

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
├── ffmpeg-8.1.2.zip                      # ffmpeg Windows 构建包（shared build）
├── sox-14.4.2-win32.zip                 # SoX Windows 构建包
└── src-model-runtime.zip                # Python 模型运行时打包
```

获取方式：

- `ffmpeg-8.1.2.zip`：ffmpeg **shared build**（含 avcodec 等 DLL，供 torchcodec/torchaudio 加载）。从 https://www.gyan.dev/ffmpeg/builds/ 下载 full shared build，重打包为顶层目录 `ffmpeg-8.1.2/`（或保留原名，安装钩子会规整）。版本号须与文件名一致。
- `sox-14.4.2-win32.zip`：从 https://sourceforge.net/projects/sox/files/sox/ 下载对应版本的 Windows 包。
- `src-model-runtime.zip`：是仓库 `src-model/` 目录的打包产物。在开发阶段应用会自动识别 workspace 根目录下的 `src-model/`（即仓库 clone 下来的目录），因此本地开发时该 zip 文件缺失不影响运行时功能，但 Tauri 在启动/构建前会检查文件存在性。可以先创建一个空 zip 文件占位，或联系项目维护者获取正式的运行时包。

### D3. 克隆与初始化

```bash
git clone --recurse-submodules <repo_url>
cd kirine-client
npm install
```

如果你已经完成过普通 `git clone`，需要额外执行一次子模块初始化：

```bash
git submodule update --init --recursive
```

当某个模型适配器子项目有更新时，在主仓库中执行：

```bash
git submodule update --remote --recursive
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
- **Python 脚本日志**：每次任务执行时，脚本输出会重定向到 `log_dir/task/` 下按任务类型前缀命名的日志文件（如 `tts-<id>.log`、`voice-clone-<id>.log`、`voice-design-<id>.log`、`training-<id>.log`、`streaming-<id>.log`）。
- **模型运行时查找**：Rust 启动时会依次尝试 `<workspace>/src-model`、`<app_dir>/src-model`、`<app_dir>/lib/src-model`，本地开发通常命中第一个路径（仓库目录下的 `src-model/`）。

### D9. 模型适配器开发

如果需要为 `src-model/` 新增或维护模型适配器（包括两个配置文件 `model-config.json` / `params-config.json` 的结构、前端支持的表单组件类型、Python 适配器的实现流程与调用链路），请阅读 **[模型适配器开发指南](src-model/ADAPTER_DEVELOPMENT.md)**。
