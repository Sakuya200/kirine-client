//! 训练音频规范化并发引擎（run_training_transcodes）：
//! - Semaphore 控并发、JoinSet 收集乱序结果，全部产出与完成顺序无关
//! - 失败样本收集（序号 + 输入 + 样本日志 tail）；失败率超阈值整体失败，
//!   低于阈值返回失败列表由调用方降级
//!
//! 传输层用假 spawn 闭包（写 sidecar 文件 + 样本日志）验证编排逻辑；
//! 真实脚本链（powershell → transcode_audio.ps1 → ffmpeg.exe）由真实训练任务回归覆盖。

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kirine_client_lib::test_support::{NormalizationFailure, anyhow, run_training_transcodes};

fn unique_test_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("kirine-{label}-{}-{nanos}", std::process::id()))
}

fn sample_dir_contents(log_dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(log_dir)
        .expect("log dir")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

/// 假转码：写样本日志 + sidecar 文件；输入名含 "bad" 时失败（仅写日志）。
fn fake_transcode(
    input: String,
    log_path: PathBuf,
) -> impl std::future::Future<Output = kirine_client_lib::Result<String>> + Send {
    async move {
        let tail = if input.contains("bad") {
            std::fs::write(&log_path, "[transcode-audio] simulated ffmpeg failure tail").expect("write log");
            return Err(anyhow::anyhow!("训练音频转码后未生成 WAV 文件: {input}"));
        }
        else {
            "[transcode-audio] fake ffmpeg progress output".to_string()
        };
        std::fs::write(&log_path, tail).expect("write log");
        let output = format!("{input}.normalized.wav");
        std::fs::write(&output, b"RIFF-fake-wav").expect("write sidecar");
        Ok(output)
    }
}

#[tokio::test]
async fn three_samples_concurrency_two_produce_all_outputs() {
    let dir = unique_test_dir("train-norm-ok");
    let log_dir = dir.join("logs");
    std::fs::create_dir_all(&log_dir).expect("log dir");
    let inputs: Vec<String> = (0..3)
        .map(|index| {
            let path = dir.join(format!("sample-{index}.wav"));
            std::fs::write(&path, b"fake audio").expect("write input");
            path.to_string_lossy().to_string()
        })
        .collect();

    let (normalized, failures) = run_training_transcodes(
        inputs.clone(),
        2,
        10,
        &log_dir,
        |input, log_path| fake_transcode(input, log_path),
    )
    .await
    .expect("all samples normalize");

    assert!(failures.is_empty());
    // 全部产出，与完成顺序无关：每个输入都有映射且 sidecar 文件存在
    assert_eq!(normalized.len(), 3);
    for input in &inputs {
        let output = format!("{input}.normalized.wav");
        assert_eq!(normalized.get(input).expect("mapping"), &output);
        assert!(Path::new(&output).exists(), "sidecar missing: {output}");
    }
    // 每个样本独立日志文件（并发进程不交错写同一日志）
    assert_eq!(sample_dir_contents(&log_dir).len(), 3);

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn failure_rate_above_threshold_fails_with_summary() {
    let dir = unique_test_dir("train-norm-fail");
    let log_dir = dir.join("logs");
    std::fs::create_dir_all(&log_dir).expect("log dir");
    let mut inputs: Vec<String> = (0..2)
        .map(|index| {
            let path = dir.join(format!("sample-{index}.wav"));
            std::fs::write(&path, b"fake audio").expect("write input");
            path.to_string_lossy().to_string()
        })
        .collect();
    let bad_path = dir.join("bad-sample.wav");
    std::fs::write(&bad_path, b"corrupted").expect("write input");
    inputs.push(bad_path.to_string_lossy().to_string());

    // 1/3 ≈ 34% > 10%：整体失败，错误信息含失败样本汇总与样本日志 tail
    let err = run_training_transcodes(inputs, 2, 10, &log_dir, |input, log_path| {
        fake_transcode(input, log_path)
    })
    .await
    .expect_err("above threshold should fail");
    let message = err.to_string();
    assert!(message.contains("超过阈值 10%"), "message: {message}");
    assert!(message.contains("bad-sample.wav"), "message: {message}");
    assert!(
        message.contains("simulated ffmpeg failure tail"),
        "stderr tail should be included: {message}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[tokio::test]
async fn failure_rate_below_threshold_returns_failures_for_degradation() {
    let dir = unique_test_dir("train-norm-tolerate");
    let log_dir = dir.join("logs");
    std::fs::create_dir_all(&log_dir).expect("log dir");
    let mut inputs: Vec<String> = (0..2)
        .map(|index| {
            let path = dir.join(format!("sample-{index}.wav"));
            std::fs::write(&path, b"fake audio").expect("write input");
            path.to_string_lossy().to_string()
        })
        .collect();
    let bad_path = dir.join("bad-sample.wav");
    std::fs::write(&bad_path, b"corrupted").expect("write input");
    inputs.push(bad_path.to_string_lossy().to_string());

    // 阈值 50%：1/3 ≈ 34% 低于阈值 → 不整体失败，返回失败列表供调用方降级
    let (normalized, failures) = run_training_transcodes(inputs, 2, 50, &log_dir, |input, log_path| {
        fake_transcode(input, log_path)
    })
    .await
    .expect("below threshold should not fail");

    assert_eq!(normalized.len(), 2);
    assert!(!normalized.values().any(|output| output.contains("bad-sample")));
    assert_eq!(failures.len(), 1);
    let NormalizationFailure { index, input, log_tail } = &failures[0];
    assert_eq!(*index, 2);
    assert!(input.contains("bad-sample.wav"));
    assert!(log_tail.contains("simulated ffmpeg failure tail"));

    let _ = std::fs::remove_dir_all(&dir);
}
