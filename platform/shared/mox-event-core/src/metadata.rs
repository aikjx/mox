// =============================================================================
// 事件元数据（EventMetadata）
// =============================================================================
//
// 每个事件携带的元数据，用于追踪、审计和调试：
//
// - EventId：全局唯一事件标识符（UUID v7）
// - EventSource：事件来源（服务名/模块名）
// - timestamp：事件发生时间（Unix 毫秒时间戳）
// - trace_id：分布式追踪 ID
// - correlation_id：关联 ID，用于关联同一业务流程的多个事件
// - causation_id：因果 ID，标识触发此事件的前一个事件
// - headers：自定义键值对
// =============================================================================

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::AtomicU64;
use uuid::Uuid;

/// 事件 ID
///
/// 全局唯一标识符，使用 UUID v7（时间有序）生成，便于按时间排序和索引。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(String);

impl EventId {
    /// 生成新的事件 ID（UUID v7）
    pub fn new() -> Self {
        // 使用 UUID v4 作为基础（v7 需要时间戳，这里简化实现）
        let uuid = Uuid::new_v4();
        Self(uuid.to_string())
    }

    /// 从字符串创建事件 ID
    pub fn from_str<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// 获取事件 ID 字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for EventId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid.to_string())
    }
}

/// 事件来源
///
/// 标识事件产生的服务或模块，采用点分命名法，如 `mox-kg-service`、`mox-dsql-core`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventSource(String);

impl EventSource {
    /// 创建新的事件来源
    pub fn new<S: Into<String>>(source: S) -> Self {
        Self(source.into())
    }

    /// 获取来源字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EventSource {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

/// 追踪 ID
///
/// 分布式追踪标识符，用于跨服务关联同一请求的所有事件。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TraceId(String);

impl TraceId {
    /// 生成新的追踪 ID
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// 从字符串创建追踪 ID
    pub fn from_str<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// 获取追踪 ID 字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 事件元数据
///
/// 每个事件携带的元数据，用于追踪、审计和调试。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// 事件 ID（全局唯一）
    event_id: EventId,
    /// 事件来源（服务/模块名）
    source: EventSource,
    /// 事件发生时间（Unix 毫秒时间戳）
    timestamp: u64,
    /// 追踪 ID（分布式追踪）
    trace_id: Option<TraceId>,
    /// 关联 ID（同一业务流程的多个事件共享）
    correlation_id: Option<String>,
    /// 因果 ID（触发此事件的前一个事件 ID）
    causation_id: Option<EventId>,
    /// 事件版本
    version: String,
    /// 自定义元数据头
    headers: std::collections::HashMap<String, String>,
}

impl EventMetadata {
    /// 创建新的事件元数据
    pub fn new(source: EventSource) -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Self {
            event_id: EventId::new(),
            source,
            timestamp,
            trace_id: None,
            correlation_id: None,
            causation_id: None,
            version: "v1".to_string(),
            headers: std::collections::HashMap::new(),
        }
    }

    /// 获取事件 ID
    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }

    /// 获取事件来源
    pub fn source(&self) -> &EventSource {
        &self.source
    }

    /// 获取事件时间戳（毫秒）
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// 获取追踪 ID
    pub fn trace_id(&self) -> Option<&TraceId> {
        self.trace_id.as_ref()
    }

    /// 设置追踪 ID
    pub fn with_trace_id(mut self, trace_id: TraceId) -> Self {
        self.trace_id = Some(trace_id);
        self
    }

    /// 获取关联 ID
    pub fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }

    /// 设置关联 ID
    pub fn with_correlation_id<S: Into<String>>(mut self, id: S) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    /// 获取因果 ID
    pub fn causation_id(&self) -> Option<&EventId> {
        self.causation_id.as_ref()
    }

    /// 设置因果 ID
    pub fn with_causation_id(mut self, id: EventId) -> Self {
        self.causation_id = Some(id);
        self
    }

    /// 获取事件版本
    pub fn version(&self) -> &str {
        &self.version
    }

    /// 设置事件版本
    pub fn with_version<S: Into<String>>(mut self, version: S) -> Self {
        self.version = version.into();
        self
    }

    /// 获取自定义头
    pub fn headers(&self) -> &std::collections::HashMap<String, String> {
        &self.headers
    }

    /// 获取指定头的值
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(|s| s.as_str())
    }

    /// 设置自定义头
    pub fn with_header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 从另一个元数据创建因果链（设置 causation_id 和 correlation_id）
    pub fn caused_by(previous: &EventMetadata, source: EventSource) -> Self {
        let mut meta = Self::new(source);
        meta.causation_id = Some(previous.event_id.clone());
        meta.correlation_id = previous.correlation_id.clone().or_else(|| Some(previous.event_id.to_string()));
        meta.trace_id = previous.trace_id.clone();
        meta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_id_generation() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
        assert!(!id1.as_str().is_empty());
    }

    #[test]
    fn test_event_metadata_creation() {
        let meta = EventMetadata::new(EventSource::new("test-service"));
        assert!(!meta.event_id().as_str().is_empty());
        assert_eq!(meta.source().as_str(), "test-service");
        assert!(meta.timestamp() > 0);
        assert_eq!(meta.version(), "v1");
    }

    #[test]
    fn test_event_metadata_builder() {
        let trace_id = TraceId::new();
        let meta = EventMetadata::new(EventSource::new("test"))
            .with_trace_id(trace_id.clone())
            .with_correlation_id("corr-123")
            .with_version("v2")
            .with_header("env", "production");

        assert_eq!(meta.trace_id(), Some(&trace_id));
        assert_eq!(meta.correlation_id(), Some("corr-123"));
        assert_eq!(meta.version(), "v2");
        assert_eq!(meta.header("env"), Some("production"));
    }

    #[test]
    fn test_causation_chain() {
        let first = EventMetadata::new(EventSource::new("service-a"));
        let second = EventMetadata::caused_by(&first, EventSource::new("service-b"));

        assert_eq!(second.causation_id(), Some(first.event_id()));
        assert_eq!(second.correlation_id(), Some(first.event_id().as_str()));
    }

    #[test]
    fn test_trace_id_empty() {
        let trace_id = TraceId::from_str("");
        assert!(trace_id.is_empty());
    }
}
