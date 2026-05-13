$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--base-model', '--log-path', '--task-log-file') -SwitchOptions @('--cpu-mode') -ActionName 'ensure-torch-runtime'
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
$venvPython = Join-Path $modelRoot 'venv\Scripts\python.exe'
$taskLogFile = $parsed['--task-log-file']
Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.'

function Invoke-LoggedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Description,

        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[ensure-torch-runtime] ${Description}: ${Command} $($Arguments -join ' ')"

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
        throw "[ensure-torch-runtime] $Description failed with exit code $exitCode."
    }
}

function Get-ExternalCommandOutput {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [string[]]$Arguments = @()
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $previousPythonIoEncoding = $env:PYTHONIOENCODING
    $previousPythonUtf8 = $env:PYTHONUTF8

    try {
        $ErrorActionPreference = 'Continue'
        $env:PYTHONIOENCODING = 'utf-8'
        $env:PYTHONUTF8 = '1'
        $output = & $Command @Arguments 2>&1 | Out-String
        if ($LASTEXITCODE -ne 0) {
            return $null
        }

        return $output.Trim()
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        $env:PYTHONIOENCODING = $previousPythonIoEncoding
        $env:PYTHONUTF8 = $previousPythonUtf8
    }
}

function Get-CommandOutput {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [string[]]$Arguments = @()
    )

    $resolved = Get-Command $Command -ErrorAction SilentlyContinue
    if ($null -eq $resolved) {
        return $null
    }

    return Get-ExternalCommandOutput -Command $resolved.Source -Arguments $Arguments
}

function Get-TorchRuntimeMetadataArguments {
    return @(
        '-c',
        "import torch; print('TORCH_METADATA|{}|{}|{}'.format(torch.__version__, torch.version.cuda or '', int(bool(torch.cuda.is_available()))))"
    )
}

function Get-TorchRuntimeMetadata {
    if (-not (Test-Path -LiteralPath $venvPython)) {
        return $null
    }

    $output = Get-ExternalCommandOutput -Command $venvPython -Arguments (Get-TorchRuntimeMetadataArguments)
    if ([string]::IsNullOrWhiteSpace($output)) {
        return $null
    }

    $line = ($output -split "`r?`n" | Where-Object { $_ -like 'TORCH_METADATA|*' } | Select-Object -Last 1)
    if ([string]::IsNullOrWhiteSpace($line)) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[ensure-torch-runtime] failed to parse torch runtime metadata output: $output"
        return $null
    }

    $parts = $line -split '\|', 4
    if ($parts.Length -ne 4) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[ensure-torch-runtime] failed to parse torch runtime metadata line: $line"
        return $null
    }

    return @{
        torch_version  = $parts[1]
        torch_cuda     = $parts[2]
        cuda_available = ($parts[3] -eq '1')
    }
}

function Convert-TorchCudaVersionToTag {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TorchCudaVersion
    )

    if ($TorchCudaVersion -match '^(\d+)\.(\d+)$') {
        return "{0}{1}" -f [int]$Matches[1], [int]$Matches[2]
    }

    return $null
}

function Get-CudaVersion {
    foreach ($commandSpec in @(
            @{ Command = 'nvidia-smi'; Arguments = @() },
            @{ Command = 'nvcc'; Arguments = @('--version') }
        )) {
        $output = Get-CommandOutput -Command $commandSpec.Command -Arguments $commandSpec.Arguments
        if ([string]::IsNullOrWhiteSpace($output)) {
            continue
        }

        if ($output -match 'CUDA Version:\s*([0-9]+)\.([0-9]+)') {
            return @{ Major = [int]$Matches[1]; Minor = [int]$Matches[2] }
        }

        if ($output -match 'release\s+([0-9]+)\.([0-9]+)') {
            return @{ Major = [int]$Matches[1]; Minor = [int]$Matches[2] }
        }
    }

    throw '[ensure-torch-runtime] No usable NVIDIA GPU or CUDA toolkit was detected.'
}

function Test-VersionAtLeast {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$Version,

        [Parameter(Mandatory = $true)]
        [hashtable]$Supported
    )

    if ($Version.Major -gt $Supported.Major) {
        return $true
    }
    if ($Version.Major -lt $Supported.Major) {
        return $false
    }

    return $Version.Minor -ge $Supported.Minor
}

function Select-CudaTag {
    param(
        [Parameter(Mandatory = $true)]
        [hashtable]$Version
    )

    $supportedTags = @(
        @{ Major = 13; Minor = 0; Tag = '130' },
        @{ Major = 12; Minor = 8; Tag = '128' },
        @{ Major = 12; Minor = 6; Tag = '126' },
        @{ Major = 12; Minor = 4; Tag = '124' },
        @{ Major = 12; Minor = 1; Tag = '121' },
        @{ Major = 11; Minor = 8; Tag = '118' }
    )

    $candidates = @()
    foreach ($supported in $supportedTags) {
        if (Test-VersionAtLeast -Version $Version -Supported $supported) {
            $candidates += $supported
        }
    }

    if ($candidates.Count -eq 0) {
        throw "[ensure-torch-runtime] Detected CUDA version $($Version.Major).$($Version.Minor) is lower than minimum supported 11.8."
    }

    return $candidates
}

function Install-CpuTorchRuntime {
    $metadata = Get-TorchRuntimeMetadata
    if ($null -ne $metadata -and [string]::IsNullOrWhiteSpace([string]$metadata.torch_cuda)) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[ensure-torch-runtime] torch runtime already matches CPU mode (torch=$($metadata.torch_version))"
        return
    }

    Invoke-LoggedCommand -Description 'install torch CPU wheels' -Command $venvPython -Arguments @('-m', 'pip', 'install', '--force-reinstall', '--no-cache-dir', 'torch==2.10.0', 'torchvision', 'torchaudio', '--index-url', 'https://download.pytorch.org/whl/cpu')
    Invoke-LoggedCommand -Description 'verify torch CPU runtime' -Command $venvPython -Arguments @('-c', "import torch; assert not (torch.version.cuda or ''), 'CPU runtime expected no CUDA tag'; print(torch.__version__)")
}

function Install-CudaTorchRuntime {
    $cudaVersion = Get-CudaVersion
    $candidates = Select-CudaTag -Version $cudaVersion
    $candidateTags = @($candidates | ForEach-Object { [string]$_.Tag })

    $metadata = Get-TorchRuntimeMetadata
    $installedTag = $null
    if ($null -ne $metadata -and [bool]$metadata.cuda_available) {
        $installedTag = Convert-TorchCudaVersionToTag -TorchCudaVersion ([string]$metadata.torch_cuda)
    }

    if (-not [string]::IsNullOrWhiteSpace($installedTag) -and $candidateTags -contains $installedTag) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[ensure-torch-runtime] torch runtime already matches CUDA mode (cu$installedTag, torch=$($metadata.torch_version))"
        return
    }

    $failedTags = @()
    foreach ($candidate in $candidates) {
        $tag = [string]$candidate.Tag
        try {
            Invoke-LoggedCommand -Description "install torch CUDA wheels (cu$tag)" -Command $venvPython -Arguments @('-m', 'pip', 'install', '--force-reinstall', '--no-cache-dir', 'torch==2.10.0', 'torchvision', 'torchaudio', '--index-url', "https://download.pytorch.org/whl/cu$tag")
            Invoke-LoggedCommand -Description "verify torch CUDA runtime (cu$tag)" -Command $venvPython -Arguments @('-c', "import torch; assert torch.cuda.is_available(), 'torch.cuda.is_available() is False'; assert torch.version.cuda, 'torch.version.cuda is empty'; print(torch.__version__); print(torch.version.cuda)")
            return
        }
        catch {
            $failedTags += "cu$tag"
            $errorMessage = ($_.Exception | Select-Object -ExpandProperty Message)
            $failureMessage = "[ensure-torch-runtime] verification failed for cu{0}: {1}" -f $tag, $errorMessage
            Append-TaskLog -TaskLogFile $taskLogFile -Value $failureMessage
        }
    }

    throw "[ensure-torch-runtime] Unable to initialize working CUDA torch runtime. Tried: $($failedTags -join ', ')"
}

try {
    if (-not (Test-Path -LiteralPath $venvPython)) {
        throw "[ensure-torch-runtime] Python virtual environment not found: $venvPython. Please install model from model management first."
    }

    if ($parsed['--cpu-mode']) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value '[ensure-torch-runtime] CPU mode enabled'
        Install-CpuTorchRuntime
    }
    else {
        Append-TaskLog -TaskLogFile $taskLogFile -Value '[ensure-torch-runtime] CUDA mode enabled'
        Install-CudaTorchRuntime
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value '[ensure-torch-runtime] torch runtime is ready'
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}
