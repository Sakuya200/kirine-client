# Kirine Client 免安装绿色包打包脚本
#
# 用法：
#   powershell -ExecutionPolicy Bypass -File src-tauri/scripts/make-portable.ps1 [-SkipBuild] [-OutputDir <dir>]
#
# 默认先执行 `npm run tauri build`（保证 exe / 前端 dist / resources 版本一致），
# 然后把构建产物组装为免安装 zip：
#   kirine-client.exe + config.toml + lib\sox-14-4-2 + lib\ffmpeg-8.1.2 + lib\src-model + README-portable.txt
#
# 布局常量对齐表（必须与 NSIS 钩子 windows/prepare-dependencies*.nsh 和运行时约定一致，改动需同步）：
#   <root>\kirine-client.exe                          NSIS: $INSTDIR 根          运行时: exe_dir / app_dir
#   <root>\config.toml                                NSIS: POSTINSTALL 移到根   运行时: env_config.rs 按 cwd 相对解析
#   <root>\lib\sox-14-4-2\sox.exe                     NSIS: sox.nsh 目标名（连字符；资源 zip 顶层是 sox-14.4.2，需重命名）
#   <root>\lib\ffmpeg-8.1.2\bin\ffmpeg.exe            NSIS: ffmpeg.nsh 重命名规则（zip 顶层 ffmpeg-8.1.2-full_build-shared）
#   <root>\lib\src-model\scripts\windows\*.ps1        NSIS: src-model.nsh 合并解压   运行时: script_paths.rs / config 模块
#
# 未来签名预留：加 -SignThumbprint <证书指纹> 可在复制 exe 后调用 signtool 签名（三方 ffmpeg/sox 不签）。

#Requires -Version 5.1
param(
    [switch]$SkipBuild,
    [string]$OutputDir,
    [string]$SignThumbprint
)

$ErrorActionPreference = 'Stop'

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot '..\..')
$tauriDir = Join-Path $repoRoot 'src-tauri'
$conf = Get-Content (Join-Path $tauriDir 'tauri.conf.json') -Raw | ConvertFrom-Json
$version = $conf.version
$productName = $conf.productName
$exeName = "$productName.exe"
$resources = Join-Path $tauriDir 'resources'

if (-not $OutputDir) { $OutputDir = Join-Path $tauriDir 'target\portable' }

# 1) 构建
if (-not $SkipBuild) {
    Push-Location $repoRoot
    try {
        npm run tauri build
        if ($LASTEXITCODE -ne 0) { throw "tauri build failed (exit $LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}
$exe = Join-Path $tauriDir "target\release\$exeName"
if (-not (Test-Path -LiteralPath $exe)) { throw "build output not found: $exe" }

# 2) 组装 staging（先清旧）
$staging = Join-Path $OutputDir $version
if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $staging 'lib') -Force | Out-Null
Copy-Item -LiteralPath $exe -Destination (Join-Path $staging $exeName)
Copy-Item -LiteralPath (Join-Path $resources 'config.toml') -Destination $staging

# 3) sox：解压后规整 sox-14.4.2 -> sox-14-4-2（连字符，运行时约定；勿复刻 NSIS hook 的点号陷阱）
$soxZip = Join-Path $resources 'sox-14.4.2-win32.zip'
Expand-Archive -LiteralPath $soxZip -DestinationPath (Join-Path $staging 'lib') -Force
Rename-Item -LiteralPath (Join-Path $staging 'lib\sox-14.4.2') -NewName 'sox-14-4-2'
if (-not (Test-Path -LiteralPath (Join-Path $staging 'lib\sox-14-4-2\sox.exe'))) {
    throw "unexpected sox zip layout: lib\sox-14-4-2\sox.exe missing"
}

# 4) ffmpeg：解压后规整顶层目录，与 ffmpeg.nsh 行为一致
$ffmpegZip = Join-Path $resources 'ffmpeg-8.1.2.zip'
Expand-Archive -LiteralPath $ffmpegZip -DestinationPath (Join-Path $staging 'lib') -Force
Rename-Item -LiteralPath (Join-Path $staging 'lib\ffmpeg-8.1.2-full_build-shared') -NewName 'ffmpeg-8.1.2'
if (-not (Test-Path -LiteralPath (Join-Path $staging 'lib\ffmpeg-8.1.2\bin\ffmpeg.exe'))) {
    throw "unexpected ffmpeg zip layout: lib\ffmpeg-8.1.2\bin\ffmpeg.exe missing"
}

# 5) src-model：zip 无顶层目录，直接解压到 lib\src-model（等价 NSIS Merge 语义）
$srcModelZip = Join-Path $resources 'src-model-runtime.zip'
Expand-Archive -LiteralPath $srcModelZip -DestinationPath (Join-Path $staging 'lib\src-model') -Force
if (-not (Test-Path -LiteralPath (Join-Path $staging 'lib\src-model\scripts\windows\init_task_runtime.ps1'))) {
    throw "unexpected src-model zip layout: lib\src-model\scripts\windows\init_task_runtime.ps1 missing"
}

# 6) exe 签名（可选，预留）
if ($SignThumbprint) {
    $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if (-not $signtool) { throw 'signtool.exe not found in PATH' }
    & $signtool.Source sign /sha1 $SignThumbprint /fd sha256 /tr http://timestamp.digicert.com /td sha256 "$staging\$exeName"
    if ($LASTEXITCODE -ne 0) { throw 'signtool sign failed' }
}

# 7) README-portable.txt
$readme = @"
Kirine Client $version 免安装绿色版
=====================================

使用：解压到任意目录后运行 kirine-client.exe。

系统要求
--------
- Windows 10/11 x64
- 需系统已安装 Microsoft Edge WebView2 Runtime（Win11 自带；Win10 若缺失，
  请从 https://developer.microsoft.com/microsoft-edge/webview2/ 安装 Evergreen Runtime，
  或先安装一次 NSIS 安装包版本——安装器会自动补装）。
- 建议解压到不含中文或空格的常规路径，以规避部分第三方工具（conda 等）的边角问题。

与安装版的差异
--------------
- 本包不写注册表、不注册系统 PATH；终端里手动输入 ffmpeg/sox 不可用，
  但应用内所有功能不受影响（应用会自动使用自带 lib 目录下的工具）。
- 首次运行如被 SmartScreen 拦截（未签名程序），点击"更多信息 -> 仍要运行"，
  或右键 exe -> 属性 -> 勾选"解除锁定"。

升级 / 重置 / 卸载
------------------
- 升级：新版本 zip 直接覆盖解压即可。lib\src-model 下的 venv/conda_env、已下载的
  模型与 data 目录不在 zip 内，覆盖解压会原样保留。大版本升级建议先删除 lib 目录
  再解压（代价是首次使用任务时自动重建 Python 环境并重装 torch）。
- 彻底重置：删除 lib 目录后重新解压。
- 卸载：直接删除本目录即可，无注册表残留。
"@
$readme | Out-File -LiteralPath (Join-Path $staging 'README-portable.txt') -Encoding utf8

# 8) 压缩：逐文件写条目并统一正斜杠分隔符（PS5.1 的 CreateFromDirectory 在 .NET Framework
#    下生成反斜杠条目名，不符合 zip 规范，部分解压工具会将其解成单层文件名）；
#    也不用 Compress-Archive（PS5.1 有 2GB 限制）。
Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zipPath = Join-Path $OutputDir "$productName-$version-portable-x64.zip"
if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
$zip = [System.IO.Compression.ZipFile]::Open($zipPath, [System.IO.Compression.ZipArchiveMode]::Create)
try {
    Get-ChildItem -LiteralPath $staging -Recurse -File | ForEach-Object {
        $entryName = $_.FullName.Substring($staging.Length).TrimStart('\', '/').Replace('\', '/')
        [System.IO.Compression.ZipFileExtensions]::CreateEntryFromFile(
            $zip, $_.FullName, $entryName, [System.IO.Compression.CompressionLevel]::Optimal) | Out-Null
    }
} finally {
    $zip.Dispose()
}
Remove-Item -LiteralPath $staging -Recurse -Force

Write-Host "Portable package: $zipPath"
