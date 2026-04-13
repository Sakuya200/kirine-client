use crate::Result;
use serde::{Deserialize, Serialize};

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
            message: Some(message),
        }
    }

    pub fn from_result(res: Result<T>) -> Self {
        match res {
            Ok(data) => CommonResponse::success(Some(data)),
            Err(e) => CommonResponse::error(500, e.to_string()),
        }
    }
}
