use std::{
    ffi::OsString,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use kirine_client_lib::utils::process::{
    append_process_output, join_path_entries, read_task_log_tail,
};

#[test]
fn append_process_output_writes_labeled_stdout_and_stderr() {
    let unique_name = format!(
        "process-log-{}-{}-{}.log",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        "test"
    );
    let log_path = std::env::temp_dir().join(unique_name);

    append_process_output(&log_path, b"hello from stdout\n", b"hello from stderr\n").unwrap();

    let content = fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("[stdout]"));
    assert!(content.contains("hello from stdout"));
    assert!(content.contains("[stderr]"));
    assert!(content.contains("hello from stderr"));

    let _ = fs::remove_file(log_path);
}

#[test]
fn join_path_entries_concatenates_without_quoting() {
    // Regression: std::env::join_paths wraps components containing ';' in double
    // quotes on Windows. The inherited PATH always contains ';', so passing it as
    // one component produced `D:\tools;"C:\Windows\system32;..."`, and children
    // splitting on ';' resolved invalid quoted entries (broke cmd.exe and
    // nvidia-smi resolution in spawned PowerShell scripts).
    let bundled = OsString::from("D:\\app\\lib\\ffmpeg-8.1.2\\bin");
    let machine_path = OsString::from(
        "C:\\Windows\\system32;C:\\Windows;C:\\Program Files\\NVIDIA Corporation\\NVSMI",
    );
    let joined = join_path_entries(&[bundled, machine_path]);

    assert_eq!(
        joined.to_string_lossy(),
        "D:\\app\\lib\\ffmpeg-8.1.2\\bin;C:\\Windows\\system32;C:\\Windows;C:\\Program Files\\NVIDIA Corporation\\NVSMI"
    );
    assert!(!joined.to_string_lossy().contains('"'));
}

fn unique_log_path(label: &str) -> std::path::PathBuf {
    let unique_name = format!(
        "task-log-tail-{}-{}-{}.log",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        label
    );
    std::env::temp_dir().join(unique_name)
}

#[test]
fn read_task_log_tail_returns_whole_small_file() {
    let log_path = unique_log_path("small");

    let content = (0..100)
        .map(|i| format!("line-{i:04}\n"))
        .collect::<String>();
    fs::write(&log_path, &content).unwrap();

    let tail = read_task_log_tail(&log_path);

    // 小文件无需截断：tail 覆盖到最后一行且各行为完整行
    assert!(tail.contains("line-0000"));
    assert!(tail.ends_with("line-0099"));
    assert!(tail.lines().all(|line| line.starts_with("line-")));

    let _ = fs::remove_file(log_path);
}

#[test]
fn read_task_log_tail_reads_only_tail_window_aligned_to_line() {
    let log_path = unique_log_path("large");

    // 10_000 行 × ~12B ≈ 120KB > 64KB tail 窗口，必然触发从中间起读
    let content = (0..10_000)
        .map(|i| format!("line-{i:05}\n"))
        .collect::<String>();
    fs::write(&log_path, &content).unwrap();

    let tail = read_task_log_tail(&log_path);

    // tail 只包含文件末尾片段（≤64KB 窗口），且对齐到完整行
    assert!(tail.chars().count() <= 64 * 1024);
    assert!(!tail.contains("line-00000"));
    assert!(tail.ends_with("line-09999"));
    // trim_error_snippet 按字符取末尾片段，切割点可能落在行中间：仅首行可能不完整
    let lines = tail.lines().collect::<Vec<_>>();
    assert!(lines.len() > 1);
    assert!(lines[1..].iter().all(|line| line.starts_with("line-")));

    let _ = fs::remove_file(log_path);
}

#[test]
fn read_task_log_tail_missing_file_is_empty() {
    let log_path = unique_log_path("missing");
    assert_eq!(read_task_log_tail(&log_path), String::new());
}
