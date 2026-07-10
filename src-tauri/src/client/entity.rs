use serde::{Deserialize, Serialize};

use crate::{
    service::models::{Page, PageRequest as ServicePageRequest, SpeakerInfo, SpeakerPageResult},
    Result,
};

const MAX_CLIENT_ERROR_MESSAGE_CHARS: usize = 420;

fn truncate_client_error_message(message: String) -> String {
    let trimmed = message.trim();
    let mut chars = trimmed.chars();
    let head = chars
        .by_ref()
        .take(MAX_CLIENT_ERROR_MESSAGE_CHARS)
        .collect::<String>();

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

/// 分页请求结构体（client 层统一规范，与 `service::models::PageRequest` 字段对齐）。
/// page: 页码
/// page_size: 每页条数
/// filter: 过滤条件
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageRequest<T> {
    pub page: u32,
    pub page_size: u32,
    pub filter: Option<T>,
}

/// 通用分页响应结构体（client 层统一规范，与 `service::models::Page` 字段对齐）。
/// items: 当前页数据
/// total: 总条数
/// page: 当前页码
/// page_size: 每页条数
/// total_pages: 总页数
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// 说话人分页响应（附带统计，供页面统计卡使用），与 `service::models::SpeakerPageResult` 对齐。
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerPageResponse {
    pub items: Vec<SpeakerInfo>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
    pub ready_count: u64,
    pub training_count: u64,
    pub disabled_count: u64,
    pub total_samples: u64,
}

// ---- 与 service 层领域类型的边界转换 ----

impl<T> From<ServicePageRequest<T>> for PageRequest<T> {
    fn from(req: ServicePageRequest<T>) -> Self {
        Self {
            page: req.page,
            page_size: req.page_size,
            filter: req.filter,
        }
    }
}

impl<T> From<PageResponse<T>> for Page<T> {
    fn from(resp: PageResponse<T>) -> Self {
        Self {
            items: resp.items,
            total: resp.total,
            page: resp.page,
            page_size: resp.page_size,
            total_pages: resp.total_pages,
        }
    }
}

impl From<SpeakerPageResponse> for SpeakerPageResult {
    fn from(resp: SpeakerPageResponse) -> Self {
        Self {
            items: resp.items,
            total: resp.total,
            page: resp.page,
            page_size: resp.page_size,
            total_pages: resp.total_pages,
            ready_count: resp.ready_count,
            training_count: resp.training_count,
            disabled_count: resp.disabled_count,
            total_samples: resp.total_samples,
        }
    }
}
