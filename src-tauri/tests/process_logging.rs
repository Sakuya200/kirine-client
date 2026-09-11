use std::{
    ffi::OsString,
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use kirine_client_lib::utils::process::{append_process_output, join_path_entries};

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
    let machine_path = OsString::from("C:\\Windows\\system32;C:\\Windows;C:\\Program Files\\NVIDIA Corporation\\NVSMI");
    let joined = join_path_entries(&[bundled, machine_path]);

    assert_eq!(
        joined.to_string_lossy(),
        "D:\\app\\lib\\ffmpeg-8.1.2\\bin;C:\\Windows\\system32;C:\\Windows;C:\\Program Files\\NVIDIA Corporation\\NVSMI"
    );
    assert!(!joined.to_string_lossy().contains('"'));
}
