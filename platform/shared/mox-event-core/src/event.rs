// =============================================================================
// 事件模型（Event）
// =============================================================================
//
// 所有事件的基础接口，参考领域驱动设计（DDD）中的领域事件模式：
//
// - Event trait：所有事件必须实现，提供事件类型和序列化能力
// - EventType：事件类型标识符，用于订阅过滤
// - EventPayload：事件载荷，支持任意可序列化类型
// =============================================================================

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt;

/// 事件类型标识符
///
/// 采用点分命名法，如 `user.created`、`order.paid`、`system.alert`
/// 支持通配符订阅：`user.*` 匹配所有 user 开头的事件
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventType(String);

impl EventType {
    /// 创建新的事件类型
    pub fn new<S: Into<String>>(event_type: S) -> Self {
        Self(event_type.into())
    }

    /// 获取事件类型字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 检查当前事件类型是否匹配目标类型（支持通配符）
    ///
    /// # 示例
    /// ```
    /// use mox_event_core::EventType;
    ///
    /// let specific = EventType::new("user.created");
    /// let wildcard = EventType::new("user.*");
    /// assert!(wildcard.matches(&specific));
    /// assert!(!specific.matches(&wildcard));
    /// ```
    pub fn matches(&self, other: &EventType) -> bool {
        if self.0 == other.0 {
            return true;
        }
        // 通配符匹配：self 是模式，other 是具体事件
        if self.0.ends_with(".*") {
            let prefix = &self.0[..self.0.len() - 2];
            return other.0.starts_with(prefix) && other.0.len() > prefix.len();
        }
        false
    }

    /// 获取事件的域（第一段）
    pub fn domain(&self) -> &str {
        self.0.split('.').next().unwrap_or("")
    }

    /// 获取事件的动作（最后一段）
    pub fn action(&self) -> &str {
        self.0.split('.').last().unwrap_or("")
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EventType {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for EventType {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// 事件载荷
///
/// 支持任意可序列化类型作为事件载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EventPayload {
    /// JSON 对象载荷
    Json(serde_json::Value),
    /// 原始字节载荷
    Bytes(Vec<u8>),
    /// 空载荷
    Empty,
}

impl EventPayload {
    /// 从可序列化类型创建 JSON 载荷
    pub fn from_json<T: Serialize>(value: &T) -> Result<Self, serde_json::Error> {
        Ok(Self::Json(serde_json::to_value(value)?))
    }

    /// 从字节创建载荷
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self::Bytes(bytes)
    }

    /// 创建空载荷
    pub fn empty() -> Self {
        Self::Empty
    }

    /// 尝试解析为指定类型
    pub fn parse<T: DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        match self {
            Self::Json(v) => serde_json::from_value(v.clone()),
            Self::Bytes(b) => serde_json::from_slice(b),
            Self::Empty => Err(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "empty payload",
            ))),
        }
    }

    /// 获取 JSON 引用
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Self::Json(v) => Some(v),
            _ => None,
        }
    }

    /// 获取字节引用
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Empty)
    }
}

impl Default for EventPayload {
    fn default() -> Self {
        Self::Empty
    }
}

/// 事件 trait
///
/// 所有领域事件必须实现此 trait，提供事件类型标识和序列化能力。
///
/// # 示例
/// ```
/// use mox_event_core::{Event, EventType};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct UserCreatedEvent {
///     user_id: String,
///     username: String,
/// }
///
/// impl Event for UserCreatedEvent {
///     fn event_type(&self) -> EventType {
///         EventType::new("user.created")
///     }
/// }
/// ```
pub trait Event: Send + Sync + Serialize + DeserializeOwned + Clone + 'static {
    /// 获取事件类型
    fn event_type(&self) -> EventType;

    /// 获取事件版本（用于事件演进，默认 v1）
    fn event_version(&self) -> &str {
        "v1"
    }

    /// 序列化为事件载荷
    fn to_payload(&self) -> Result<EventPayload, serde_json::Error> {
        EventPayload::from_json(self)
    }

    /// 从事件载荷反序列化
    fn from_payload(payload: &EventPayload) -> Result<Self, serde_json::Error> {
        payload.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestEvent {
        id: String,
        value: i32,
    }

    impl Event for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::new("test.event")
        }
    }

    #[test]
    fn test_event_type_matching() {
        let specific = EventType::new("user.created");
        let wildcard = EventType::new("user.*");

        assert!(wildcard.matches(&specific));
        assert!(!specific.matches(&wildcard));
        assert!(specific.matches(&specific));
    }

    #[test]
    fn test_event_type_domain_action() {
        let evt = EventType::new("order.paid.completed");
        assert_eq!(evt.domain(), "order");
        assert_eq!(evt.action(), "completed");
    }

    #[test]
    fn test_event_payload_json() {
        let evt = TestEvent {
            id: "123".to_string(),
            value: 42,
        };
        let payload = evt.to_payload().unwrap();
        assert!(payload.as_json().is_some());

        let parsed = TestEvent::from_payload(&payload).unwrap();
        assert_eq!(parsed.id, "123");
        assert_eq!(parsed.value, 42);
    }

    #[test]
    fn test_event_payload_bytes() {
        let payload = EventPayload::from_bytes(vec![1, 2, 3]);
        assert_eq!(payload.as_bytes(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn test_event_payload_empty() {
        let payload = EventPayload::empty();
        assert!(payload.is_empty());
        assert!(payload.as_json().is_none());
    }
}
