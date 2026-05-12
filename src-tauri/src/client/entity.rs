use crate::Result;
use serde::{Deserialize, Serialize};

const MAX_CLIENT_ERROR_MESSAGE_CHARS: usize = 420;

fn truncate_client_error_message(message: String) -> String {
    let trimmed = message.trim();
    let mut chars = trimmed.chars();
    let head = chars.by_ref().take(MAX_CLIENT_ERROR_MESSAGE_CHARS).collect::<String>();

    if chars.next().is_some() {
        format!("{}...", head)
    } else {
        head
    }
}

/// 通用响应结构体
/// status_code: 状态码
/// data: 响应数据
/// message: 响应消息
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommonResponse<T> {
    pub code: i32,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> CommonResponse<T> {
    pub fn new(status_code: i32, data: Option<T>, message: Option<String>) -> Self {
        CommonResponse {
            code: status_code,
            data,
            message,
        }
    }

    pub fn success(data: Option<T>) -> Self {
        CommonResponse {
            code: 200,
            data,
            message: None,
        }
    }

    pub fn error(status_code: i32, message: String) -> Self {
        CommonResponse {
            code: status_code,
            data: None,
            message: Some(truncate_client_error_message(message)),
        }
    }

    pub fn from_result(res: Result<T>) -> Self {
        match res {
            Ok(data) => CommonResponse::success(Some(data)),
            Err(e) => CommonResponse::error(500, e.to_string()),
        }
    }
}
