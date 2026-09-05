//! 流式契约 serde 往返测试：验证 StreamingArgs / StreamingSpeakerArg / StreamingSpeaker
//! 新增字段（model_root_path / streaming_socket_addr / streaming_socket_token /
//! model_params_json / speaker_dir_name / category / side / avatar_path / avatar_name）
//! 在序列化-反序列化往返中保持，且缺省时按 serde(default) 规则回填。

use serde_json::Value;

use kirine_client_lib::test_support::{
    models, StreamingArgs, StreamingSpeaker, StreamingSpeakerArg,
};

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
        side: "left".into(),
        avatar_path: Some("%DATA_DIR_PATH%/samples/streaming-speech_9/avatar_1_spk1.png".into()),
        avatar_name: Some("头像.png".into()),
    };
    let json = serde_json::to_string(&spk).expect("serialize");
    let back: StreamingSpeaker = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.category, "trained");
    assert_eq!(back.speaker_dir_name.as_deref(), Some("42"));
    assert_eq!(back.side, "left");
    assert_eq!(
        back.avatar_path.as_deref(),
        Some("%DATA_DIR_PATH%/samples/streaming-speech_9/avatar_1_spk1.png")
    );
    assert_eq!(back.avatar_name.as_deref(), Some("头像.png"));
    // camelCase 字段名
    assert!(json.contains("\"side\""));
    assert!(json.contains("\"avatarPath\""));
    assert!(json.contains("\"avatarName\""));
}

#[test]
fn streaming_speaker_defaults_when_absent() {
    let json = r#"{"id":"s","name":"n","baseModel":"m","refAudioPath":"/r","refAudioName":"r.wav","refText":"t"}"#;
    let spk: StreamingSpeaker = serde_json::from_str(json).expect("deserialize");
    assert_eq!(spk.category, "");
    assert!(spk.speaker_dir_name.is_none());
    assert!(spk.description.is_none());
    assert!(spk.model_version.is_none());
    assert_eq!(spk.side, "");
    assert!(spk.avatar_path.is_none());
    assert!(spk.avatar_name.is_none());
    // 静默引用 Value，避免未使用告警影响编译（严格 crate 视为告警即拒）。
    let _v: Value = Value::Null;
}

#[test]
fn streaming_speaker_input_roundtrip_preserves_avatar_fields() {
    // 经 JSON 文本构造，覆盖前端 CreateStreamingSpeechTaskPayload 与回放快照两条
    // 使用路径上 StreamingSpeakerInput 的 camelCase 契约。
    let json = r#"{"name":"A","baseModel":"moss_tts_realtime","refAudioPath":"/r","refAudioName":"r.wav","refText":"t","category":"voice-clone","side":"right","avatarPath":"C:/pics/a.png","avatarName":"a.png"}"#;
    let input: models::StreamingSpeakerInput = serde_json::from_str(json).expect("deserialize");
    assert_eq!(input.side, "right");
    assert_eq!(input.avatar_path.as_deref(), Some("C:/pics/a.png"));
    assert_eq!(input.avatar_name.as_deref(), Some("a.png"));
    let back = serde_json::to_string(&input).expect("serialize");
    assert!(back.contains("\"side\":\"right\""));
    assert!(back.contains("\"avatarPath\":\"C:/pics/a.png\""));
    assert!(back.contains("\"avatarName\":\"a.png\""));
}

#[test]
fn streaming_speaker_input_defaults_when_absent() {
    let json = r#"{"name":"A","baseModel":"moss_tts_realtime","refAudioPath":"/r","refAudioName":"r.wav","refText":"t"}"#;
    let input: models::StreamingSpeakerInput = serde_json::from_str(json).expect("deserialize");
    assert_eq!(input.side, "");
    assert!(input.avatar_path.is_none());
    assert!(input.avatar_name.is_none());
}
