// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # Registry Svc HTTP 客户端
//!
//! 网关调用 registry-svc 的 HTTP 客户端封装。
//! 用于替换内嵌 experts_registry 模块的数据访问。

use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;

/// Registry Svc 客户端
#[derive(Clone)]
pub struct RegistryClient {
    http: Client,
    base_url: String,
}

impl RegistryClient {
    /// 创建新客户端
    pub fn new(base_url: String) -> Self {
        let mut builder = Client::builder().timeout(Duration::from_secs(10));
        // 内部服务间鉴权：与下游 svc internal_auth_layer 同源同令牌。
        // 配置 MOX_INTERNAL_TOKEN 后所有出站请求自动携带 Authorization: Bearer <token>；
        // 未配置则不注入（下游中间件未配置时放行，向后兼容）。
        if let Ok(token) = std::env::var("MOX_INTERNAL_TOKEN") {
            if !token.is_empty() {
                let mut headers = axum::http::HeaderMap::new();
                headers.insert(
                    axum::http::header::AUTHORIZATION,
                    format!("Bearer {token}").parse().unwrap(),
                );
                builder = builder.default_headers(headers);
            }
        }
        let http = builder.build().unwrap_or_else(|e| {
            // N6 修复：不再 .expect() 恐慌；记录错误后降级为默认 client（无超时/鉴权头）
            tracing::error!("registry client http build failed: {e}; falling back to default client");
            Client::new()
        });

        Self { http, base_url }
    }

    /// 健康检查
    pub async fn health(&self) -> Result<bool, String> {
        let url = format!("{}/health", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 列出专家
    pub async fn list_experts(&self, domain: Option<&str>) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/experts", self.base_url);
        let mut req = self.http.get(&url);
        if let Some(d) = domain {
            req = req.query(&[("domain", d)]);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    /// 获取专家详情
    pub async fn get_expert(&self, id: &str) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/experts/{}", self.base_url, id);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    /// 创建专家
    pub async fn create_expert(&self, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/experts", self.base_url);
        let resp = self.http.post(&url).json(&body).send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    /// 更新专家
    pub async fn update_expert(&self, id: &str, body: serde_json::Value) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/experts/{}", self.base_url, id);
        let resp = self.http.put(&url).json(&body).send().await.map_err(|e| e.to_string())?;
        resp.json().await.map_err(|e| e.to_string())
    }

    /// 删除专家
    pub async fn delete_expert(&self, id: &str) -> Result<(), String> {
        let url = format!("{}/api/v1/experts/{}", self.base_url, id);
        self.http.delete(&url).send().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// 共享客户端状态
#[derive(Clone)]
pub struct SharedRegistryClient {
    pub client: Arc<RegistryClient>,
}

impl SharedRegistryClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: Arc::new(RegistryClient::new(base_url)),
        }
    }
}
