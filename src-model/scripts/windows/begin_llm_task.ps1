$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath
$venvDir = $null
$venvPython = $null
$scriptPath = $null
$paramsFile = $null
$logPath = $null
$taskLogFile = $null

$scriptArgs = $args
$forwardedScriptArgs = @()
$separatorIndex = [Array]::IndexOf($scriptArgs, '--')
if ($separatorIndex -ge 0) {
    if ($separatorIndex + 1 -lt $scriptArgs.Length) {
        $forwardedScriptArgs = $scriptArgs[($separatorIndex + 1)..($scriptArgs.Length - 1)]
    }
    if ($separatorIndex -gt 0) {
        $scriptArgs = $scriptArgs[0..($separatorIndex - 1)]
    }
    else {
        $scriptArgs = @()
    }
}
try {
    $parsed = Parse-CliArguments -Arguments $scriptArgs -OptionsWithValues @('--base-model', '--params-file', '--script-path', '--log-path', '--task-log-file') -ActionName 'begin-llm-task'
}
catch {
    Write-Error $_.Exception.Message
    exit 64
}

$baseModel = $parsed['--base-model']
if ([string]::IsNullOrWhiteSpace($baseModel)) {
    Write-Error 'Missing --base-model argument.'
    exit 64
}
$scriptPath = $parsed['--script-path']
if ([string]::IsNullOrWhiteSpace($scriptPath)) {
    Write-Error 'Missing --script-path argument.'
    exit 64
}
$paramsFile = $parsed['--params-file']
$logPath = $parsed['--log-path']
$taskLogFile = $parsed['--task-log-file']
Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.'

$modelRoot = Join-Path $srcModelRoot $baseModel
# Use whichever Python environment already exists in the model directory
# (conda_env or venv). This keeps an older venv-based install working even
# after conda is later installed, instead of routing execution through a
# conda_env that was never set up for this model.
$pyEnv = Resolve-ModelPythonEnvironment -ModelRoot $modelRoot
$venvDir = $pyEnv.EnvDir
$venvPython = $pyEnv.Python

function Invoke-LoggedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Description,

        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] ${Description}: ${Command} $($Arguments -join ' ')"

    $previousErrorActionPreference = $ErrorActionPreference
    $previousPythonIoEncoding = $env:PYTHONIOENCODING
    $previousPythonUtf8 = $env:PYTHONUTF8
    $exitCode = 0

    try {
        $ErrorActionPreference = 'Continue'
        $env:PYTHONIOENCODING = 'utf-8'
        $env:PYTHONUTF8 = '1'
        & $Command @Arguments 2>&1 | Out-File -LiteralPath $taskLogFile -Append -Encoding utf8
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        $env:PYTHONIOENCODING = $previousPythonIoEncoding
        $env:PYTHONUTF8 = $previousPythonUtf8
    }

    if ($exitCode -ne 0) {
        throw "[begin-llm-task] $Description failed with exit code $exitCode."
    }
}

function Initialize-VisualStudioEnvironment {
    # VsDevCmd.bat's batch parsing is sensitive to the inherited environment:
    # on some machines a PATH entry (e.g. legacy NVIDIA PhysX with parentheses
    # and spaces) makes the cmd parser die with "\X was unexpected at this
    # time." inside Microsoft's own scripts, which no quoting on our side can
    # avoid. Use the standard "clean-environment capture" technique (as
    # CMake/conda-build do): run VsDevCmd once against a minimal, guaranteed
    # batch-parse-safe system PATH, capture its `set` output, then merge the
    # VS environment into the real session. Application PATH entries therefore
    # never reach the batch parser, while python still gets the full VS env.
    #
    # Returns $true when the VS environment has been merged into this session
    # (caller then invokes python directly); $false on setup failure (caller
    # falls back to invoking python without the VS environment).
    param(
        [Parameter(Mandatory = $true)]
        [string]$VsDevCmdPath,

        [Parameter(Mandatory = $true)]
        [string]$CmdExe
    )

    $savedPath = $env:PATH
    $setupPath = @(
        [Environment]::SystemDirectory,
        (Join-Path $env:SystemRoot 'System32\Wbem'),
        (Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0')
    ) -join ';'

    $setupOutput = $null
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $env:PATH = $setupPath
        $ErrorActionPreference = 'Continue'
        # /s: let cmd strip only the outermost quotes and keep the inner
        # quotes around the VsDevCmd path. stderr is merged into the success
        # stream purely for log visibility; the parser below filters by
        # [string] type, so error records are skipped safely.
        $setupOutput = & $CmdExe /d /s /c ('call "{0}" -arch=x64 -no_logo && set' -f $VsDevCmdPath) 2>&1
    }
    catch {
        return $false
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        $env:PATH = $savedPath
    }

    if ($LASTEXITCODE -ne 0) {
        return $false
    }

    $setupVars = @{}
    foreach ($line in @($setupOutput)) {
        if ($line -isnot [string]) {
            continue
        }
        $separatorIndex = $line.IndexOf('=')
        if ($separatorIndex -gt 0) {
            $setupVars[$line.Substring(0, $separatorIndex)] = $line.Substring($separatorIndex + 1)
        }
    }

    if (-not $setupVars.ContainsKey('PATH')) {
        return $false
    }

    foreach ($key in @($setupVars.Keys)) {
        if ($key -ieq 'PATH') {
            continue
        }
        Set-Item -Path ("env:{0}" -f $key) -Value $setupVars[$key]
    }

    # VS dirs (including the clean system base) first, the original PATH (with
    # the app-injected ffmpeg/sox dirs and machine entries) afterwards, so both
    # stay available at runtime.
    $env:PATH = $setupVars['PATH'] + ';' + $savedPath
    return $true
}

function Invoke-LoggedPythonCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PythonExecutable,

        [Parameter(Mandatory = $true)]
        [string[]]$PythonArguments
    )

    # Resolve cmd.exe via an absolute path: the app's subprocess PATH is not
    # guaranteed to contain System32 (it depends on the environment the app
    # itself was launched with), so a bare 'cmd.exe' token can fail command
    # resolution even on a healthy Windows install. Mirrors the SystemRoot-based
    # chcp.com lookup in common.ps1.
    $cmdExe = Join-Path ([Environment]::SystemDirectory) 'cmd.exe'
    $vsDevCmdPath = Get-VisualStudioDeveloperCommandPrompt
    if (-not [string]::IsNullOrWhiteSpace($vsDevCmdPath) -and (Test-Path -LiteralPath $cmdExe)) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Initializing VS dev environment: $vsDevCmdPath"
        if (Initialize-VisualStudioEnvironment -VsDevCmdPath $vsDevCmdPath -CmdExe $cmdExe) {
            Invoke-LoggedCommand -Description 'Running Python command' -Command $PythonExecutable -Arguments $PythonArguments
            return
        }

        # VS environment setup is a best-effort enhancement and must never
        # break the task: fall back to invoking python directly (the behavior
        # before VS integration was introduced).
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] VS dev environment setup failed, falling back to direct python invocation."
    }

    Invoke-LoggedCommand -Description 'Running Python command' -Command $PythonExecutable -Arguments $PythonArguments
}

Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Starting LLM task with base model '$baseModel'."

if (-not (Test-Path -LiteralPath $scriptPath)) {
    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Execute Error: Script not found: $scriptPath"
    exit 1
}

if (-not (Test-Path -LiteralPath $venvPython)) {
    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Execute Error: Python executable not found: $venvPython"
    exit 1
}

Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] executing script: $scriptPath"

try {
    # NOTE: --log-path / --task-log-file are intentionally NOT forwarded to
    # the target Python script.  They are consumed by this wrapper for its
    # own task-log output (Append-TaskLog / Out-File above).  The params-file
    # entry scripts (voice_clone/tts/training/voice_design across all models)
    # only accept --params-file and would fail with argparse
    # "unrecognized arguments" if these were passed through.  Scripts that genuinely
    # need them (e.g. download.py) receive them via the forwarded args after `--`.
    $pythonArgs = @(
        '-X', 'utf8',
        '-X', 'faulthandler',
        '-u',
        $scriptPath
    )
    if (-not [string]::IsNullOrWhiteSpace($paramsFile)) {
        $pythonArgs += @('--params-file', $paramsFile)
    }
    $pythonArgs += $forwardedScriptArgs
    Invoke-LoggedPythonCommand -PythonExecutable $venvPython -PythonArguments $pythonArgs
}
catch {
    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Execute Error: $_"
    exit 1
}
Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Completed LLM task successfully."
exit 0