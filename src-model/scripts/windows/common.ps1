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

function Get-CondaExecutable {
    $condaCommand = @(Get-Command conda -CommandType Application -ErrorAction SilentlyContinue)
    if ($condaCommand.Count -gt 0) {
        $selected = ($condaCommand | Where-Object { $_.Source -like '*.exe' } | Select-Object -First 1)
        if ($null -eq $selected) {
            $selected = $condaCommand[0]
        }

        return $selected.Source
    }

    $candidatePaths = @(
        Join-Path $env:USERPROFILE 'miniforge3\Scripts\conda.exe',
        Join-Path $env:USERPROFILE 'miniforge3\condabin\conda.bat',
        Join-Path $env:USERPROFILE 'anaconda3\Scripts\conda.exe',
        Join-Path $env:USERPROFILE 'anaconda3\condabin\conda.bat',
        Join-Path $env:USERPROFILE 'miniconda3\Scripts\conda.exe',
        Join-Path $env:USERPROFILE 'miniconda3\condabin\conda.bat',
        Join-Path $env:ProgramData 'miniforge3\Scripts\conda.exe',
        Join-Path $env:ProgramData 'anaconda3\Scripts\conda.exe',
        Join-Path $env:ProgramData 'miniconda3\Scripts\conda.exe'
    )

    foreach ($candidatePath in $candidatePaths) {
        if (Test-Path -LiteralPath $candidatePath) {
            return $candidatePath
        }
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
        return @('conda', 'run', '--prefix', $condaEnvPath, 'python', '--')
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