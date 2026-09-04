// =============================================================================
// 分布式追踪模块
// =============================================================================

use crate::{ObservabilityError, ObservabilityResult};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use uuid::Uuid;

// =============================================================================
// TraceId / SpanId
// =============================================================================

/// 追踪 ID（128位，UUID v4）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(Uuid);

impl TraceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    pub fn as_string(&self) -> String {
        self.0.to_string()
    }

    pub fn parse(s: &str) -> ObservabilityResult<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| ObservabilityError::TraceContextError(format!("TraceId 解析失败: {}", e)))
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

/// Span ID（64位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpanId(u64);

impl SpanId {
    pub fn new() -> Self {
        use rand::Rng;
        Self(rand::thread_rng().gen())
    }

    pub fn from_u64(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    pub fn as_string(&self) -> String {
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
        write!(f, "{:016x}", self.0)
    }
}

// =============================================================================
// 追踪状态
// =============================================================================

/// 追踪状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceState {
    /// 默认（采样决策未做）
    Unspecified,
    /// 已采样（记录）
    Sampled,
    /// 未采样（不记录）
    NotSampled,
    /// 强制采样（即使采样率为0也记录）
    ForceSampled,
}

impl TraceState {
    pub fn is_sampled(&self) -> bool {
        matches!(self, TraceState::Sampled | TraceState::ForceSampled)
    }
}

// =============================================================================
// 追踪上下文
// =============================================================================

/// 追踪上下文（跨服务传播）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// 追踪 ID
    pub trace_id: TraceId,
    /// 当前 Span ID
    pub span_id: SpanId,
    /// 父 Span ID（根 span 为 None）
    pub parent_span_id: Option<SpanId>,
    /// 追踪状态
    pub state: TraceState,
    /// 服务名称
    pub service_name: String,
    /// 附加属性
    pub attributes: std::collections::BTreeMap<String, String>,
}

impl TraceContext {
    /// 创建新的根追踪上下文
    pub fn new_root(service_name: impl Into<String>) -> Self {
        Self {
            trace_id: TraceId::new(),
            span_id: SpanId::new(),
            parent_span_id: None,
            state: TraceState::Sampled,
            service_name: service_name.into(),
            attributes: std::collections::BTreeMap::new(),
        }
    }

    /// 从父上下文创建子 Span
    pub fn child(&self, name: impl Into<String>) -> Span {
        Span::new(
            name,
            self.trace_id,
            Some(self.span_id),
            self.service_name.clone(),
        )
    }

    /// 创建子上下文
    pub fn child_context(&self) -> Self {
        Self {
            trace_id: self.trace_id,
            span_id: SpanId::new(),
            parent_span_id: Some(self.span_id),
            state: self.state,
            service_name: self.service_name.clone(),
            attributes: self.attributes.clone(),
        }
    }

    /// 添加属性
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 序列化为 W3C TraceContext 格式（traceparent header）
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id.as_string().replace('-', ""),
            self.span_id.as_string(),
            if self.state.is_sampled() { 0x01 } else { 0x00 }
        )
    }

    /// 从 W3C TraceContext 格式解析
    pub fn from_traceparent(header: &str, service_name: impl Into<String>) -> ObservabilityResult<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return Err(ObservabilityError::TraceContextError(
                "traceparent 格式错误".to_string(),
            ));
        }

        let trace_id_hex = parts[1];
        let span_id_hex = parts[2];
        let flags = u8::from_str_radix(parts[3], 16)
            .map_err(|e| ObservabilityError::TraceContextError(format!("flags 解析失败: {}", e)))?;

        // 32位十六进制 → UUID
        let trace_id_uuid = Uuid::parse_str(&format!(
            "{}-{}-{}-{}-{}",
            &trace_id_hex[0..8],
            &trace_id_hex[8..12],
            &trace_id_hex[12..16],
            &trace_id_hex[16..20],
            &trace_id_hex[20..32]
        ))
        .map_err(|e| ObservabilityError::TraceContextError(format!("trace_id 解析失败: {}", e)))?;

        let span_id_u64 = u64::from_str_radix(span_id_hex, 16)
            .map_err(|e| ObservabilityError::TraceContextError(format!("span_id 解析失败: {}", e)))?;

        Ok(Self {
            trace_id: TraceId::from_uuid(trace_id_uuid),
            span_id: SpanId::from_u64(span_id_u64),
            parent_span_id: None,
            state: if flags & 0x01 != 0 {
                TraceState::Sampled
            } else {
                TraceState::NotSampled
            },
            service_name: service_name.into(),
            attributes: std::collections::BTreeMap::new(),
        })
    }
}

// =============================================================================
// Span
// =============================================================================

/// 追踪 Span（一个操作的时间范围）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Span 名称
    pub name: String,
    /// 追踪 ID
    pub trace_id: TraceId,
    /// Span ID
    pub span_id: SpanId,
    /// 父 Span ID
    pub parent_span_id: Option<SpanId>,
    /// 服务名称
    pub service_name: String,
    /// 开始时间
    pub start_time: chrono::DateTime<chrono::Utc>,
    /// 持续时间（微秒）
    pub duration_us: Option<u64>,
    /// 状态（OK / Error）
    pub status: SpanStatus,
    /// 属性
    pub attributes: std::collections::BTreeMap<String, String>,
    /// 事件
    pub events: Vec<SpanEvent>,
}

/// Span 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpanStatus {
    Unset,
    Ok,
    Error,
}

/// Span 事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    pub name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attributes: std::collections::BTreeMap<String, String>,
}

impl Span {
    pub fn new(
        name: impl Into<String>,
        trace_id: TraceId,
        parent_span_id: Option<SpanId>,
        service_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            trace_id,
            span_id: SpanId::new(),
            parent_span_id,
            service_name: service_name.into(),
            start_time: chrono::Utc::now(),
            duration_us: None,
            status: SpanStatus::Unset,
            attributes: std::collections::BTreeMap::new(),
            events: vec![],
        }
    }

    /// 添加属性
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// 添加事件
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: chrono::Utc::now(),
            attributes: std::collections::BTreeMap::new(),
        });
    }

    /// 设置为成功
    pub fn set_ok(&mut self) {
        self.status = SpanStatus::Ok;
    }

    /// 设置为错误
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status = SpanStatus::Error;
        self.set_attribute("error.message", message);
    }

    /// 结束 Span（记录持续时间）
    pub fn end(&mut self) {
        let duration = chrono::Utc::now() - self.start_time;
        self.duration_us = Some(duration.num_microseconds().unwrap_or(0) as u64);
        if self.status == SpanStatus::Unset {
            self.status = SpanStatus::Ok;
        }
    }

    /// 创建子 Span
    pub fn child(&self, name: impl Into<String>) -> Span {
        Span::new(
            name,
            self.trace_id,
            Some(self.span_id),
            self.service_name.clone(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_new_and_parse() {
        let id = TraceId::new();
        let s = id.as_string();
        let parsed = TraceId::parse(&s).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_span_id_format() {
        let id = SpanId::from_u64(42);
        assert_eq!(id.as_string(), "000000000000002a");
        assert_eq!(id.as_u64(), 42);
    }

    #[test]
    fn test_trace_context_root() {
        let ctx = TraceContext::new_root("test-service");
        assert!(ctx.parent_span_id.is_none());
        assert_eq!(ctx.service_name, "test-service");
        assert!(ctx.state.is_sampled());
    }

    #[test]
    fn test_trace_context_child() {
        let parent = TraceContext::new_root("parent");
        let child = parent.child_context();
        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn test_traceparent_roundtrip() {
        let ctx = TraceContext::new_root("test-service");
        let header = ctx.to_traceparent();
        assert!(header.starts_with("00-"));

        let parsed = TraceContext::from_traceparent(&header, "test-service").unwrap();
        assert_eq!(parsed.trace_id, ctx.trace_id);
        assert!(parsed.state.is_sampled());
    }

    #[test]
    fn test_span_lifecycle() {
        let mut span = Span::new("test-operation", TraceId::new(), None, "test-service");
        span.set_attribute("key", "value");
        span.add_event("started");

        std::thread::sleep(Duration::from_millis(10));
        span.set_ok();
        span.end();

        assert!(span.duration_us.is_some());
        assert!(span.duration_us.unwrap() >= 10_000); // 至少10ms
        assert_eq!(span.status, SpanStatus::Ok);
        assert_eq!(span.attributes.get("key").unwrap(), "value");
        assert_eq!(span.events.len(), 1);
    }

    #[test]
    fn test_span_error() {
        let mut span = Span::new("failing-operation", TraceId::new(), None, "test-service");
        span.set_error("something went wrong");
        span.end();

        assert_eq!(span.status, SpanStatus::Error);
        assert_eq!(span.attributes.get("error.message").unwrap(), "something went wrong");
    }

    #[test]
    fn test_span_child() {
        let parent = Span::new("parent", TraceId::new(), None, "test-service");
        let child = parent.child("child");

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent_span_id, Some(parent.span_id));
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn test_trace_state() {
        assert!(TraceState::Sampled.is_sampled());
        assert!(TraceState::ForceSampled.is_sampled());
        assert!(!TraceState::NotSampled.is_sampled());
        assert!(!TraceState::Unspecified.is_sampled());
    }
}
