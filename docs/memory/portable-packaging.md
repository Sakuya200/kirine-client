# 绿色免安装包打包

> 状态截至 2026-09-10 · 分支 `v0.12.2`

## 背景

NSIS 安装包因"安装期 `powershell -ExecutionPolicy Bypass -WindowStyle Hidden Expand-Archive` 解压 zip（含 ~105MB ffmpeg shared build）+ src-model 经 `%TEMP%` 中转搬运 + 写 `HKCU\Environment\Path` 并广播 `WM_SETTINGCHANGE` + 未签名"的形态组合，常被杀软启发式误报为木马。经审计确认全仓库无 COM/regsvr32 注册、无内存加载 DLL、无运行时下载执行——属形态问题而非恶意行为。

决策：保留现有 NSIS 安装程序逻辑不动、暂不签名，**新增免安装 zip 绿色包**作为替代分发方式（用户解压即用，彻底避开"安装后释放文件"启发式）。

## 当前状态

### 打包脚本

- `src-tauri/scripts/make-portable.ps1`：默认先 `npm run tauri build`，再组装 staging（exe + config.toml 在根；sox/ffmpeg/src-model 解压规整到 `lib\`），生成 `README-portable.txt`，用 `[System.IO.Compression.ZipFile]::CreateFromDirectory` 压缩输出到 `src-tauri/target/portable/kirine-client-<版本>-portable-x64.zip`（不用 `Compress-Archive`，PS5.1 有 2GB 限制）。
- 参数：`-SkipBuild`（复用现有产物）、`-OutputDir`、`-SignThumbprint`（**签名预留参数位**，复制 exe 后调 `signtool sign`；将来启用签名时：NSIS 版在 `tauri.conf.json` 的 `bundle.windows` 配 `certificateThumbprint`/`digestAlgorithm`/`timestampUrl`；三方 ffmpeg/sox 不签）。
- **脚本必须保存为带 BOM 的 UTF-8**（含中文，Windows PowerShell 5.1 对无 BOM 文件按 ANSI/GBK 解析会乱码报语法错）。

### 布局常量对齐表（与 NSIS 钩子/运行时约定一致，改动需三处同步）

| 包内路径 | 来源/规整 | 运行时约定 |
|---|---|---|
| `<root>\kirine-client.exe` | tauri build 产物 | exe_dir / app_dir |
| `<root>\config.toml` | `resources/config.toml` 直放根 | `env_config.rs` 按 cwd 相对解析 |
| `<root>\lib\sox-14-4-2\sox.exe` | zip 顶层 `sox-14.4.2`（点号）→ 重命名连字符（NSIS `sox.nsh` 已补同名重命名） | 当前无脚本端调用，仅预留 |
| `<root>\lib\ffmpeg-8.1.2\bin\` | zip 顶层 `ffmpeg-8.1.2-full_build-shared` → 重命名 | `transcode_audio.ps1` 按名解析 + `moss_tts_realtime/common.py` 的 `os.add_dll_directory` |
| `<root>\lib\src-model\` | zip 无顶层目录，直接解压（等价 NSIS Merge） | `script_paths.rs` 候选 `app_dir/lib/src-model` |

### PATH 前缀注入（绿色包不写注册表的关键）

`src-tauri/src/utils/process.rs` 的 `bundled_tool_path_prefix()`（`OnceLock` 缓存）：取 exe 同级 `lib\ffmpeg-8.1.2\bin` 与 `lib\sox-14-4-2` 中实际存在的目录，`std::env::join_paths` 前置到现有 PATH 后经 `prepare_command_with_stdio`（所有子进程 spawn 的唯一汇聚点）注入为子进程 env `PATH`。传递链已验证：Rust → powershell.exe（脚本内 `& ffmpeg` 按子进程 PATH 解析）→ python.exe → Python subprocess 孙进程逐级继承。目录不存在时返回 None 不注入（开发模式 target/debug 无 lib，天然 no-op）。

### WebView2 缺失检测

`lib.rs` 的 `run()` 最前面用 `tauri::webview_version()` 检测，缺失时经 `show_error_dialog`（`windows-sys` `MessageBoxW`，需 feature `Win32_UI_WindowsAndMessaging`）弹原生提示后退出——此前缺失时 `builder.build` 失败仅 `eprintln` 静默退出（绿色包无安装器 `downloadBootstrapper` 兜底，用户视角闪退）。

## 与安装版的行为差异

- 两端均**不写** `HKCU\Environment\Path`（安装器的写入/注销流程已于本次整体移除）：终端手动 `ffmpeg/sox` 不可用，应用内功能无影响；老版本用户的 HKCU Path 残留条目不做自动清理，可手动删除。
- 升级：覆盖解压即可，`lib/src-model` 下 venv/conda_env、模型、data 不在 zip 内天然保留；大版本升级建议删 `lib` 重解（首次任务自动重建环境+重装 torch）。
- 卸载：删目录即净；安装版卸载同不再涉及 PATH 清理。
- 首启 SmartScreen 可能拦未签名 exe（README-portable.txt 已写解除方法）。

## 相关文件

- `src-tauri/scripts/make-portable.ps1`（打包脚本）
- `src-tauri/src/utils/process.rs`（PATH 前缀注入）
- `src-tauri/src/lib.rs`（WebView2 检测 + `show_error_dialog`）
- `src-tauri/Cargo.toml`（windows-sys 增加 `Win32_UI_WindowsAndMessaging` feature）
- `src-tauri/windows/prepare-dependencies*.nsh`（NSIS 布局参照基准，未改动）
- README.md D5.1（构建入口文档）
