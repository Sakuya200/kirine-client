use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use kirine_client_lib::utils::process::append_process_output;

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
