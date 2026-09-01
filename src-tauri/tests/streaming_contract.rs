//! 流式契约 serde 往返测试：验证 StreamingArgs / StreamingSpeakerArg / StreamingSpeaker
//! 新增字段（model_root_path / streaming_socket_addr / streaming_socket_token /
//! model_params_json / speaker_dir_name / category）在序列化-反序列化往返中保持，
//! 且缺省时按 serde(default) 规则回填。

use serde_json::Value;

use kirine_client_lib::test_support::{StreamingArgs, StreamingSpeaker, StreamingSpeakerArg};

#[test]
fn streaming_args_roundtrip_preserves_trained_speaker() {
    let args = StreamingArgs {
        context_file_path: "/ctx.json".into(),
        input_cache_file_path: "/in.jsonl".into(),
        output_audio_dir: "/out".into(),
        model_root_path: "/models".into(),
        streaming_socket_addr: "127.0.0.1:54321".into(),
        streaming_socket_token: "tok-abc".into(),
        model_params_json: serde_json::json!({"temperature": 0.5}),
        speakers: vec![StreamingSpeakerArg {
            name: "spk1".into(),
            ref_audio_path: "/ref.wav".into(),
            ref_text: "hi".into(),
            speaker_dir_name: Some("42".into()),
            category: "trained".into(),
        }],
    };
    let json = serde_json::to_string(&args).expect("serialize");
    let back: StreamingArgs = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.model_root_path, "/models");
    assert_eq!(back.streaming_socket_addr, "127.0.0.1:54321");
    assert_eq!(back.streaming_socket_token, "tok-abc");
    assert_eq!(back.model_params_json["temperature"], 0.5);
    let spk = &back.speakers[0];
    assert_eq!(spk.category, "trained");
    assert_eq!(spk.speaker_dir_name.as_deref(), Some("42"));
}

#[test]
fn streaming_args_defaults_when_absent() {
    let json = r#"{"context_file_path":"/c","input_cache_file_path":"/i","output_audio_dir":"/o","speakers":[{"name":"n","ref_audio_path":"/r","ref_text":"t"}]}"#;
    let args: StreamingArgs = serde_json::from_str(json).expect("deserialize");
    assert_eq!(args.model_root_path, "");
    assert_eq!(args.streaming_socket_addr, "");
    assert_eq!(args.streaming_socket_token, "");
    assert!(args.model_params_json.is_null());
    assert_eq!(args.speakers[0].category, "");
    assert!(args.speakers[0].speaker_dir_name.is_none());
}

#[test]
fn streaming_speaker_roundtrip_preserves_category() {
    let spk = StreamingSpeaker {
        id: "spk1".into(),
        name: "spk1".into(),
        base_model: "moss_tts_realtime".into(),
        model_version: Some("1.7B".into()),
        ref_audio_path: "/r".into(),
        ref_audio_name: "r.wav".into(),
        ref_text: "t".into(),
        description: None,
        speaker_dir_name: Some("42".into()),
        category: "trained".into(),
    };
    let json = serde_json::to_string(&spk).expect("serialize");
    let back: StreamingSpeaker = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.category, "trained");
    assert_eq!(back.speaker_dir_name.as_deref(), Some("42"));
}

#[test]
fn streaming_speaker_defaults_when_absent() {
    let json = r#"{"id":"s","name":"n","baseModel":"m","refAudioPath":"/r","refAudioName":"r.wav","refText":"t"}"#;
    let spk: StreamingSpeaker = serde_json::from_str(json).expect("deserialize");
    assert_eq!(spk.category, "");
    assert!(spk.speaker_dir_name.is_none());
    assert!(spk.description.is_none());
    assert!(spk.model_version.is_none());
    // 静默引用 Value，避免未使用告警影响编译（严格 crate 视为告警即拒）。
    let _v: Value = Value::Null;
}
