use std::{
    fs,
    path::{Path, PathBuf},
};

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::service::models::TextToSpeechFormat;

pub fn content_type_for_format(format: TextToSpeechFormat) -> &'static str {
    match format {
        TextToSpeechFormat::Wav => "audio/wav",
        TextToSpeechFormat::Mp3 => "audio/mpeg",
        TextToSpeechFormat::Flac => "audio/flac",
    }
}

pub fn resolve_temp_wav_path(final_output_path: &str, format: TextToSpeechFormat) -> PathBuf {
    if format == TextToSpeechFormat::Wav {
        return PathBuf::from(final_output_path);
    }

    PathBuf::from(format!("{}.tmp.wav", final_output_path))
}

pub fn resolve_audio_input_format_hint(path: &Path) -> Option<&'static str> {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("wav") => Some("wav"),
        Some("mp3") => Some("mp3"),
        Some("flac") => Some("flac"),
        Some("ogg") => Some("ogg"),
        _ => None,
    }
}

pub fn build_ffmpeg_transcode_args(
    input_path: &Path,
    output_path: &Path,
    output_format: &str,
    sample_rate: Option<u32>,
) -> Vec<String> {
    let mut args = vec![
        "--input-path".to_string(),
        input_path.to_string_lossy().to_string(),
        "--output-path".to_string(),
        output_path.to_string_lossy().to_string(),
        "--format".to_string(),
        output_format.to_string(),
    ];

    if let Some(input_format) = resolve_audio_input_format_hint(input_path) {
        args.push("--input-format".to_string());
        args.push(input_format.to_string());
    }

    if let Some(sample_rate) = sample_rate {
        args.push("--sample-rate".to_string());
        args.push(sample_rate.to_string());
    }

    args
}

pub fn resolve_normalized_wav_sidecar_path(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.normalized.wav", path.to_string_lossy()))
}

pub fn save_audio_bytes_as(
    app: &AppHandle,
    file_name: &str,
    bytes: &[u8],
) -> std::result::Result<bool, String> {
    save_bytes_as(
        app,
        file_name,
        bytes,
        Some(audio_filter_name_from_file_name(file_name)),
    )
}

pub fn save_bytes_as(
    app: &AppHandle,
    file_name: &str,
    bytes: &[u8],
    filter_name: Option<&'static str>,
) -> std::result::Result<bool, String> {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "wav".to_string());

    let mut dialog = app.dialog().file().set_file_name(file_name);
    if let Some(filter_name) = filter_name {
        dialog = dialog.add_filter(filter_name, &[extension.as_str()]);
    }

    let selected_path = dialog.blocking_save_file();

    let Some(selected_path) = selected_path else {
        return Ok(false);
    };

    let output_path = resolve_save_path(selected_path)?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    fs::write(&output_path, bytes).map_err(|err| err.to_string())?;
    Ok(true)
}

fn resolve_save_path(file_path: FilePath) -> std::result::Result<PathBuf, String> {
    file_path
        .into_path()
        .map_err(|_| "当前平台返回了不可直接写入的文件路径。".to_string())
}

fn audio_filter_name_from_file_name(file_name: &str) -> &'static str {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .filter(|ext| !ext.is_empty())
        .unwrap_or_else(|| "wav".to_string());
    audio_filter_name(&extension)
}

fn audio_filter_name(extension: &str) -> &'static str {
    match extension {
        "wav" => "WAV 音频",
        "mp3" => "MP3 音频",
        "flac" => "FLAC 音频",
        "jsonl" => "JSONL 文件",
        "xlsx" => "Excel 文件",
        _ => "音频文件",
    }
}
