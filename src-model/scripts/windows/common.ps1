Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:Utf8NoBomEncoding = New-Object System.Text.UTF8Encoding($false)
[Console]::InputEncoding = $script:Utf8NoBomEncoding
[Console]::OutputEncoding = $script:Utf8NoBomEncoding
$OutputEncoding = $script:Utf8NoBomEncoding
try {
    $chcpPath = Join-Path $env:SystemRoot 'System32\chcp.com'
    if (Test-Path -LiteralPath $chcpPath) {
        & $chcpPath 65001 > $null 2> $null
    }
}
catch {
}
$PSDefaultParameterValues['Out-File:Encoding'] = 'utf8'
$PSDefaultParameterValues['Add-Content:Encoding'] = 'utf8'
$PSDefaultParameterValues['Set-Content:Encoding'] = 'utf8'

function Get-SrcModelRoot {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ScriptPath
    )

    $scriptDir = Split-Path -Parent $ScriptPath
    return [System.IO.Path]::GetFullPath((Join-Path $scriptDir '..\..'))
}

function Parse-CliArguments {
    param(
        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [Parameter(Mandatory = $true)]
        [string[]]$OptionsWithValues,

        [string[]]$SwitchOptions = @(),

        [Parameter(Mandatory = $true)]
        [string]$ActionName
    )

    $parsed = @{}
    foreach ($option in $OptionsWithValues) {
        $parsed[$option] = $null
    }
    foreach ($option in $SwitchOptions) {
        $parsed[$option] = $false
    }

    for ($index = 0; $index -lt $Arguments.Length; $index++) {
        $argument = $Arguments[$index]
        if ($OptionsWithValues -contains $argument) {
            if ($index + 1 -ge $Arguments.Length) {
                throw "Missing value for $argument."
            }

            $parsed[$argument] = $Arguments[$index + 1]
            $index++
            continue
        }

        if ($SwitchOptions -contains $argument) {
            $parsed[$argument] = $true
            continue
        }

        throw "Unknown $ActionName argument: $argument"
    }

    return $parsed
}

function Ensure-TaskLogFile {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [string]$TaskLogFile,

        [Parameter(Mandatory = $true)]
        [string]$MissingMessage,

        [switch]$Initialize
    )

    if ([string]::IsNullOrWhiteSpace($TaskLogFile)) {
        Write-Error $MissingMessage
        exit 64
    }

    $taskLogDir = Split-Path -Parent $TaskLogFile
    if (-not [string]::IsNullOrWhiteSpace($taskLogDir) -and -not (Test-Path -LiteralPath $taskLogDir)) {
        New-Item -ItemType Directory -Path $taskLogDir -Force | Out-Null
    }

    if (-not [string]::IsNullOrWhiteSpace($taskLogDir) -and -not (Test-Path -LiteralPath $taskLogDir)) {
        Write-Error "Failed to create task log directory: $taskLogDir"
        exit 65
    }

    if ($Initialize) {
        [System.IO.File]::WriteAllText($TaskLogFile, '', $script:Utf8NoBomEncoding)
    }
}

function Write-TaskLog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TaskLogFile,

        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [string]$Value
    )

    [System.IO.File]::WriteAllText($TaskLogFile, $Value, $script:Utf8NoBomEncoding)
}

function Append-TaskLog {
    param(
        [Parameter(Mandatory = $true)]
        [string]$TaskLogFile,

        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [string]$Value
    )

    [System.IO.File]::AppendAllText($TaskLogFile, $Value + [Environment]::NewLine, $script:Utf8NoBomEncoding)
}

function Get-BootstrapPythonCommand {
    $python = Get-Command python -ErrorAction SilentlyContinue
    if ($null -ne $python) {
        return @($python.Source)
    }

    $py = Get-Command py -ErrorAction SilentlyContinue
    if ($null -ne $py) {
        & $py.Source -3 --version *> $null
        if ($LASTEXITCODE -eq 0) {
            return @($py.Source, '-3')
        }
    }

    return $null
}

function Get-VisualStudioDeveloperCommandPrompt {
    $programFilesX86 = ${env:ProgramFiles(x86)}
    if ([string]::IsNullOrWhiteSpace($programFilesX86)) {
        $programFilesX86 = $env:ProgramFiles
    }

    $vswherePath = Join-Path $programFilesX86 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path -LiteralPath $vswherePath) {
        $installationPaths = & $vswherePath -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($LASTEXITCODE -eq 0 -and $null -ne $installationPaths) {
            foreach ($installationPath in @($installationPaths)) {
                if ([string]::IsNullOrWhiteSpace($installationPath)) {
                    continue
                }

                $candidate = Join-Path $installationPath 'Common7\Tools\VsDevCmd.bat'
                if (Test-Path -LiteralPath $candidate) {
                    return $candidate
                }
            }
        }
    }

    $searchRoots = @(
        (Join-Path $programFilesX86 'Microsoft Visual Studio'),
        (Join-Path $env:ProgramFiles 'Microsoft Visual Studio'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Microsoft Visual Studio')
    )

    foreach ($root in $searchRoots) {
        if (-not (Test-Path -LiteralPath $root)) {
            continue
        }

        $match = Get-ChildItem -LiteralPath $root -Filter 'VsDevCmd.bat' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty FullName
        if (-not [string]::IsNullOrWhiteSpace($match) -and (Test-Path -LiteralPath $match)) {
            return $match
        }
    }

    return $null
}

function Get-CondaExecutable {
    # Only probe the conda CLI on PATH. We deliberately do NOT search common
    # install locations under $env:USERPROFILE / $env:ProgramData: doing so
    # previously relied on a comma-separated Join-Path list that PowerShell
    # parsed incorrectly (the comma bound adjacent Join-Path calls into a single
    # array argument, crashing with "Cannot convert System.Object[] to
    # ChildPath"). When conda is not on PATH we return $null and let callers
    # fall back to a plain venv.
    $condaCommand = @(Get-Command conda -CommandType Application -ErrorAction SilentlyContinue)
    if ($condaCommand.Count -gt 0) {
        $selected = ($condaCommand | Where-Object { $_.Source -like '*.exe' } | Select-Object -First 1)
        if ($null -eq $selected) {
            $selected = $condaCommand[0]
        }

        return $selected.Source
    }

    return $null
}

function Get-CondaEnvPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModelRoot
    )

    return Join-Path $ModelRoot 'conda_env'
}

function Resolve-ModelPythonEnvironment {
    # Resolve which Python environment a model directory should use.
    #
    # Selection order — we always prefer an environment that already exists on
    # disk, so a model that was set up with a plain venv keeps using that venv
    # even after the user later installs conda. Migrating an existing working
    # environment just because conda became available on PATH previously caused
    # init-task-runtime to create a fresh (often incomplete) conda_env and break.
    #
    #   1. conda_env/python.exe present  -> existing conda env
    #   2. venv/Scripts/python.exe present -> existing venv
    #   3. neither present               -> conda if the conda CLI is on PATH,
    #                                       otherwise venv (Exists = $false;
    #                                       caller is responsible for creation)
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModelRoot
    )

    $venvDir = Join-Path $ModelRoot 'venv'
    $venvPython = Join-Path $venvDir 'Scripts\python.exe'
    $condaEnvPath = Get-CondaEnvPath -ModelRoot $ModelRoot
    $condaEnvPython = Join-Path $condaEnvPath 'python.exe'

    if (Test-Path -LiteralPath $condaEnvPython) {
        return @{
            Backend = 'conda'
            EnvDir  = $condaEnvPath
            Python  = $condaEnvPython
            Exists  = $true
        }
    }

    if (Test-Path -LiteralPath $venvPython) {
        return @{
            Backend = 'venv'
            EnvDir  = $venvDir
            Python  = $venvPython
            Exists  = $true
        }
    }

    # Neither environment exists yet — pick the one to create. Prefer conda
    # when its CLI is available; otherwise fall back to a plain venv.
    $condaExe = Get-CondaExecutable
    if ($null -ne $condaExe) {
        return @{
            Backend = 'conda'
            EnvDir  = $condaEnvPath
            Python  = $condaEnvPython
            Exists  = $false
        }
    }

    return @{
        Backend = 'venv'
        EnvDir  = $venvDir
        Python  = $venvPython
        Exists  = $false
    }
}

function Resolve-PythonCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$ModelRoot,

        [Parameter(Mandatory = $true)]
        [string]$VenvPython
    )

    $condaEnvPath = Get-CondaEnvPath -ModelRoot $ModelRoot
    $condaPython = Join-Path $condaEnvPath 'python.exe'
    if (Test-Path -LiteralPath $condaPython) {
        $condaExe = Get-CondaExecutable
        if ($null -ne $condaExe) {
            return @($condaExe, 'run', '--prefix', $condaEnvPath, 'python', '--')
        }
        # conda env exists but the conda CLI is not on PATH. Invoke the env's
        # python.exe directly rather than emitting a bare 'conda' token that the
        # caller cannot resolve.
        return @($condaPython)
    }

    return @($VenvPython)
}

function Invoke-ExternalCommand {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Command,

        [Parameter(Mandatory = $true)]
        [string[]]$Arguments,

        [Parameter(Mandatory = $true)]
        [string]$TaskLogFile
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $previousPythonIoEncoding = $env:PYTHONIOENCODING
    $previousPythonUtf8 = $env:PYTHONUTF8
    $exitCode = 0

    try {
        $ErrorActionPreference = 'Continue'
        $env:PYTHONIOENCODING = 'utf-8'
        $env:PYTHONUTF8 = '1'
        & $Command @Arguments 2>&1 | Out-File -LiteralPath $TaskLogFile -Append -Encoding utf8
        $exitCode = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
        $env:PYTHONIOENCODING = $previousPythonIoEncoding
        $env:PYTHONUTF8 = $previousPythonUtf8
    }

    exit $exitCode
}