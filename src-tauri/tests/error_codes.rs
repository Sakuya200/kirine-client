//! 覆盖 AppError 错误码结构的序列化契约：code/params 传递、plain 错误无 code、
//! anyhow::Error 自动映射为 plain。前端 `useErrorMessage` 依赖该 JSON 形状。

use std::collections::HashMap;

use kirine_client_lib::error::{codes, AppError};

#[test]
fn coded_error_serializes_with_code_and_params() {
    let err = AppError::coded(codes::MODEL_DEVICE_UNSUPPORTED, "模型 dots_tts 不支持设备 Cpu")
        .with_param("model", "dots_tts")
        .with_param("device", "Cpu");

    let json = serde_json::to_value(&err).unwrap();

    assert_eq!(json["code"], codes::MODEL_DEVICE_UNSUPPORTED);
    assert_eq!(json["params"]["model"], "dots_tts");
    assert_eq!(json["params"]["device"], "Cpu");
    assert!(json["message"].is_string());
}

#[test]
fn plain_error_has_no_code() {
    let err = AppError::plain("boom");
    let json = serde_json::to_value(&err).unwrap();

    assert!(json["code"].is_null());
    assert_eq!(json["message"], "boom");
    assert!(json["params"].is_object());
}

#[test]
fn anyhow_error_maps_to_plain() {
    let err: AppError = anyhow::anyhow!("原始失败").into();

    assert!(err.code.is_none());
    assert_eq!(err.message, "原始失败");
    assert!(err.params.is_empty());
}

#[test]
fn display_shows_message() {
    let err = AppError::coded(codes::MODEL_NOT_INSTALLED, "请先在模型管理页安装后再执行任务");

    assert_eq!(err.to_string(), "请先在模型管理页安装后再执行任务");
}

#[test]
fn deserialization_tolerates_missing_params() {
    // 前端回传或日志还原场景：params 缺省时应为空表
    let json = r#"{"code": "task.handleReadFailed", "message": "无法读取运行中任务句柄"}"#;
    let err: AppError = serde_json::from_str(json).unwrap();

    assert_eq!(err.code.as_deref(), Some(codes::TASK_HANDLE_READ_FAILED));
    assert_eq!(err.params, HashMap::new());
}
