//! 分布式追踪 — OpenTelemetry，零配置自动注入trace_id

use tracing::Span;
use uuid::Uuid;

/// 追踪上下文（贯穿整个请求链路）
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
}

impl TraceContext {
    /// 创建新的追踪上下文（新trace）
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string().replace('-', ""),
            span_id: Uuid::new_v4().to_string().replace('-', "")[..16].to_string(),
            parent_span_id: None,
            service_name: service_name.into(),
        }
    }

    /// 从上游请求头恢复追踪上下文
    pub fn from_headers(headers: &axum::http::HeaderMap, service_name: impl Into<String>) -> Self {
        let trace_id = headers
            .get("x-trace-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string().replace('-', ""));
        let parent_span_id = headers
            .get("x-span-id")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        Self {
            trace_id,
            span_id: Uuid::new_v4().to_string().replace('-', "")[..16].to_string(),
            parent_span_id,
            service_name: service_name.into(),
        }
    }

    /// 注入到下游请求头
    pub fn inject_headers(&self, headers: &mut reqwest::header::HeaderMap) {
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&self.trace_id) {
            headers.insert("x-trace-id", v);
        }
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&self.span_id) {
            headers.insert("x-span-id", v);
        }
    }

    /// 创建子span
    pub fn child_span(&self, name: &str) -> Span {
        tracing::info_span!(
            name,
            trace_id = %self.trace_id,
            span_id = %self.span_id,
            parent_span_id = ?self.parent_span_id,
            service = %self.service_name,
        )
    }
}
