$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath
$venvPython = Join-Path $srcModelRoot 'venv\Scripts\python.exe'

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--mode', '--task-log-file') -ActionName 'toggle-lora-dependencies'
}
catch {
    Write-Error $_.Exception.Message
    exit 64
}

$mode = $parsed['--mode']
$taskLogFile = $parsed['--task-log-file']
Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.' -Initialize

if ($mode -notin @('enable', 'disable')) {
    Write-Error "Unsupported --mode value: $mode"
    exit 64
}

if (-not (Test-Path -LiteralPath $venvPython)) {
    Write-Error "LoRA dependency toggle requires an initialized Python environment at $venvPython"
    exit 65
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

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[toggle-lora-dependencies] ${Description}: ${Command} $($Arguments -join ' ')"

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
        throw "[toggle-lora-dependencies] $Description failed with exit code $exitCode."
    }
}

Append-TaskLog -TaskLogFile $taskLogFile -Value "[toggle-lora-dependencies] mode=$mode"

if ($mode -eq 'enable') {
    Invoke-LoggedCommand -Description 'install peft' -Command $venvPython -Arguments @('-m', 'pip', 'install', '--upgrade', 'peft')
    Append-TaskLog -TaskLogFile $taskLogFile -Value '[toggle-lora-dependencies] LoRA dependencies are enabled'
    exit 0
}

Invoke-LoggedCommand -Description 'uninstall peft' -Command $venvPython -Arguments @('-m', 'pip', 'uninstall', '-y', 'peft')
Append-TaskLog -TaskLogFile $taskLogFile -Value '[toggle-lora-dependencies] LoRA dependencies are disabled'