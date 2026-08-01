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

function Convert-ToCmdArgument {
    param(
        [Parameter(Mandatory = $false)]
        [AllowNull()]
        [string]$Value
    )

    if ($null -eq $Value) {
        return '""'
    }

    if ($Value -match '[\s"&|<>^]') {
        return '"' + $Value.Replace('"', '\"') + '"'
    }

    return $Value
}

function Invoke-LoggedPythonCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PythonExecutable,

        [Parameter(Mandatory = $true)]
        [string[]]$PythonArguments
    )

    $vsDevCmdPath = Get-VisualStudioDeveloperCommandPrompt
    if (-not [string]::IsNullOrWhiteSpace($vsDevCmdPath)) {
        $commandLine = 'call "{0}" -arch=x64 -no_logo && set VSCMD_ARG_TGT_ARCH=x64 && {1}' -f $vsDevCmdPath, (Convert-ToCmdArgument -Value $PythonExecutable)
        foreach ($argument in $PythonArguments) {
            $commandLine += ' ' + (Convert-ToCmdArgument -Value $argument)
        }

        Invoke-LoggedCommand -Description 'Running Python command' -Command 'cmd.exe' -Arguments @('/d', '/c', $commandLine)
        return
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