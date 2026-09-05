// =============================================================================
// 分布式追踪工具（TracingUtils）
// =============================================================================
//
// 提供跨服务 trace_id 传播能力：
// - 从 HTTP 请求头提取 trace_id（X-Trace-Id / traceparent）
// - 生成新的 trace_id（UUIDv4）
// - 在响应头中注入 trace_id
// - 与 tracing crate 的 span 集成
//
// 设计原则：
// - 无外部依赖（不依赖 opentelemetry），轻量级实现
// - 兼容 W3C Trace Context（traceparent 头）
// - 与现有 tracing 系统无缝集成
// =============================================================================

use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Response,
};
use uuid::Uuid;

/// Trace ID 头名称
pub const TRACE_ID_HEADER: &str = "x-trace-id";
/// W3C Trace Context 头名称
pub const TRACEPARENT_HEADER: &str = "traceparent";

/// 从请求头提取 trace_id
///
/// 优先级：x-trace-id > traceparent（W3C）> 生成新 ID
pub fn extract_trace_id(headers: &HeaderMap) -> String {
    // 1. 优先从 x-trace-id 提取
    if let Some(value) = headers.get(TRACE_ID_HEADER) {
        if let Ok(id) = value.to_str() {
            if !id.is_empty() {
                return id.to_string();
            }
        }
    }

    // 2. 从 W3C traceparent 提取（格式：version-traceid-parentid-flags）
    if let Some(value) = headers.get(TRACEPARENT_HEADER) {
        if let Ok(tp) = value.to_str() {
            let parts: Vec<&str> = tp.split('-').collect();
            if parts.len() >= 2 && parts[1].len() == 32 {
                return parts[1].to_string();
            }
        }
    }

    // 3. 生成新的 trace_id
    generate_trace_id()
}

/// 生成新的 trace_id（UUIDv4，无连字符，32 字符十六进制）
pub fn generate_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

/// 在响应头中注入 trace_id
pub fn inject_trace_id<B>(response: &mut Response<B>, trace_id: &str) {
    if let Ok(name) = HeaderName::from_bytes(TRACE_ID_HEADER.as_bytes()) {
        if let Ok(value) = HeaderValue::from_str(trace_id) {
            response.headers_mut().insert(name, value);
        }
    }
}

/// 创建带 trace_id 的 tracing span（固定名称 "request"）
///
/// 使用方式：
/// ```ignore
/// let trace_id = extract_trace_id(&headers);
/// let span = make_request_span(&trace_id);
/// let _guard = span.enter();
/// ```
pub fn make_request_span(trace_id: &str) -> tracing::Span {
    tracing::info_span!("request", trace_id = %trace_id)
}

/// 在当前 span 中记录 trace_id 字段
pub fn record_trace_id(trace_id: &str) {
    tracing::Span::current().record("trace_id", tracing::field::display(trace_id));
}

/// 从当前 tracing span 中提取 trace_id（如果存在）
pub fn current_trace_id() -> Option<String> {
    // tracing crate 不直接支持从 span 中提取字段，
    // 这里返回 None，实际使用时应通过请求头传递
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_trace_id() {
        let id = generate_trace_id();
        assert_eq!(id.len(), 32);
        assert!(id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_extract_from_x_trace_id() {
        let mut headers = HeaderMap::new();
        headers.insert(TRACE_ID_HEADER, HeaderValue::from_static("abc123"));
        let id = extract_trace_id(&headers);
        assert_eq!(id, "abc123");
    }

    #[test]
    fn test_extract_from_traceparent() {
        let mut headers = HeaderMap::new();
        // W3C 格式：version-traceid-parentid-flags
        headers.insert(
            TRACEPARENT_HEADER,
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );
        let id = extract_trace_id(&headers);
        assert_eq!(id, "0af7651916cd43dd8448eb211c80319c");
    }

    #[test]
    fn test_extract_no_header_generates_new() {
        let headers = HeaderMap::new();
        let id = extract_trace_id(&headers);
        assert_eq!(id.len(), 32);
    }

    #[test]
    fn test_extract_invalid_traceparent_generates_new() {
        let mut headers = HeaderMap::new();
        headers.insert(TRACEPARENT_HEADER, HeaderValue::from_static("invalid"));
        let id = extract_trace_id(&headers);
        assert_eq!(id.len(), 32);
    }

    #[test]
    fn test_inject_trace_id() {
        let mut response = Response::new(());
        inject_trace_id(&mut response, "test-trace-id");
        let value = response.headers().get(TRACE_ID_HEADER).unwrap();
        assert_eq!(value, "test-trace-id");
    }

    #[test]
    fn test_x_trace_id_priority_over_traceparent() {
        let mut headers = HeaderMap::new();
        headers.insert(TRACE_ID_HEADER, HeaderValue::from_static("priority-id"));
        headers.insert(
            TRACEPARENT_HEADER,
            HeaderValue::from_static("00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"),
        );
        let id = extract_trace_id(&headers);
        assert_eq!(id, "priority-id");
    }
}
