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
        return [pscustomobject]@{ Backend = 'modelscope'; Command = $modelscopeExe; Prefix = @() }
    }

    $modelscopeCmd = Join-Path $venvDir 'Scripts\modelscope.cmd'
    if (Test-Path -LiteralPath $modelscopeCmd) {
        return [pscustomobject]@{ Backend = 'modelscope'; Command = $modelscopeCmd; Prefix = @() }
    }

    if (Test-Path -LiteralPath $venvPython) {
        return [pscustomobject]@{ Backend = 'modelscope'; Command = $venvPython; Prefix = @('-m', 'modelscope.cli.cli') }
    }

    return $null
}

function Resolve-HuggingFaceCommand {
    $hfExe = Join-Path $venvDir 'Scripts\hf.exe'
    if (Test-Path -LiteralPath $hfExe) {
        return [pscustomobject]@{ Backend = 'huggingface'; Command = $hfExe; Prefix = @() }
    }

    $hfCmd = Join-Path $venvDir 'Scripts\hf.cmd'
    if (Test-Path -LiteralPath $hfCmd) {
        return [pscustomobject]@{ Backend = 'huggingface'; Command = $hfCmd; Prefix = @() }
    }

    $legacyHfExe = Join-Path $venvDir 'Scripts\huggingface-cli.exe'
    if (Test-Path -LiteralPath $legacyHfExe) {
        return [pscustomobject]@{ Backend = 'huggingface-legacy'; Command = $legacyHfExe; Prefix = @() }
    }

    $legacyHfCmd = Join-Path $venvDir 'Scripts\huggingface-cli.cmd'
    if (Test-Path -LiteralPath $legacyHfCmd) {
        return [pscustomobject]@{ Backend = 'huggingface-legacy'; Command = $legacyHfCmd; Prefix = @() }
    }

    if (Test-Path -LiteralPath $venvPython) {
        return [pscustomobject]@{
            Backend = 'huggingface-python'
            Command = $venvPython
            Prefix  = @(
                '-c',
                'import sys; from huggingface_hub import snapshot_download; snapshot_download(repo_id=sys.argv[1], local_dir=sys.argv[2], local_dir_use_symlinks=False)'
            )
        }
    }

    return $null
}

function Get-DownloadArguments {
    param(
        [Parameter(Mandatory = $true)]
        [pscustomobject]$CommandSpec,

        [Parameter(Mandatory = $true)]
        [string]$ModelId,

        [Parameter(Mandatory = $true)]
        [string]$TargetDir
    )

    $arguments = @()
    $arguments += $CommandSpec.Prefix

    switch ($CommandSpec.Backend) {
        'modelscope' {
            $arguments += @('download', '--model', $ModelId, '--local_dir', $TargetDir)
        }
        'huggingface' {
            $arguments += @('download', $ModelId, '--local-dir', $TargetDir)
        }
        'huggingface-legacy' {
            $arguments += @('download', $ModelId, '--local-dir', $TargetDir)
        }
        'huggingface-python' {
            $arguments += @($ModelId, $TargetDir)
        }
        default {
            throw "[download-models] Unsupported download backend: $($CommandSpec.Backend)"
        }
    }

    return $arguments
}

function Download-Model {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModelId,

        [Parameter(Mandatory = $true)]
        [string]$TargetDir,

        [Parameter()]
        [AllowNull()]
        [pscustomobject]$PrimaryCommandSpec,

        [Parameter()]
        [AllowNull()]
        [pscustomobject]$FallbackCommandSpec
    )

    if (-not (Test-Path -LiteralPath $TargetDir)) {
        New-Item -ItemType Directory -Path $TargetDir -Force | Out-Null
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[download-models] ensuring model is ready at $TargetDir"

    if ($null -eq $PrimaryCommandSpec -and $null -eq $FallbackCommandSpec) {
        throw '[download-models] Neither ModelScope nor Hugging Face download backends are available. Run init-task-runtime first.'
    }

    if ($null -ne $PrimaryCommandSpec) {
        try {
            $primaryArguments = Get-DownloadArguments -CommandSpec $PrimaryCommandSpec -ModelId $ModelId -TargetDir $TargetDir
            Invoke-LoggedCommand -Description "download $ModelId via $($PrimaryCommandSpec.Backend)" -Command $PrimaryCommandSpec.Command -Arguments $primaryArguments
            return
        }
        catch {
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[download-models] primary backend $($PrimaryCommandSpec.Backend) failed for ${ModelId}: $($_.Exception.Message)"
            if ($null -eq $FallbackCommandSpec) {
                throw
            }
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[download-models] falling back to $($FallbackCommandSpec.Backend) for $ModelId"
        }
    }

    $fallbackArguments = Get-DownloadArguments -CommandSpec $FallbackCommandSpec -ModelId $ModelId -TargetDir $TargetDir
    Invoke-LoggedCommand -Description "download $ModelId via $($FallbackCommandSpec.Backend)" -Command $FallbackCommandSpec.Command -Arguments $fallbackArguments
}

try {
    if (-not (Test-Path -LiteralPath $venvPython)) {
        throw "[download-models] Python virtual environment is missing at $venvPython. Run init-task-runtime first."
    }

    if (-not (Test-Path -LiteralPath $targetRootDir)) {
        New-Item -ItemType Directory -Path $targetRootDir -Force | Out-Null
    }

    $primaryCommandSpec = Resolve-ModelScopeCommand
    $fallbackCommandSpec = Resolve-HuggingFaceCommand
    for ($index = 0; $index -lt $modelIdList.Count; $index++) {
        $modelId = [string]$modelIdList[$index]
        $modelName = [string]$modelNameList[$index]

        if ([string]::IsNullOrWhiteSpace($modelId) -or [string]::IsNullOrWhiteSpace($modelName)) {
            throw '[download-models] Model id list and model name list cannot contain empty values.'
        }

        $targetDir = Join-Path $targetRootDir $modelName
        Download-Model -ModelId $modelId -TargetDir $targetDir -PrimaryCommandSpec $primaryCommandSpec -FallbackCommandSpec $fallbackCommandSpec
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value '[download-models] required models are ready'
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}