//! 规则4：任务执行 hooks（4 个 `create_*_task`）不真正执行任务，仅验证调用模型层脚本
//! 时的参数契约。
//!
//! 本文件覆盖 **CLI 参数向量** —— `build_llm_task_script_args`（从 `pipeline/mod.rs`
//! 抽取的纯函数，经 `test_support` 重导出）。该向量是 Rust 侧传给 `begin_llm_task`
//! 包装器脚本的参数，决定脚本如何定位 python、params 文件与 task 日志。
//!
//! 「params 文件内容」（`PythonScriptInvocationSpec` 写入的 JSON，即模型层 python 脚本
//! 实际接收的参数）的覆盖见本文件下半部分的 `writes_*_params_file_with_expected_fields`
//! 用例。被测的 spec 类型原为 `pub(crate)`，现经 `test_support` 重导出后可供集成测试命名。

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

use kirine_client_lib::test_support::{
    build_llm_task_script_args, PythonScriptInvocationSpec, PythonScriptRuntimeOptions,
    PythonScriptTaskArgs, PythonScriptTaskKind, StreamingArgs, StreamingSpeakerArg, TTSArgs,
    TrainingArgs, VoiceCloneArgs, VoiceDesignArgs,
};

#[test]
fn build_llm_task_script_args_has_expected_order_and_values() {
    let script_path = Path::new("/src-model/qwen3_tts/tts.py");
    let params_path = Path::new("/data/params/42.json");
    let log_path = Path::new("/data/logs/task-42.log");
    let base_model = "qwen3_tts";

    let args = build_llm_task_script_args(script_path, params_path, log_path, base_model);

    // 10 个元素：5 个 flag + 5 个 value
    assert_eq!(args.len(), 10);

    let expected = [
        "--base-model",
        base_model,
        "--script-path",
        &script_path.to_string_lossy().to_string(),
        "--params-file",
        &params_path.to_string_lossy().to_string(),
        "--log-path",
        &log_path.to_string_lossy().to_string(),
        "--task-log-file",
        &log_path.to_string_lossy().to_string(),
    ];
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(&args[i], want, "arg[{i}] mismatch");
    }
}

#[test]
fn build_llm_task_script_args_log_path_equals_task_log_file() {
    // 包装器脚本：--log-path 与 --task-log-file 都指向同一个 task 日志文件
    // （脚本侧负责不把它们转发给 python，见 shell_scripts.rs 的脚本逻辑测试）。
    let args = build_llm_task_script_args(
        Path::new("/s.py"),
        Path::new("/p.json"),
        Path::new("/l.log"),
        "vox_cpm2",
    );

    let log_path = locate_value(&args, "--log-path");
    let task_log_file = locate_value(&args, "--task-log-file");
    assert_eq!(log_path, Some("/l.log"));
    assert_eq!(task_log_file, Some("/l.log"));
    assert_eq!(log_path, task_log_file);
}

#[test]
fn build_llm_task_script_args_forwards_params_file_and_base_model() {
    let args = build_llm_task_script_args(
        Path::new("/s.py"),
        Path::new("/p.json"),
        Path::new("/l.log"),
        "irodori_tts_v3",
    );

    assert_eq!(locate_value(&args, "--base-model"), Some("irodori_tts_v3"));
    assert_eq!(locate_value(&args, "--params-file"), Some("/p.json"));
    assert_eq!(locate_value(&args, "--script-path"), Some("/s.py"));
}

fn locate_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let idx = args.iter().position(|a| a == flag)?;
    args.get(idx + 1).map(String::as_str)
}

#[test]
fn streaming_script_args_match_begin_llm_task_contract() {
    // 流式会话同样经 `begin_llm_task` 包装器拉起 streaming.py，args 形状与其它流水线一致。
    let script = Path::new("/src-model/gpt_sovits_cpufast/streaming.py");
    let params = Path::new("/d/streaming_1/streaming.params.json");
    let task_log = Path::new("/log/task/streaming-1.log");

    let args = build_llm_task_script_args(script, params, task_log, "gpt_sovits_cpufast");

    // 与既有用例一致：用 to_string_lossy 比对，避免 Windows 路径分隔符差异。
    let expected = [
        "--base-model",
        "gpt_sovits_cpufast",
        "--script-path",
        &script.to_string_lossy().to_string(),
        "--params-file",
        &params.to_string_lossy().to_string(),
        "--log-path",
        &task_log.to_string_lossy().to_string(),
        "--task-log-file",
        &task_log.to_string_lossy().to_string(),
    ];
    assert_eq!(args.len(), expected.len());
    for (i, want) in expected.iter().enumerate() {
        assert_eq!(&args[i], want, "streaming arg[{i}] mismatch");
    }
}

// ---------- params 文件内容（PythonScriptInvocationSpec 写入的 JSON）----------

/// 规则4：`PythonScriptInvocationSpec::write_to_json_file` 写入的 params 文件即
/// 「调用模型层 python 脚本时的参数」。此处对 5 种任务类型分别构造 spec、写入临时
/// 文件、读回 JSON，断言 kind 标签与各 args 的关键字段被正确序列化。
fn unique_path(label: &str) -> std::path::PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "kirine-params-{}-{}-{}.json",
        std::process::id(),
        label,
        n
    ))
}

fn runtime() -> PythonScriptRuntimeOptions {
    PythonScriptRuntimeOptions {
        device: Some("cpu".to_string()),
        logging_dir: None,
        attn_implementation: Some("sdpa".to_string()),
    }
}

fn write_and_read(spec: &PythonScriptInvocationSpec, label: &str) -> Value {
    let path = unique_path(label);
    spec.write_to_json_file(&path)
        .expect("write spec to json file");
    let bytes = fs::read(&path).expect("read back params file");
    let _ = fs::remove_file(&path);
    serde_json::from_slice(&bytes).expect("parse params json")
}

#[test]
fn writes_tts_params_file_with_expected_fields() {
    let spec = PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: "qwen3_tts".to_string(),
        model_version: "1.7B".to_string(),
        kind: PythonScriptTaskKind::TextToSpeech,
        runtime: runtime(),
        args: PythonScriptTaskArgs::TextToSpeech(TTSArgs {
            model_root_path: "/models/root".to_string(),
            speaker_dir_name: None,
            model_params_json: serde_json::json!({}),
            text: "你好".to_string(),
            language: "chinese".to_string(),
            speaker: "Alice".to_string(),
            output_path: "/out/tts.wav".to_string(),
        }),
    };

    let json = write_and_read(&spec, "tts");
    assert_eq!(json["base_model"], "qwen3_tts");
    assert_eq!(json["model_version"], "1.7B");
    assert_eq!(json["kind"], "TextToSpeech");
    assert_eq!(json["args"]["TextToSpeech"]["text"], "你好");
    assert_eq!(json["args"]["TextToSpeech"]["language"], "chinese");
    assert_eq!(json["args"]["TextToSpeech"]["output_path"], "/out/tts.wav");
    assert_eq!(json["runtime"]["device"], "cpu");
}

#[test]
fn writes_voice_clone_params_file_with_expected_fields() {
    let spec = PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: "qwen3_tts".to_string(),
        model_version: "1.7B".to_string(),
        kind: PythonScriptTaskKind::VoiceClone,
        runtime: runtime(),
        args: PythonScriptTaskArgs::VoiceClone(VoiceCloneArgs {
            model_root_path: "/models/root".to_string(),
            speaker_dir_name: None,
            model_params_json: serde_json::json!({}),
            ref_audio_path: "/ref.wav".to_string(),
            ref_text: Some("参考".to_string()),
            language: "chinese".to_string(),
            output_path: "/out/vc.wav".to_string(),
            text: "生成".to_string(),
        }),
    };

    let json = write_and_read(&spec, "vc");
    assert_eq!(json["kind"], "VoiceClone");
    assert_eq!(json["args"]["VoiceClone"]["ref_audio_path"], "/ref.wav");
    assert_eq!(json["args"]["VoiceClone"]["ref_text"], "参考");
}

#[test]
fn writes_voice_design_params_file_with_expected_fields() {
    let spec = PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: "vox_cpm2".to_string(),
        model_version: "2B".to_string(),
        kind: PythonScriptTaskKind::VoiceDesign,
        runtime: runtime(),
        args: PythonScriptTaskArgs::VoiceDesign(VoiceDesignArgs {
            model_root_path: "/models/root".to_string(),
            speaker_dir_name: None,
            model_params_json: serde_json::json!({}),
            text: "生成".to_string(),
            language: "chinese".to_string(),
            instruct: "温柔".to_string(),
            output_path: "/out/vd.wav".to_string(),
        }),
    };

    let json = write_and_read(&spec, "vd");
    assert_eq!(json["kind"], "VoiceDesign");
    assert_eq!(json["args"]["VoiceDesign"]["instruct"], "温柔");
    assert_eq!(json["base_model"], "vox_cpm2");
}

#[test]
fn writes_training_params_file_with_expected_fields() {
    let spec = PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: "qwen3_tts".to_string(),
        model_version: "1.7B".to_string(),
        kind: PythonScriptTaskKind::Training,
        runtime: runtime(),
        args: PythonScriptTaskArgs::Training(TrainingArgs {
            model_root_path: "/models/root".to_string(),
            speaker_dir_name: None,
            model_params_json: serde_json::json!({}),
            input_jsonl: "/in/manifest.jsonl".to_string(),
            output_jsonl: "/out/manifest.jsonl".to_string(),
            output_model_path: "/out/model".to_string(),
            batch_size: 4,
            lr: Some("2e-5".to_string()),
            num_epochs: 12,
            speaker_name: "Alice".to_string(),
            gradient_accumulation_steps: 2,
        }),
    };

    let json = write_and_read(&spec, "train");
    assert_eq!(json["kind"], "Training");
    assert_eq!(json["args"]["Training"]["input_jsonl"], "/in/manifest.jsonl");
    assert_eq!(json["args"]["Training"]["batch_size"], 4);
    assert_eq!(json["args"]["Training"]["num_epochs"], 12);
    assert_eq!(json["args"]["Training"]["lr"], "2e-5");
}

#[test]
fn writes_streaming_params_file_with_expected_fields() {
    let spec = PythonScriptInvocationSpec {
        version: "1.0.0".to_string(),
        base_model: "gpt_sovits_cpufast".to_string(),
        model_version: "v1".to_string(),
        kind: PythonScriptTaskKind::StreamingSpeech,
        runtime: runtime(),
        args: PythonScriptTaskArgs::Streaming(StreamingArgs {
            context_file_path: "/ctx/context.json".to_string(),
            input_cache_file_path: "/ctx/input.jsonl".to_string(),
            output_audio_dir: "/ctx/audio".to_string(),
            model_root_path: String::new(),
            frames_file_path: "/ctx/frames.jsonl".to_string(),
            model_params_json: Value::Null,
            speakers: vec![StreamingSpeakerArg {
                name: "A".to_string(),
                ref_audio_path: "/ref.wav".to_string(),
                ref_text: "参考".to_string(),
                speaker_dir_name: None,
                category: String::new(),
            }],
        }),
    };

    let json = write_and_read(&spec, "streaming");
    assert_eq!(json["kind"], "StreamingSpeech");
    assert_eq!(json["args"]["Streaming"]["context_file_path"], "/ctx/context.json");
    assert_eq!(json["args"]["Streaming"]["input_cache_file_path"], "/ctx/input.jsonl");
    assert_eq!(json["args"]["Streaming"]["frames_file_path"], "/ctx/frames.jsonl");
    assert_eq!(json["args"]["Streaming"]["speakers"][0]["name"], "A");
    assert_eq!(json["args"]["Streaming"]["speakers"][0]["ref_text"], "参考");
}
