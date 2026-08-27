// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Webhook 连接器 — 向外部URL发送HTTP回调

use crate::protocol::{HttpMethod, RestAdapter};
use crate::traits::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Webhook 连接器
pub struct WebhookConnector {
    config: ConnectorConfig,
    adapter: Arc<RestAdapter>,
}

impl WebhookConnector {
    pub fn new(config: ConnectorConfig) -> Self {
        let mut adapter = RestAdapter::new(&config.endpoint);
        // 添加认证头
        match config.auth_type.as_str() {
            "bearer" => {
                if let Some(token) = config.credentials.get("token") {
                    adapter = adapter.with_bearer_auth(token);
                }
            }
            "api_key" => {
                if let Some(key) = config.credentials.get("api_key") {
                    let header_name = config.credentials.get("header_name").cloned().unwrap_or_else(|| "X-API-Key".into());
                    adapter = adapter.with_default_header(header_name, key);
                }
            }
            "basic" => {
                if let (Some(user), Some(pass)) = (config.credentials.get("username"), config.credentials.get("password")) {
                    let encoded = base64_encode(&format!("{}:{}", user, pass));
                    adapter = adapter.with_default_header("Authorization", format!("Basic {}", encoded));
                }
            }
            _ => {}
        }
        // 添加配置中的默认头
        for (k, v) in &config.headers {
            adapter = adapter.with_default_header(k.clone(), v.clone());
        }
        Self { config, adapter: Arc::new(adapter) }
    }

    fn method_from_operation(operation: &str) -> HttpMethod {
        match operation.to_lowercase().as_str() {
            "get" | "query" | "fetch" | "list" => HttpMethod::Get,
            "post" | "create" | "send" | "trigger" | "notify" => HttpMethod::Post,
            "put" | "update" | "replace" => HttpMethod::Put,
            "patch" | "modify" => HttpMethod::Patch,
            "delete" | "remove" => HttpMethod::Delete,
            _ => HttpMethod::Post,
        }
    }
}

fn base64_encode(input: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = input.as_bytes();
    let mut result = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] as u32 } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if i + 1 < bytes.len() {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if i + 2 < bytes.len() {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
        i += 3;
    }
    result
}

#[async_trait]
impl Connector for WebhookConnector {
    fn connector_id(&self) -> &str { &self.config.id }
    fn connector_name(&self) -> &str { &self.config.name }
    fn connector_type(&self) -> ConnectorType { ConnectorType::Webhook }

    fn supported_protocols(&self) -> Vec<String> {
        vec!["rest".into(), "http".into(), "https".into()]
    }

    fn supported_operations(&self) -> Vec<String> {
        vec!["get".into(), "post".into(), "put".into(), "patch".into(), "delete".into(),
             "send".into(), "notify".into(), "trigger".into(), "query".into(), "create".into(), "update".into(), "delete_op".into()]
    }

    async fn connect(&self) -> ConnectorResult<()> {
        // Webhook不需要持久连接
        Ok(())
    }

    async fn execute(&self, request: &ConnectorRequest) -> ConnectorResult<ConnectorResponse> {
        if !self.config.enabled {
            return Err(ConnectorError::Disabled(self.config.id.clone()));
        }

        let start = std::time::Instant::now();
        let method = Self::method_from_operation(&request.operation);
        let path = request.params.get("path").cloned().unwrap_or_else(|| "".into());

        let mut headers = HashMap::new();
        for (k, v) in &request.headers {
            headers.insert(k.clone(), v.clone());
        }

        let (status, body, resp_headers) = self.adapter
            .request(method, &path, Some(request.body.clone()), &headers)
            .await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        let success = status >= 200 && status < 300;

        Ok(ConnectorResponse {
            success,
            status_code: status,
            body,
            headers: resp_headers,
            latency_ms,
            error: if success { None } else { Some(format!("HTTP {}", status)) },
            retries: 0,
        })
    }

    async fn health_check(&self) -> ConnectorResult<bool> {
        // 发送一个简单的GET请求到endpoint
        let result = self.adapter.request(HttpMethod::Get, "", None, &HashMap::new()).await;
        match result {
            Ok((status, _, _)) => Ok(status >= 200 && status < 500),
            Err(_) => Ok(false),
        }
    }

    async fn close(&self) -> ConnectorResult<()> {
        Ok(())
    }
}
