//! # AI Provider 抽象层
//!
//! 统一抽象三种 AI 后端：
//! - OpenAI API (GPT-4o, GPT-4, etc.)
//! - Anthropic API (Claude 3.5, etc.)
//! - Local LLM (llm crate, 支持 llama/cuda 等本地模型)
//!
//! trait 设计遵循：
//! - Send + Sync（支持并发调用）
//! - chat() 返回 Stream 或一次性 Result<String, Error>
//! - 纯 std 实现，不依赖 reqwest（避免大量 transitive deps）

use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// 对话角色
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}

/// 单条对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
        }
    }
    fn role_str(&self) -> &'static str {
        match self.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
        }
    }
}

/// 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// 模型名称，如 "gpt-4o-mini"、"claude-3-5-sonnet"
    pub model: String,
    /// 最大生成 token 数
    pub max_tokens: usize,
    /// temperature，0.0~2.0
    pub temperature: f32,
    /// top_p
    pub top_p: Option<f32>,
    /// 停止序列
    pub stop: Vec<String>,
    /// 是否流式输出
    pub stream: bool,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model: "gpt-4o-mini".into(),
            max_tokens: 4096,
            temperature: 0.7,
            top_p: None,
            stop: vec![],
            stream: false,
        }
    }
}

/// AI Provider trait — 所有 AI 后端必须实现此接口
pub trait AiProvider: Send + Sync {
    /// 提供商类型名称
    fn provider_name(&self) -> &'static str;
    /// 可用模型列表
    fn available_models(&self) -> Vec<String>;
    /// 单轮对话（同步）
    fn chat_sync(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<String, AiProviderError>;
    /// 流式对话
    fn chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<Box<dyn AiStream + Send>, AiProviderError>;
    /// 估算 token 数量
    fn estimate_tokens(&self, text: &str) -> usize;
    /// 健康检查
    fn health_check(&self) -> bool;
}

/// 流式响应 trait
pub trait AiStream: Read + Send {
    fn collect(self: Box<Self>) -> Result<String, std::io::Error>;
}

/// AI Provider 错误类型
#[derive(Debug, Clone)]
pub enum AiProviderError {
    AuthError(String),
    NetworkError(String),
    ModelNotFound(String),
    RateLimited {
        retry_after_secs: Option<u64>,
        message: String,
    },
    ContentFiltered(String),
    Timeout,
    QuotaExceeded,
    Other(String),
}

impl std::fmt::Display for AiProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthError(s) => write!(f, "认证错误: {}", s),
            Self::NetworkError(s) => write!(f, "网络错误: {}", s),
            Self::ModelNotFound(s) => write!(f, "模型未找到: {}", s),
            Self::RateLimited {
                retry_after_secs,
                message,
            } => write!(
                f,
                "限流: {} (retry after {}s)",
                message,
                retry_after_secs.unwrap_or(0)
            ),
            Self::ContentFiltered(s) => write!(f, "内容过滤: {}", s),
            Self::Timeout => write!(f, "请求超时"),
            Self::QuotaExceeded => write!(f, "超出配额"),
            Self::Other(s) => write!(f, "错误: {}", s),
        }
    }
}

impl std::error::Error for AiProviderError {}

impl From<std::io::Error> for AiProviderError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::TimedOut {
            AiProviderError::Timeout
        } else {
            AiProviderError::NetworkError(e.to_string())
        }
    }
}

// ─── 纯 std HTTP 客户端 ──────────────────────────────────────────────────────

/// 基于 std::net::TcpStream 的 HTTP/1.1 客户端
struct HttpClient {
    host: String,
    port: u16,
    timeout_secs: u64,
}

impl HttpClient {
    fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            timeout_secs: 60,
        }
    }

    fn post(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<String, AiProviderError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| AiProviderError::NetworkError(format!("invalid address: {}", e)))?,
            Duration::from_secs(self.timeout_secs),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(self.timeout_secs)))?;

        let mut req = format!("POST {} HTTP/1.1\r\n", path);
        req.push_str(&format!("Host: {}\r\n", self.host));
        for (k, v) in headers {
            req.push_str(&format!("{}: {}\r\n", k, v));
        }
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        req.push_str("Content-Type: application/json\r\n");
        req.push_str("Accept: application/json\r\n");
        req.push_str("User-Agent: mox-ai/1.0\r\n");
        req.push_str("Connection: close\r\n");
        req.push_str("\r\n");
        req.push_str(body);

        stream.write_all(req.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut resp = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            resp.extend_from_slice(&buf[..n]);
        }

        // 解析 HTTP 响应
        let resp_str = String::from_utf8_lossy(&resp);
        let body_start = resp_str
            .find("\r\n\r\n")
            .ok_or_else(|| AiProviderError::NetworkError("invalid HTTP response".into()))?;
        let headers_part = &resp_str[..body_start];
        let body = &resp_str[body_start + 4..];

        // 检查状态码
        let status_line = headers_part.lines().next().unwrap_or("");
        let status_code: u16 = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        match status_code {
            200..=299 => Ok(body.to_string()),
            401 => Err(AiProviderError::AuthError("Invalid API key".into())),
            404 => Err(AiProviderError::ModelNotFound("model not found".into())),
            429 => Err(AiProviderError::RateLimited {
                retry_after_secs: headers_part
                    .lines()
                    .find(|l| l.starts_with("Retry-After:"))
                    .and_then(|l| l.split(':').nth(1))
                    .and_then(|s| s.trim().parse().ok()),
                message: "rate limited".into(),
            }),
            _ => Err(AiProviderError::Other(format!(
                "HTTP {}: {}",
                status_code,
                body.chars().take(200).collect::<String>()
            ))),
        }
    }
}

// ─── HttpClient 流式扩展 ──────────────────────────────────────────────────────

/// HTTP body 解码模式
enum BodyMode {
    Close,          // 连接关闭即结束（无 Content-Length / 无 chunked）
    Length(usize),  // Content-Length 指定长度
    Chunked,        // Transfer-Encoding: chunked
}

/// HTTP body 解码器：将 chunked 编码解码为纯字节流
struct HttpBodyReader<R: Read + Send> {
    inner: R,
    mode: BodyMode,
    done: bool,
}

impl<R: Read + Send> HttpBodyReader<R> {
    fn new(inner: R, mode: BodyMode) -> Self {
        Self {
            inner,
            mode,
            done: false,
        }
    }

    /// 读取一行（直到 \n），跳过 \r
    fn read_line(&mut self, line: &mut String) -> std::io::Result<()> {
        line.clear();
        let mut byte = [0u8; 1];
        loop {
            let n = self.inner.read(&mut byte)?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "unexpected EOF reading HTTP line",
                ));
            }
            let c = byte[0];
            if c == b'\n' {
                return Ok(());
            }
            if c == b'\r' {
                continue;
            }
            line.push(c as char);
        }
    }
}

impl<R: Read + Send> Read for HttpBodyReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.done {
            return Ok(0);
        }
        match self.mode {
            BodyMode::Close => self.inner.read(buf),
            BodyMode::Length(0) => {
                self.done = true;
                Ok(0)
            }
            BodyMode::Length(ref mut n) => {
                let to_read = std::cmp::min(buf.len(), *n);
                let got = self.inner.read(&mut buf[..to_read])?;
                *n -= got;
                if *n == 0 {
                    self.done = true;
                }
                Ok(got)
            }
            BodyMode::Chunked => {
                // 读取 chunk-size 行（跳过空行）
                let mut size_line = String::new();
                loop {
                    self.read_line(&mut size_line)?;
                    if !size_line.is_empty() {
                        break;
                    }
                }
                // chunk-size 可能带 chunk-extension（如 "1a;ext"），取 ; 前部分
                let size_hex = size_line.split(';').next().unwrap_or("0").trim();
                let size: usize = usize::from_str_radix(size_hex, 16).unwrap_or(0);
                if size == 0 {
                    self.done = true;
                    return Ok(0);
                }
                // 读取 size 字节数据
                let mut total = 0;
                while total < size {
                    let want = std::cmp::min(buf.len() - total, size - total);
                    if want == 0 {
                        break;
                    }
                    let got = self.inner.read(&mut buf[total..total + want])?;
                    if got == 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "chunk truncated",
                        ));
                    }
                    total += got;
                }
                // 读取 chunk 后的 CRLF
                let mut crlf = String::new();
                let _ = self.read_line(&mut crlf);
                Ok(total)
            }
        }
    }
}

impl HttpClient {
    /// 发起 POST 请求并返回响应 body 流（支持 chunked 解码）
    ///
    /// 用于流式（SSE）场景：调用方逐字节读取，解析 SSE `data:` 行
    fn post_stream(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> Result<Box<dyn Read + Send>, AiProviderError> {
        let addr = format!("{}:{}", self.host, self.port);
        let mut stream = TcpStream::connect_timeout(
            &addr
                .parse()
                .map_err(|e| AiProviderError::NetworkError(format!("invalid address: {}", e)))?,
            Duration::from_secs(self.timeout_secs),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(self.timeout_secs)))?;

        let mut req = format!("POST {} HTTP/1.1\r\n", path);
        req.push_str(&format!("Host: {}\r\n", self.host));
        for (k, v) in headers {
            req.push_str(&format!("{}: {}\r\n", k, v));
        }
        req.push_str(&format!("Content-Length: {}\r\n", body.len()));
        req.push_str("Content-Type: application/json\r\n");
        req.push_str("Accept: application/json\r\n");
        req.push_str("User-Agent: mox-ai/1.0\r\n");
        req.push_str("Connection: close\r\n");
        req.push_str("\r\n");
        req.push_str(body);

        stream.write_all(req.as_bytes())?;
        stream.shutdown(std::net::Shutdown::Write)?;

        let mut buf_stream = BufReader::new(stream);

        // 读取并解析 header，确定 body 模式
        let mut header_lines = Vec::new();
        let mut line = String::new();
        loop {
            line.clear();
            let mut byte = [0u8; 1];
            loop {
                let n = buf_stream.read(&mut byte)?;
                if n == 0 {
                    break;
                }
                let c = byte[0];
                if c == b'\n' {
                    break;
                }
                if c == b'\r' {
                    continue;
                }
                line.push(c as char);
            }
            if line.is_empty() {
                break; // header/body 分隔
            }
            header_lines.push(line.clone());
        }

        let mut is_chunked = false;
        let mut content_length: Option<usize> = None;
        for h in &header_lines {
            let lower = h.to_lowercase();
            if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
                is_chunked = true;
            } else if lower.starts_with("content-length:") {
                if let Some(v) = h.split(':').nth(1) {
                    content_length = v.trim().parse().ok();
                }
            }
        }

        let mode = if is_chunked {
            BodyMode::Chunked
        } else if let Some(n) = content_length {
            BodyMode::Length(n)
        } else {
            BodyMode::Close
        };

        Ok(Box::new(HttpBodyReader::new(buf_stream, mode)))
    }
}

// ─── OpenAI SSE 流式解析 ──────────────────────────────────────────────────────

/// OpenAI SSE 流式响应解析器
///
/// 从 SSE 流中逐行读取 `data:` 行，解析 JSON 提取 `choices[0].delta.content`，
/// 作为 `Read` 输出累积的文本内容。
pub struct OpenAiSseStream {
    inner: Box<dyn Read + Send>,
    line_buf: String,
    out_buf: Vec<u8>,
    done: bool,
}

impl OpenAiSseStream {
    pub fn new(inner: Box<dyn Read + Send>) -> Self {
        Self {
            inner,
            line_buf: String::new(),
            out_buf: Vec::new(),
            done: false,
        }
    }
}

impl Read for OpenAiSseStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.done && self.out_buf.is_empty() {
            return Ok(0);
        }
        if !self.out_buf.is_empty() {
            let n = std::cmp::min(buf.len(), self.out_buf.len());
            buf[..n].copy_from_slice(&self.out_buf[..n]);
            self.out_buf.drain(..n);
            return Ok(n);
        }

        loop {
            if self.done {
                return Ok(0);
            }
            // 读一行
            self.line_buf.clear();
            let mut byte = [0u8; 1];
            loop {
                let n = self.inner.read(&mut byte)?;
                if n == 0 {
                    self.done = true;
                    break;
                }
                let c = byte[0];
                if c == b'\n' {
                    break;
                }
                if c == b'\r' {
                    continue;
                }
                self.line_buf.push(c as char);
            }

            let line = self.line_buf.trim();
            if line.starts_with("data:") {
                let data = line["data:".len()..].trim();
                if data == "[DONE]" {
                    self.done = true;
                    return Ok(0);
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(content) = json
                        .get("choices")
                        .and_then(|c| c.get(0))
                        .and_then(|c| c.get("delta"))
                        .and_then(|d| d.get("content"))
                        .and_then(|c| c.as_str())
                    {
                        self.out_buf.extend_from_slice(content.as_bytes());
                    }
                }
            }

            if !self.out_buf.is_empty() {
                let n = std::cmp::min(buf.len(), self.out_buf.len());
                buf[..n].copy_from_slice(&self.out_buf[..n]);
                self.out_buf.drain(..n);
                return Ok(n);
            }
            if self.done {
                return Ok(0);
            }
        }
    }
}

impl AiStream for OpenAiSseStream {
    fn collect(mut self: Box<Self>) -> Result<String, std::io::Error> {
        let mut s = String::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = self.read(&mut buf)?;
            if n == 0 {
                break;
            }
            s.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        Ok(s)
    }
}

// ─── OpenAI Provider ─────────────────────────────────────────────────────────

/// OpenAI API Provider
pub struct OpenAiProvider {
    api_key: String,
    client: HttpClient,
}

impl OpenAiProvider {
    /// 新建 Provider，默认指向 api.openai.com
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://api.openai.com".into())
    }

    /// 自定义 base URL（支持代理/本地部署）
    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let (host, port) = parse_url_host_port(&base_url);
        Self {
            api_key,
            client: HttpClient::new(host, port),
        }
    }

    fn do_chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> Result<String, AiProviderError> {
        let payload = serde_json::json!({
            "model": config.model,
            "messages": messages.iter().map(|m| {
                let mut obj = serde_json::Map::new();
                obj.insert("role".into(), serde_json::Value::String(m.role_str().into()));
                obj.insert("content".into(), serde_json::Value::String(m.content.clone()));
                if let Some(ref name) = m.name {
                    obj.insert("name".into(), serde_json::Value::String(name.clone()));
                }
                serde_json::Value::Object(obj)
            }).collect::<Vec<_>>(),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "top_p": config.top_p,
            "stop": config.stop,
        });

        let body = serde_json::to_string(&payload)
            .map_err(|e| AiProviderError::Other(format!("JSON序列化失败: {}", e)))?;

        let resp_body = self.client.post(
            "/v1/chat/completions",
            &[("Authorization", &format!("Bearer {}", self.api_key))],
            &body,
        )?;

        // 解析 OpenAI 响应
        let json: serde_json::Value = serde_json::from_str(&resp_body).map_err(|e| {
            AiProviderError::Other(format!(
                "响应JSON解析失败: {} | {}",
                e,
                &resp_body[..resp_body.len().min(200)]
            ))
        })?;

        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AiProviderError::Other(format!("响应格式异常: {}", json)))
    }
}

impl AiProvider for OpenAiProvider {
    fn provider_name(&self) -> &'static str {
        "OpenAI"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "gpt-4o".into(),
            "gpt-4o-mini".into(),
            "gpt-4-turbo".into(),
            "gpt-4".into(),
            "gpt-3.5-turbo".into(),
        ]
    }

    fn chat_sync(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<String, AiProviderError> {
        self.do_chat(messages, config)
    }

    fn chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<Box<dyn AiStream + Send>, AiProviderError> {
        let payload = serde_json::json!({
            "model": config.model,
            "messages": messages.iter().map(|m| {
                let mut obj = serde_json::Map::new();
                obj.insert("role".into(), serde_json::Value::String(m.role_str().into()));
                obj.insert("content".into(), serde_json::Value::String(m.content.clone()));
                if let Some(ref name) = m.name {
                    obj.insert("name".into(), serde_json::Value::String(name.clone()));
                }
                serde_json::Value::Object(obj)
            }).collect::<Vec<_>>(),
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "top_p": config.top_p,
            "stop": config.stop,
            "stream": true,
        });

        let body = serde_json::to_string(&payload)
            .map_err(|e| AiProviderError::Other(format!("JSON序列化失败: {}", e)))?;

        let stream = self.client.post_stream(
            "/v1/chat/completions",
            &[("Authorization", &format!("Bearer {}", self.api_key))],
            &body,
        )?;

        Ok(Box::new(OpenAiSseStream::new(stream)))
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        // 粗略估算：中文约 1.5 token/字符，英文约 0.25 token/词
        let chinese_chars = text.chars().filter(|c| !c.is_ascii()).count();
        let english_words = text
            .split_whitespace()
            .filter(|w| w.chars().all(|c| c.is_ascii()))
            .count();
        (chinese_chars * 3 / 2) + (english_words / 4)
    }

    fn health_check(&self) -> bool {
        self.do_chat(
            &[ChatMessage::user("ping")],
            &ModelConfig {
                max_tokens: 5,
                ..Default::default()
            },
        )
        .is_ok()
    }
}

// ─── Anthropic Provider ──────────────────────────────────────────────────────

/// Anthropic API Provider（Claude）
pub struct AnthropicProvider {
    api_key: String,
    client: HttpClient,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, "https://api.anthropic.com".into())
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        let (host, port) = parse_url_host_port(&base_url);
        Self {
            api_key,
            client: HttpClient::new(host, port),
        }
    }

    fn do_chat(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<String, AiProviderError> {
        // Anthropic 将 system 作为独立字段，messages 中不含 system role
        let system = messages
            .iter()
            .find(|m| m.role == Role::System)
            .map(|m| m.content.clone());

        let conv: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != Role::System)
            .map(|m| {
                serde_json::json!({
                    "role": m.role_str(),
                    "content": m.content,
                })
            })
            .collect();

        let mut payload = serde_json::json!({
            "model": config.model,
            "max_tokens": config.max_tokens,
            "messages": conv,
            "temperature": config.temperature,
        });
        if let Some(s) = system {
            payload["system"] = serde_json::Value::String(s);
        }

        let body = serde_json::to_string(&payload)
            .map_err(|e| AiProviderError::Other(format!("JSON序列化失败: {}", e)))?;

        let resp_body = self.client.post(
            "/v1/messages",
            &[
                ("x-api-key", &self.api_key),
                ("anthropic-version", "2023-06-01"),
            ],
            &body,
        )?;

        let json: serde_json::Value = serde_json::from_str(&resp_body).map_err(|e| {
            AiProviderError::Other(format!(
                "响应JSON解析失败: {} | {}",
                e,
                &resp_body[..resp_body.len().min(200)]
            ))
        })?;

        json.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AiProviderError::Other(format!("Anthropic 响应格式异常: {}", json))
            })
    }
}

impl AiProvider for AnthropicProvider {
    fn provider_name(&self) -> &'static str {
        "Anthropic"
    }

    fn available_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".into(),
            "claude-3-5-haiku-20241022".into(),
            "claude-3-opus-20240229".into(),
        ]
    }

    fn chat_sync(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> Result<String, AiProviderError> {
        self.do_chat(messages, config)
    }

    fn chat_stream(
        &self,
        _messages: &[ChatMessage],
        _config: &ModelConfig,
    ) -> Result<Box<dyn AiStream + Send>, AiProviderError> {
        Err(AiProviderError::Other(
            "Anthropic streaming not yet implemented — use chat_sync".into(),
        ))
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        text.chars().filter(|c| !c.is_ascii()).count() * 2 + text.split_whitespace().count() / 4
    }

    fn health_check(&self) -> bool {
        self.do_chat(
            &[ChatMessage::user("ping")],
            &ModelConfig {
                max_tokens: 5,
                ..Default::default()
            },
        )
        .is_ok()
    }
}

// ─── Local LLM Provider（占位）───────────────────────────────────────────────

/// 本地 LLM Provider（llm crate 集成占位）
pub struct LocalLlmProvider {
    _priv: (),
}

impl LocalLlmProvider {
    pub fn new(_model_path: String) -> Self {
        Self { _priv: () }
    }
}

impl AiProvider for LocalLlmProvider {
    fn provider_name(&self) -> &'static str {
        "Local LLM"
    }
    fn available_models(&self) -> Vec<String> {
        vec!["local-model".into()]
    }
    fn chat_sync(
        &self,
        _: &[ChatMessage],
        _: &ModelConfig,
    ) -> Result<String, AiProviderError> {
        Err(AiProviderError::Other(
            "Local LLM not yet integrated — will use llm crate in next phase".into(),
        ))
    }
    fn chat_stream(
        &self,
        _: &[ChatMessage],
        _: &ModelConfig,
    ) -> Result<Box<dyn AiStream + Send>, AiProviderError> {
        Err(AiProviderError::Other("Local LLM stream not implemented".into()))
    }
    fn estimate_tokens(&self, text: &str) -> usize {
        text.chars().filter(|c| !c.is_ascii()).count() * 2 + text.split_whitespace().count() / 4
    }
    fn health_check(&self) -> bool {
        false
    }
}

// ─── 工具函数 ─────────────────────────────────────────────────────────────────

/// 从 URL 字符串提取 host 和 port
fn parse_url_host_port(url: &str) -> (String, u16) {
    let is_https = url.starts_with("https://");
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let (host_part, _path) = url.split_once('/').unwrap_or((url, ""));
    let (host, port_str) = host_part.split_once(':').unwrap_or((host_part, ""));
    let port: u16 = port_str.parse().unwrap_or(if is_https { 443 } else { 80 });
    (host.to_string(), port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::AiStream;
    use std::io::Cursor;

    #[test]
    fn test_openai_sse_stream_parse() {
        let fake = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\" World\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let cursor = Cursor::new(fake.as_bytes().to_vec());
        let mut stream = OpenAiSseStream::new(Box::new(cursor));
        let mut out = String::new();
        let mut buf = [0u8; 64];
        loop {
            let n = stream.read(&mut buf).unwrap();
            if n == 0 {
                break;
            }
            out.push_str(&String::from_utf8_lossy(&buf[..n]));
        }
        assert_eq!(out, "Hello World");
    }

    #[test]
    fn test_openai_sse_stream_collect() {
        let fake = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\ndata: [DONE]\n\n";
        let cursor = Cursor::new(fake.as_bytes().to_vec());
        let stream = Box::new(OpenAiSseStream::new(Box::new(cursor)));
        let result = stream.collect().unwrap();
        assert_eq!(result, "Hi");
    }

    #[test]
    fn test_openai_sse_skips_non_data_lines() {
        let fake = concat!(
            ": keep-alive\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"A\"}}]}\n\n",
            "event: ping\ndata: ignore\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"B\"}}]}\n\n",
            "data: [DONE]\n\n",
        );
        let cursor = Cursor::new(fake.as_bytes().to_vec());
        let stream = Box::new(OpenAiSseStream::new(Box::new(cursor)));
        let result = stream.collect().unwrap();
        assert_eq!(result, "AB");
    }

    #[test]
    fn test_parse_url_host_port() {
        let (host, port) = parse_url_host_port("https://api.openai.com/v1/chat");
        assert_eq!(host, "api.openai.com");
        assert_eq!(port, 443);

        let (host, port) = parse_url_host_port("http://localhost:8080/v1");
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);
    }

    #[test]
    fn test_model_config_default() {
        let config = ModelConfig::default();
        assert_eq!(config.model, "gpt-4o-mini");
        assert_eq!(config.max_tokens, 4096);
        assert!((config.temperature - 0.7).abs() < 0.001);
        assert!(!config.stream);
    }

    #[test]
    fn test_chat_message_constructors() {
        let sys = ChatMessage::system("you are helpful");
        assert_eq!(sys.role, Role::System);
        assert_eq!(sys.role_str(), "system");

        let user = ChatMessage::user("hello");
        assert_eq!(user.role, Role::User);
        assert_eq!(user.content, "hello");

        let asst = ChatMessage::assistant("hi there");
        assert_eq!(asst.role, Role::Assistant);
    }

    #[test]
    fn test_estimate_tokens_openai() {
        let provider = OpenAiProvider::new("test-key".into());
        // 纯英文
        let tokens = provider.estimate_tokens("hello world this is a test");
        assert_eq!(tokens, 1); // 5 words / 4 = 1
        // 中文（"你好世界测试" = 6 字符）
        let tokens = provider.estimate_tokens("你好世界测试");
        assert_eq!(tokens, 9); // 6 chars * 3/2 = 9
    }
}
