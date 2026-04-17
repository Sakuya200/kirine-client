$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath
$modelRoot = $null
$venvDir = $null
$venvPython = $null

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--base-model', '--model-id-list', '--model-name-list', '--target-root-dir', '--log-path', '--task-log-file') -ActionName 'download-models'
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

$modelRoot = Join-Path $srcModelRoot $baseModel
$venvDir = Join-Path $modelRoot 'venv'
$venvPython = Join-Path $venvDir 'Scripts\python.exe'

$modelIdListJson = $parsed['--model-id-list']
$modelNameListJson = $parsed['--model-name-list']
$targetRootDir = $parsed['--target-root-dir']

$taskLogFile = $parsed['--task-log-file']
Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.'

foreach ($requiredArg in @{
        '--model-id-list'   = $modelIdListJson;
        '--model-name-list' = $modelNameListJson;
        '--target-root-dir' = $targetRootDir
    }.GetEnumerator()) {
    if ([string]::IsNullOrWhiteSpace($requiredArg.Value)) {
        Write-Error "Missing $($requiredArg.Key) argument."
        exit 64
    }
}

try {
    $parsedModelIds = $modelIdListJson | ConvertFrom-Json
    $parsedModelNames = $modelNameListJson | ConvertFrom-Json

    if ($null -eq $parsedModelIds -or -not ($parsedModelIds -is [System.Array])) {
        throw 'download-models requires --model-id-list to be a JSON array.'
    }

    if ($null -eq $parsedModelNames -or -not ($parsedModelNames -is [System.Array])) {
        throw 'download-models requires --model-name-list to be a JSON array.'
    }

    $modelIdList = [string[]]$parsedModelIds
    $modelNameList = [string[]]$parsedModelNames
}
catch {
    Write-Error 'download-models requires valid JSON arrays for --model-id-list and --model-name-list.'
    exit 64
}

if ($modelIdList.Count -eq 0) {
    Write-Error 'download-models requires at least one model id.'
    exit 64
}

if ($modelIdList.Count -ne $modelNameList.Count) {
    Write-Error 'download-models requires --model-id-list and --model-name-list to have the same length.'
    exit 64
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

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[download-models] ${Description}: ${Command} $($Arguments -join ' ')"

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
        throw "[download-models] $Description failed with exit code $exitCode."
    }
}

function Resolve-ModelScopeCommand {
    $modelscopeExe = Join-Path $venvDir 'Scripts\modelscope.exe'
    if (Test-Path -LiteralPath $modelscopeExe) {
        return [pscustomobject]@{ Command = $modelscopeExe; Prefix = @() }
    }

    $modelscopeCmd = Join-Path $venvDir 'Scripts\modelscope.cmd'
    if (Test-Path -LiteralPath $modelscopeCmd) {
        return [pscustomobject]@{ Command = $modelscopeCmd; Prefix = @() }
    }

    if (Test-Path -LiteralPath $venvPython) {
        return [pscustomobject]@{ Command = $venvPython; Prefix = @('-m', 'modelscope.cli.cli') }
    }

    throw "[download-models] ModelScope CLI is unavailable because the Python runtime is missing at $venvPython. Run init-task-runtime first."
}

function Download-Model {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModelId,

        [Parameter(Mandatory = $true)]
        [string]$TargetDir,

        [Parameter(Mandatory = $true)]
        [pscustomobject]$CommandSpec
    )

    if (-not (Test-Path -LiteralPath $TargetDir)) {
        New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[download-models] ensuring model is ready at $TargetDir"

    $arguments = @()
    $arguments += $CommandSpec.Prefix
    $arguments += @('download', '--model', $ModelId, '--local_dir', $TargetDir)
    Invoke-LoggedCommand -Description "download $ModelId" -Command $CommandSpec.Command -Arguments $arguments
}

try {
    if (-not (Test-Path -LiteralPath $venvPython)) {
        throw "[download-models] Python virtual environment is missing at $venvPython. Run init-task-runtime first."
    }

    if (-not (Test-Path -LiteralPath $targetRootDir)) {
        New-Item -ItemType Directory -Path $targetRootDir -Force | Out-Null
    }

    $commandSpec = Resolve-ModelScopeCommand
    for ($index = 0; $index -lt $modelIdList.Count; $index++) {
        $modelId = [string]$modelIdList[$index]
        $modelName = [string]$modelNameList[$index]

        if ([string]::IsNullOrWhiteSpace($modelId) -or [string]::IsNullOrWhiteSpace($modelName)) {
            throw '[download-models] Model id list and model name list cannot contain empty values.'
        }

        $targetDir = Join-Path $targetRootDir $modelName
        Download-Model -ModelId $modelId -TargetDir $targetDir -CommandSpec $commandSpec
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value '[download-models] required models are ready'
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}