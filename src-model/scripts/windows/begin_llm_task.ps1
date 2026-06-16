$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath
$modelRoot = $null
$venvDir = $null
$venvPython = $null
$scriptPath = $null
$paramsFile = $null
$logPath = $null
$taskLogFile = $null

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--base-model', '--params-file', '--log-path', '--task-log-file') -ActionName 'begin-llm-task'
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
if ([string]::IsNullOrWhiteSpace($paramsFile)) {
    Write-Error 'Missing --params-file argument.'
    exit 64
}
$logPath = $parsed['--log-path']
$taskLogFile = $parsed['--task-log-file']
Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.'



$modelRoot = Join-Path $srcModelRoot $baseModel
$venvDir = Join-Path $modelRoot 'venv'
$venvPython = Join-Path $venvDir 'Scripts\python.exe'

$condaExe = Get-CondaExecutable
$condaEnvPath = $null
if ($null -ne $condaExe) {
    $condaEnvPath = Get-CondaEnvironmentPath -EnvironmentName $baseModel
    if ($null -ne $condaEnvPath) {
        $venvDir = $condaEnvPath
        $venvPython = Join-Path $venvDir 'python.exe'
    }
}

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

Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Starting LLM task with base model '$baseModel'."
$pythonCommand = Resolve-PythonCommand -BaseModel $baseModel -VenvPython $venvPython
try {
    Invoke-LoggedCommand -Description "Running Python command" -Command $pythonCommand[0] -Arguments (
        $pythonCommand[1..($pythonCommand.Length - 1)],
        '-X', 'utf8',
        '-X', 'faulthandler',
        '-u',
        $scriptPath,
        '--params-file', $paramsFile,
        '--log-path', $logPath,
        '--task-log-file', $taskLogFile
    )
}
catch {
    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Execute Error: $_"
    $exitCode = 1
}
else {
    Append-TaskLog -TaskLogFile $taskLogFile -Value "[begin-llm-task] Completed LLM task successfully."
}


