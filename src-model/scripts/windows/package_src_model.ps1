param(
    [string]$OutputFile
)

$ErrorActionPreference = 'Stop'

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$srcModelRoot = [System.IO.Path]::GetFullPath((Join-Path $scriptDir '..\..'))

if ([string]::IsNullOrWhiteSpace($OutputFile)) {
    $OutputFile = [System.IO.Path]::GetFullPath((Join-Path $srcModelRoot '..\src-tauri\resources\src-model-runtime.zip'))
}

$outputDir = Split-Path -Parent $OutputFile
if (-not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

Add-Type -AssemblyName System.IO.Compression
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-RelativeArchivePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RootPath,

        [Parameter(Mandatory = $true)]
        [string]$FullPath
    )

    $normalizedRoot = [System.IO.Path]::GetFullPath($RootPath).TrimEnd([char[]]@([char]92, [char]47))
    $normalizedFullPath = [System.IO.Path]::GetFullPath($FullPath)

    if (-not $normalizedFullPath.StartsWith($normalizedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Path is outside of the src-model root: $FullPath"
    }

    return $normalizedFullPath.Substring($normalizedRoot.Length).TrimStart([char[]]@([char]92, [char]47))
}

$excludeDirectoryNames = @('base-models', 'tests', '__pycache__', '.pytest_cache', '.mypy_cache', '.ruff_cache', 'venv', '.venv')
$excludeExtensions = @('.pyc', '.pyo')
$sourceDirectories = @('scripts', 'configs')
$modelDirectories = @('qwen3_tts', 'vox_cpm2', 'moss_tts_local')

function Test-ShouldExcludeArchivePath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$RelativePath,

        [AllowEmptyString()]
        [string]$Extension
    )

    foreach ($directorySegment in ($RelativePath -split '[\\/]')) {
        if ($excludeDirectoryNames -contains $directorySegment) {
            return $true
        }
    }

    if ($excludeExtensions -contains $Extension.ToLowerInvariant()) {
        return $true
    }

    return $false
}

function Add-ArchiveFile {
    param(
        [Parameter(Mandatory = $true)]
        [System.IO.Compression.ZipArchive]$Archive,

        [Parameter(Mandatory = $true)]
        [string]$FullPath
    )

    $relativePath = Get-RelativeArchivePath -RootPath $srcModelRoot -FullPath $FullPath
    $entryPath = $relativePath -replace '\\', '/'
    $entry = $Archive.CreateEntry($entryPath, [System.IO.Compression.CompressionLevel]::Optimal)
    $entryStream = $entry.Open()
    try {
        $fileStream = [System.IO.File]::OpenRead($FullPath)
        try {
            $fileStream.CopyTo($entryStream)
        }
        finally {
            $fileStream.Dispose()
        }
    }
    finally {
        $entryStream.Dispose()
    }
}

function Add-DirectoryFilesToArchive {
    param(
        [Parameter(Mandatory = $true)]
        [System.IO.Compression.ZipArchive]$Archive,

        [Parameter(Mandatory = $true)]
        [string]$DirectoryPath
    )

    Get-ChildItem -LiteralPath $DirectoryPath -Recurse -File | ForEach-Object {
        $fullPath = $_.FullName
        $relativePath = Get-RelativeArchivePath -RootPath $srcModelRoot -FullPath $fullPath

        if (Test-ShouldExcludeArchivePath -RelativePath $relativePath -Extension $_.Extension) {
            return
        }

        Add-ArchiveFile -Archive $Archive -FullPath $fullPath
    }
}

if (Test-Path -LiteralPath $OutputFile) {
    Remove-Item -LiteralPath $OutputFile -Force
}

$zipFile = [System.IO.File]::Open($OutputFile, [System.IO.FileMode]::CreateNew)
try {
    $archive = New-Object System.IO.Compression.ZipArchive($zipFile, [System.IO.Compression.ZipArchiveMode]::Create, $false)
    try {
        foreach ($directoryName in $sourceDirectories) {
            $directoryPath = Join-Path $srcModelRoot $directoryName
            if (-not (Test-Path -LiteralPath $directoryPath)) {
                throw "Source directory not found: $directoryPath"
            }

            Add-DirectoryFilesToArchive -Archive $archive -DirectoryPath $directoryPath
        }

        foreach ($directoryName in $modelDirectories) {
            $directoryPath = Join-Path $srcModelRoot $directoryName
            if (-not (Test-Path -LiteralPath $directoryPath)) {
                throw "Source directory not found: $directoryPath"
            }

            Add-DirectoryFilesToArchive -Archive $archive -DirectoryPath $directoryPath
        }
    }
    finally {
        $archive.Dispose()
    }
}
finally {
    $zipFile.Dispose()
}

Write-Host "Created model runtime archive: $OutputFile"