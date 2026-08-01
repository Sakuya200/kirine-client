//! 通用 HTTP 客户端封装：统一处理超时、鉴权（Bearer Token）与不同请求方式。
//!
//! 当前阶段为前瞻基础设施，`client` 模块尚未接入真实 HTTP 调用，故整体暂不使用。
//! 待远端接口正式联调时，由 `client::ApiClient` 通过本封装发起请求。

use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::{Client, Method, Response};
use serde::Serialize;

/// 默认请求超时（秒）。
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// 通用 HTTP 客户端，封装 `reqwest::Client`，统一注入鉴权头。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct HttpClient {
    inner: Client,
    token: Option<String>,
}

impl HttpClient {
    /// 构造客户端；`token` 为空字符串时视为未配置。
    pub(crate) fn new(token: Option<String>) -> Result<Self> {
        let inner = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .context("failed to build reqwest client")?;
        Ok(HttpClient {
            inner,
            token: token.filter(|t| !t.is_empty()),
        })
    }

    /// 构造请求并注入 Bearer Token（若已配置）。
    fn build(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.inner.request(method, url);
        if let Some(token) = self.token.as_deref() {
            request = request.bearer_auth(token);
        }
        request
    }

    /// GET 请求，`query` 为查询参数键值对（无参时传 `&[]`）。
    pub(crate) async fn get(&self, url: &str, query: &[(&str, &str)]) -> Result<Response> {
        self.build(Method::GET, url)
            .query(query)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("GET {url} 失败: {e}"))
    }

    /// POST 请求，`body` 以 JSON 发送。
    pub(crate) async fn post<T: Serialize>(&self, url: &str, body: &T) -> Result<Response> {
        self.build(Method::POST, url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("POST {url} 失败: {e}"))
    }

    /// PUT 请求，`body` 以 JSON 发送。
    pub(crate) async fn put<T: Serialize>(&self, url: &str, body: &T) -> Result<Response> {
        self.build(Method::PUT, url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("PUT {url} 失败: {e}"))
    }

    /// DELETE 请求。
    pub(crate) async fn delete(&self, url: &str) -> Result<Response> {
        self.build(Method::DELETE, url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("DELETE {url} 失败: {e}"))
    }
}
