// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 市场API客户端 — HTTP封装 + 认证 + 缓存 + 重试

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// 市场API客户端
#[derive(Clone)]
pub struct MarketClient {
    base_url: String,
    api_token: Option<String>,
    client: reqwest::Client,
    /// 简单内存缓存：key -> (value, expire_at)
    cache: Arc<RwLock<HashMap<String, (serde_json::Value, Instant)>>>,
    /// 缓存默认TTL
    cache_ttl: Duration,
    /// 最大重试次数
    max_retries: u32,
}

impl MarketClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("failed to build market client");
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_token: None,
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
            max_retries: 2,
        }
    }

    pub fn with_api_token(mut self, token: impl Into<String>) -> Self {
        self.api_token = Some(token.into());
        self
    }

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// 构建完整URL
    fn build_url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// 构建带认证的请求
    fn build_request(&self, method: reqwest::Method, url: &str) -> reqwest::RequestBuilder {
        let mut builder = self.client.request(method, url);
        if let Some(token) = &self.api_token {
            builder = builder.bearer_auth(token);
        }
        builder
    }

    /// GET请求（带缓存）
    pub async fn get(&self, path: &str, query: Option<&HashMap<String, String>>) -> Result<serde_json::Value, MarketClientError> {
        let cache_key = format!("GET:{}:{:?}", path, query);

        // 检查缓存
        if let Some((value, expire_at)) = self.cache.read().get(&cache_key) {
            if *expire_at > Instant::now() {
                return Ok(value.clone());
            }
        }

        // 发起请求（带重试）
        let url = self.build_url(path);
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            let mut builder = self.build_request(reqwest::Method::GET, &url);
            if let Some(q) = query {
                builder = builder.query(q);
            }
            match builder.send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);
                        // 写入缓存
                        self.cache.write().insert(cache_key, (body.clone(), Instant::now() + self.cache_ttl));
                        return Ok(body);
                    }
                    last_error = Some(MarketClientError::HttpError { status: status.as_u16(), message: format!("HTTP {}", status) });
                }
                Err(e) => {
                    last_error = Some(MarketClientError::NetworkError(e.to_string()));
                }
            }
            if attempt < self.max_retries {
                tokio::time::sleep(Duration::from_millis(200 * 2u64.pow(attempt))).await;
            }
        }

        Err(last_error.unwrap_or(MarketClientError::Other("unknown error".into())))
    }

    /// POST请求（不缓存）
    pub async fn post(&self, path: &str, body: &serde_json::Value) -> Result<serde_json::Value, MarketClientError> {
        let url = self.build_url(path);
        let resp = self.build_request(reqwest::Method::POST, &url)
            .json(body)
            .send()
            .await?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();
        if status.is_success() {
            serde_json::from_str(&body_text).map_err(|e| MarketClientError::ParseError(e.to_string()))
        } else {
            Err(MarketClientError::HttpError { status: status.as_u16(), message: body_text })
        }
    }

    /// 下载文件（流式）
    pub async fn download(&self, url: &str) -> Result<Vec<u8>, MarketClientError> {
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Err(MarketClientError::HttpError {
                status: resp.status().as_u16(),
                message: format!("download failed: HTTP {}", resp.status()),
            });
        }
        let bytes = resp.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// 清除缓存
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// 清除指定路径的缓存
    pub fn invalidate_cache(&self, path: &str) {
        let prefix = format!("GET:{}:", path);
        self.cache.write().retain(|k, _| !k.starts_with(&prefix));
    }
}

/// 市场客户端错误
#[derive(Debug, thiserror::Error)]
pub enum MarketClientError {
    #[error("network error: {0}")]
    NetworkError(String),
    #[error("HTTP error {status}: {message}")]
    HttpError { status: u16, message: String },
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("authentication failed: {0}")]
    AuthError(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("other error: {0}")]
    Other(String),
}

impl From<reqwest::Error> for MarketClientError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            MarketClientError::NetworkError("timeout".into())
        } else if e.is_connect() {
            MarketClientError::NetworkError(e.to_string())
        } else {
            MarketClientError::Other(e.to_string())
        }
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_url() {
        let client = MarketClient::new("https://market.example.com/api/v1/");
        assert_eq!(client.build_url("/plugins"), "https://market.example.com/api/v1/plugins");
        assert_eq!(client.build_url("plugins"), "https://market.example.com/api/v1/plugins");
    }
}
