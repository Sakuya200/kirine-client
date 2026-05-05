$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

try {
    $parsed = Parse-CliArguments -Arguments $args -OptionsWithValues @('--input-path', '--output-path', '--format', '--input-format', '--sample-rate', '--task-log-file') -ActionName 'transcode-audio'
}
catch {
    Write-Error $_.Exception.Message
    exit 64
}

$inputPath = $parsed['--input-path']
$outputPath = $parsed['--output-path']
$format = $parsed['--format']
$inputFormat = $parsed['--input-format']
$sampleRate = $parsed['--sample-rate']
$taskLogFile = $parsed['--task-log-file']

Ensure-TaskLogFile -TaskLogFile $taskLogFile -MissingMessage 'Missing --task-log-file argument.'

if ([string]::IsNullOrWhiteSpace($inputPath)) {
    Write-Error 'Missing --input-path argument.'
    exit 64
}
if ([string]::IsNullOrWhiteSpace($outputPath)) {
    Write-Error 'Missing --output-path argument.'
    exit 64
}
if ([string]::IsNullOrWhiteSpace($format)) {
    Write-Error 'Missing --format argument.'
    exit 64
}

$resolvedInputPath = [System.IO.Path]::GetFullPath($inputPath)
$resolvedOutputPath = [System.IO.Path]::GetFullPath($outputPath)
if (-not (Test-Path -LiteralPath $resolvedInputPath)) {
    Write-Error "Transcode input path does not exist: $resolvedInputPath"
    exit 66
}

$outputDir = Split-Path -Parent $resolvedOutputPath
if (-not [string]::IsNullOrWhiteSpace($outputDir) -and -not (Test-Path -LiteralPath $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}

$normalizedFormat = $format.Trim().ToLowerInvariant()
if ($normalizedFormat -eq 'wave') {
    $normalizedFormat = 'wav'
}

$outputOptions = @('-vn', '-sn', '-dn')
switch ($normalizedFormat) {
    'mp3' {
        $outputOptions += @('-codec:a', 'libmp3lame')
    }
    'flac' {
        $outputOptions += @('-codec:a', 'flac')
    }
    'wav' {
        $outputOptions += @('-acodec', 'pcm_s16le', '-ac', '1')
    }
    default {
        Write-Error "Unsupported transcode format: $format"
        exit 64
    }
}

if (-not [string]::IsNullOrWhiteSpace($sampleRate)) {
    $sampleRateValue = 0
    if (-not [int]::TryParse($sampleRate, [ref]$sampleRateValue) -or $sampleRateValue -le 0) {
        Write-Error "Sample rate must be positive: $sampleRate"
        exit 64
    }

    $outputOptions += @('-ar', $sampleRateValue.ToString())
}

$ffmpegArgs = @('-y', '-nostdin')
if (-not [string]::IsNullOrWhiteSpace($inputFormat)) {
    $ffmpegArgs += @('-f', $inputFormat.Trim().ToLowerInvariant())
}
$ffmpegArgs += @('-i', $resolvedInputPath)
$ffmpegArgs += $outputOptions
$ffmpegArgs += @($resolvedOutputPath)

Append-TaskLog -TaskLogFile $taskLogFile -Value "[transcode-audio] ffmpeg $($ffmpegArgs -join ' ')"
Invoke-ExternalCommand -Command 'ffmpeg' -Arguments $ffmpegArgs -TaskLogFile $taskLogFile