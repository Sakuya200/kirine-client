$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$srcModelRoot = Get-SrcModelRoot -ScriptPath $PSCommandPath
$modelRoot = $null
$requirementsFile = $null
$venvDir = $null
$venvPython = $null

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--base-model', '--requirements-file', '--log-path', '--task-log-file') -SwitchOptions @('--cpu-mode') -ActionName 'init-task-runtime'
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
$requirementsFile = Join-Path $modelRoot 'requirements.txt'
$venvDir = Join-Path $modelRoot 'venv'
$venvPython = Join-Path $venvDir 'Scripts\python.exe'

if (-not [string]::IsNullOrWhiteSpace($parsed['--requirements-file'])) {
    $requirementsFile = $parsed['--requirements-file']
}

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

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] ${Description}: ${Command} $($Arguments -join ' ')"

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
        throw "[init-task-runtime] $Description failed with exit code $exitCode."
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

    $previousErrorActionPreference = $ErrorActionPreference
    $previousPythonIoEncoding = $env:PYTHONIOENCODING
    $previousPythonUtf8 = $env:PYTHONUTF8

    try {
        $ErrorActionPreference = 'Continue'
        $env:PYTHONIOENCODING = 'utf-8'
        $env:PYTHONUTF8 = '1'
        $output = & $resolved.Source @Arguments 2>&1 | Out-String
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
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] detected CUDA $major.$minor via $($commandSpec.Command)"
            return @{ Major = $major; Minor = $minor }
        }

        if ($output -match 'release\s+([0-9]+)\.([0-9]+)') {
            $major = [int]$Matches[1]
            $minor = [int]$Matches[2]
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] detected CUDA $major.$minor via $($commandSpec.Command)"
            return @{ Major = $major; Minor = $minor }
        }
    }

    throw '[init-task-runtime] No usable NVIDIA GPU or CUDA toolkit was detected. init-task-runtime requires nvidia-smi or nvcc to report a supported CUDA version.'
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

    if ($candidates.Count -gt 0) {
        $candidateTagsText = (($candidates | ForEach-Object { "cu$($_.Tag)" }) -join ', ')
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] candidate PyTorch CUDA tags for detected CUDA $($Version.Major).$($Version.Minor): $candidateTagsText"
        return $candidates
    }

    $minimum = $supportedTags[-1]
    throw "[init-task-runtime] Detected CUDA version $($Version.Major).$($Version.Minor) is lower than the minimum supported stable PyTorch tag $($minimum.Major).$($minimum.Minor)."
}

function Get-TorchCudaVerifyArguments {
    return @(
        '-c',
        "import torch, torchaudio, torchvision; assert torch.cuda.is_available(), 'torch.cuda.is_available() returned False'; assert torch.version.cuda, 'torch.version.cuda is empty'; print(torch.__version__); print(torch.version.cuda)"
    )
}

function Get-TorchCpuVerifyArguments {
    return @(
        '-c',
        "import torch, torchaudio, torchvision; print(torch.__version__)"
    )
}

function Get-TorchRuntimeMetadataArguments {
    return @(
        '-c',
        "import torch, torchaudio, torchvision; print(f'{torch.__version__}|{torch.version.cuda or ''''}|{int(bool(torch.cuda.is_available()))}')"
    )
}

function Test-LoggedCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Description,

        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments
    )

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] ${Description}: ${Command} $($Arguments -join ' ')"

    $previousErrorActionPreference = $ErrorActionPreference
    $previousPythonIoEncoding = $env:PYTHONIOENCODING
    $previousPythonUtf8 = $env:PYTHONUTF8

    try {
        $ErrorActionPreference = 'Continue'
        $env:PYTHONIOENCODING = 'utf-8'
        $env:PYTHONUTF8 = '1'
        & $Command @Arguments 2>&1 | Out-File -LiteralPath $taskLogFile -Append -Encoding utf8
        return ($LASTEXITCODE -eq 0)
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        $env:PYTHONIOENCODING = $previousPythonIoEncoding
        $env:PYTHONUTF8 = $previousPythonUtf8
    }
}

function Get-TorchRuntimeMetadata {
    if (-not (Test-Path -LiteralPath $venvPython)) {
        return $null
    }

    $output = Get-ExternalCommandOutput -Command $venvPython -Arguments (Get-TorchRuntimeMetadataArguments)
    if ([string]::IsNullOrWhiteSpace($output)) {
        return $null
    }

    $parts = $output -split '\|', 3
    if ($parts.Length -ne 3) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] failed to parse torch runtime metadata output: $output"
        return $null
    }

    return @{
        torch_version  = $parts[0]
        torch_cuda     = $parts[1]
        cuda_available = ($parts[2] -eq '1')
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

function Test-TorchCpuInitialized {
    $metadata = Get-TorchRuntimeMetadata
    if ($null -eq $metadata) {
        return $false
    }

    if (-not [string]::IsNullOrWhiteSpace([string]$metadata.torch_cuda)) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] existing torch runtime uses CUDA $($metadata.torch_cuda), which does not match current CPU mode"
        return $false
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] detected compatible CPU torch runtime version $($metadata.torch_version); offline initialization check passed"
    return $true
}

function Test-TorchCudaInitialized {
    param(
        [Parameter(Mandatory = $true)]
        [System.Collections.IEnumerable]$CudaCandidates
    )

    $metadata = Get-TorchRuntimeMetadata
    if ($null -eq $metadata) {
        return $false
    }

    if (-not [bool]$metadata.cuda_available) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] installed torch runtime reports cuda_available=False; runtime does not match current GPU mode'
        return $false
    }

    $installedTag = Convert-TorchCudaVersionToTag -TorchCudaVersion ([string]$metadata.torch_cuda)
    if ([string]::IsNullOrWhiteSpace($installedTag)) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] installed torch runtime does not expose a usable CUDA version; runtime does not match current GPU mode'
        return $false
    }

    $candidateTags = @($CudaCandidates | ForEach-Object { [string]$_.Tag })
    if ($candidateTags -notcontains $installedTag) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] installed torch CUDA tag cu$installedTag is incompatible with the current GPU environment; expected one of: $($candidateTags -join ', ')"
        return $false
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] detected compatible CUDA torch runtime cu$installedTag (torch $($metadata.torch_version)); offline initialization check passed"
    return $true
}

function Ensure-BaseDependencies {
    Invoke-LoggedCommand -Description 'install project requirements' -Command $venvPython -Arguments @('-m', 'pip', 'install', '-r', $requirementsFile)
    Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] base Python dependencies are ready'
}

function Test-TorchCpuRuntime {
    return Test-LoggedCommand -Description 'verify existing torch runtime' -Command $venvPython -Arguments (Get-TorchCpuVerifyArguments)
}

function Test-TorchCudaRuntime {
    return Test-LoggedCommand -Description 'verify existing torch CUDA runtime' -Command $venvPython -Arguments (Get-TorchCudaVerifyArguments)
}

function Install-CompatibleTorchCuda {
    param(
        [Parameter(Mandatory = $true)]
        [System.Collections.IEnumerable]$CudaCandidates
    )

    $failedTags = @()

    foreach ($candidate in $CudaCandidates) {
        $candidateTag = $candidate.Tag

        try {
            Invoke-LoggedCommand -Description "install torch wheels for cu$candidateTag" -Command $venvPython -Arguments @('-m', 'pip', 'install', '--upgrade', '--force-reinstall', '--no-cache-dir', 'torch', 'torchvision', 'torchaudio', '--index-url', "https://download.pytorch.org/whl/cu$candidateTag")
            Invoke-LoggedCommand -Description "verify torch CUDA runtime using cu$candidateTag" -Command $venvPython -Arguments (Get-TorchCudaVerifyArguments)
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] verified working PyTorch CUDA runtime using cu$candidateTag"
            return $candidateTag
        }
        catch {
            $failedTags += "cu$candidateTag"
            Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] cu$candidateTag verification failed, trying next compatible tag: $($_.Exception.Message)"
        }
    }

    throw "[init-task-runtime] Unable to initialize a working PyTorch CUDA runtime for the detected CUDA environment. Tried $($failedTags -join ', '). Re-run with --cpu-mode if you want a CPU-only environment."
}

function Ensure-Venv {
    if (Test-Path -LiteralPath $venvPython) {
        return
    }

    $bootstrapCommand = @(Get-BootstrapPythonCommand)
    if ($bootstrapCommand.Count -eq 0) {
        throw '[init-task-runtime] Python environment is unavailable. Tried python and py -3 but neither succeeded.'
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value "[init-task-runtime] creating Python virtual environment at $venvDir"
    $bootstrapExecutable = $bootstrapCommand[0]
    $bootstrapArgs = @()
    if ($bootstrapCommand.Length -gt 1) {
        $bootstrapArgs += $bootstrapCommand[1..($bootstrapCommand.Length - 1)]
    }
    $bootstrapArgs += @('-m', 'venv', $venvDir)
    Invoke-LoggedCommand -Description 'create Python virtual environment' -Command $bootstrapExecutable -Arguments $bootstrapArgs

    if (-not (Test-Path -LiteralPath $venvPython)) {
        throw "[init-task-runtime] Python virtual environment was not created at $venvPython."
    }
}

try {
    if (-not (Test-Path -LiteralPath $requirementsFile)) {
        throw "[init-task-runtime] Requirements file not found: $requirementsFile"
    }

    Ensure-Venv

    if ($parsed['--cpu-mode']) {
        Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] CPU mode enabled; skipping CUDA detection'
        if (Test-TorchCpuInitialized) {
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] local task runtime already matches current CPU mode; skipping dependency installation'
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] local task runtime is ready'
            exit 0
        }

        Ensure-BaseDependencies
        if (Test-TorchCpuRuntime) {
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] existing torch runtime is already usable after base dependency sync; skipping torch reinstall'
        }
        else {
            Invoke-LoggedCommand -Description 'install torch CPU wheels' -Command $venvPython -Arguments @('-m', 'pip', 'install', '--upgrade', '--force-reinstall', '--no-cache-dir', 'torch', 'torchvision', 'torchaudio', '--index-url', 'https://download.pytorch.org/whl/cpu')
        }
    }
    else {
        $cudaVersion = Get-CudaVersion
        $cudaCandidates = Select-CudaTag -Version $cudaVersion
        if (Test-TorchCudaInitialized -CudaCandidates $cudaCandidates) {
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] local task runtime already matches current GPU mode; skipping dependency installation'
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] local task runtime is ready'
            exit 0
        }

        Ensure-BaseDependencies
        if (Test-TorchCudaRuntime) {
            Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] existing torch CUDA runtime is already usable after base dependency sync; skipping torch reinstall'
        }
        else {
            $null = Install-CompatibleTorchCuda -CudaCandidates $cudaCandidates
        }
    }

    if ($parsed['--cpu-mode']) {
        Invoke-LoggedCommand -Description 'verify torch CPU runtime after dependency install' -Command $venvPython -Arguments (Get-TorchCpuVerifyArguments)
    }
    else {
        Invoke-LoggedCommand -Description 'verify torch CUDA runtime after dependency install' -Command $venvPython -Arguments (Get-TorchCudaVerifyArguments)
    }

    Append-TaskLog -TaskLogFile $taskLogFile -Value '[init-task-runtime] local task runtime is ready'
    exit 0
}
catch {
    Write-Error $_.Exception.Message
    exit 1
}