// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 协议适配层 — REST / gRPC / WebSocket / SOAP / 文件

/// 协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Rest,
    Grpc,
    WebSocket,
    Soap,
    File,
    Custom,
}

impl Protocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            Protocol::Rest => "rest",
            Protocol::Grpc => "grpc",
            Protocol::WebSocket => "websocket",
            Protocol::Soap => "soap",
            Protocol::File => "file",
            Protocol::Custom => "custom",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rest" | "http" | "https" => Protocol::Rest,
            "grpc" | "grpc-web" => Protocol::Grpc,
            "websocket" | "ws" | "wss" => Protocol::WebSocket,
            "soap" => Protocol::Soap,
            "file" | "csv" | "json" | "xml" => Protocol::File,
            _ => Protocol::Custom,
        }
    }
}

/// REST 请求方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

/// REST 协议适配器
pub struct RestAdapter {
    client: reqwest::Client,
    base_url: String,
    default_headers: std::collections::HashMap<String, String>,
}

impl RestAdapter {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            default_headers: std::collections::HashMap::new(),
        }
    }

    pub fn with_default_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_headers.insert(key.into(), value.into());
        self
    }

    pub fn with_bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.default_headers.insert("Authorization".into(), format!("Bearer {}", token.into()));
        self
    }

    /// 发送REST请求
    pub async fn request(
        &self,
        method: HttpMethod,
        path: &str,
        body: Option<serde_json::Value>,
        extra_headers: &std::collections::HashMap<String, String>,
    ) -> Result<(u16, serde_json::Value, std::collections::HashMap<String, String>), reqwest::Error> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let mut builder = match method {
            HttpMethod::Get => self.client.get(&url),
            HttpMethod::Post => self.client.post(&url),
            HttpMethod::Put => self.client.put(&url),
            HttpMethod::Patch => self.client.patch(&url),
            HttpMethod::Delete => self.client.delete(&url),
            HttpMethod::Head => self.client.head(&url),
            HttpMethod::Options => self.client.request(reqwest::Method::OPTIONS, &url),
        };

        // 添加默认头
        for (k, v) in &self.default_headers {
            builder = builder.header(k, v);
        }
        // 添加额外头
        for (k, v) in extra_headers {
            builder = builder.header(k, v);
        }
        // 添加body
        if let Some(b) = body {
            builder = builder.json(&b);
        }

        let resp = builder.send().await?;
        let status = resp.status().as_u16();
        let headers: std::collections::HashMap<String, String> = resp.headers().iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
            .collect();
        let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::Value::Null);

        Ok((status, body, headers))
    }
}
