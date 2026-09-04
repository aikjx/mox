// =============================================================================
// 追踪 ID 规范（trace_id / span_id / 全链路透传）
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的 trace_id 格式（UUID v4）。
// =============================================================================

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use uuid::Uuid;

// =============================================================================
// TraceId（值对象）
// =============================================================================

/// 追踪 ID（UUID v4）
///
/// 全链路唯一标识，从前端生成，透传到所有后端服务和 LLM 调用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(Uuid);

impl TraceId {
    /// 生成新的追踪 ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    /// 获取 UUID
    pub fn uuid(&self) -> Uuid {
        self.0
    }

    /// 字符串形式
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }

    /// 短形式（前8位，用于日志显示）
    pub fn short(&self) -> String {
        self.0.to_string().chars().take(8).collect()
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TraceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for TraceId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<TraceId> for Uuid {
    fn from(trace_id: TraceId) -> Self {
        trace_id.0
    }
}

// =============================================================================
// SpanId（值对象）
// =============================================================================

/// Span ID（8字节，用于分布式追踪的子跨度）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl SpanId {
    /// 生成新的 Span ID
    pub fn new() -> Self {
        Self(rand::random())
    }

    /// 从 u64 创建
    pub fn from_u64(value: u64) -> Self {
        Self(value)
    }

    /// 获取 u64 值
    pub fn value(&self) -> u64 {
        self.0
    }

    /// 十六进制字符串
    pub fn as_hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

impl Default for SpanId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SpanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_hex())
    }
}

// =============================================================================
// TraceContext（追踪上下文）
// =============================================================================

/// 追踪上下文（全链路透传）
///
/// 包含 trace_id 和当前 span_id，支持父子 span 关系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// 追踪 ID（全链路唯一）
    pub trace_id: TraceId,
    /// 当前 Span ID
    pub span_id: SpanId,
    /// 父 Span ID（根 span 为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<SpanId>,
    /// 服务名
    pub service: String,
    /// 操作名
    pub operation: String,
    /// 开始时间戳（毫秒）
    pub start_time_ms: u64,
    /// 采样标记
    pub sampled: bool,
}

impl TraceContext {
    /// 创建新的根上下文
    pub fn new_root(service: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            service: service.into(),
            operation: operation.into(),
            start_time_ms: now_ms(),
            sampled: true,
        }
    }

    /// 创建子上下文
    pub fn child(&self, operation: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: SpanId::new(),
            parent_span_id: Some(self.span_id),
            service: self.service.clone(),
            operation: operation.into(),
            start_time_ms: now_ms(),
            sampled: self.sampled,
        }
    }

    /// 转换为 HTTP 头（用于跨服务透传）
    pub fn to_headers(&self) -> std::collections::HashMap<String, String> {
        let mut headers = std::collections::HashMap::new();
        headers.insert("X-Trace-Id".to_string(), self.trace_id.as_string());
        headers.insert("X-Span-Id".to_string(), self.span_id.as_hex());
        if let Some(parent) = self.parent_span_id {
            headers.insert("X-Parent-Span-Id".to_string(), parent.as_hex());
        }
        headers
    }

    /// 从 HTTP 头解析
    pub fn from_headers(headers: &std::collections::HashMap<String, String>, service: impl Into<String>, operation: impl Into<String>) -> Self {
        // 大小写不敏感查找
        let get_header = |key: &str| -> Option<&String> {
            headers.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v)
        };

        let trace_id = get_header("x-trace-id")
            .and_then(|v| TraceId::parse(v))
            .unwrap_or_default();
        let parent_span_id = get_header("x-span-id")
            .and_then(|v| u64::from_str_radix(v, 16).ok())
            .map(SpanId::from_u64);

        Self {
            trace_id,
            span_id: SpanId::new(),
            parent_span_id,
            service: service.into(),
            operation: operation.into(),
            start_time_ms: now_ms(),
            sampled: true,
        }
    }
}

// =============================================================================
// 全局追踪上下文（线程局部存储）
// =============================================================================

static GLOBAL_TRACE_ID: OnceLock<TraceId> = OnceLock::new();

/// 获取当前全局追踪 ID（如果已设置）
pub fn current_trace_id() -> Option<TraceId> {
    GLOBAL_TRACE_ID.get().copied()
}

/// 设置全局追踪 ID（仅在服务启动时调用一次）
pub fn set_global_trace_id(trace_id: TraceId) {
    let _ = GLOBAL_TRACE_ID.set(trace_id);
}

/// 在追踪上下文中执行闭包
pub fn with_trace_context<F, R>(_context: &TraceContext, f: F) -> R
where
    F: FnOnce() -> R,
{
    // 实际实现中应该使用 tracing::span，这里简化为直接执行
    f()
}

// =============================================================================
// 工具函数
// =============================================================================

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_id_new_and_parse() {
        let id = TraceId::new();
        let s = id.as_string();
        let parsed = TraceId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn trace_id_short() {
        let id = TraceId::new();
        assert_eq!(id.short().len(), 8);
    }

    #[test]
    fn span_id_hex() {
        let id = SpanId::from_u64(255);
        assert_eq!(id.as_hex(), "00000000000000ff");
    }

    #[test]
    fn trace_context_root_and_child() {
        let root = TraceContext::new_root("gateway", "request");
        assert!(root.parent_span_id.is_none());

        let child = root.child("database_query");
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_span_id, Some(root.span_id));
        assert_ne!(child.span_id, root.span_id);
    }

    #[test]
    fn trace_context_headers_roundtrip() {
        let context = TraceContext::new_root("svc", "op");
        let headers = context.to_headers();
        let parsed = TraceContext::from_headers(&headers, "svc2", "op2");
        assert_eq!(parsed.trace_id, context.trace_id);
        assert_eq!(parsed.parent_span_id, Some(context.span_id));
    }

    #[test]
    fn trace_id_serialization() {
        let id = TraceId::new();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: TraceId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
