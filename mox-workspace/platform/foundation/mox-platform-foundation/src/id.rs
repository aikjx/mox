//! 全局唯一 ID 生成器
//!
//! 格式：前缀 + UUID v4（去除横线，26字符）
//! 示例：usr_550e8400e29b41d4a716446655440000

use uuid::Uuid;

/// Mox 全局唯一 ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct MoxId(String);

impl MoxId {
    /// 生成新 ID
    pub fn new(prefix: &str) -> Self {
        let id = format!("{}_{}", prefix, Uuid::new_v4().simple());
        Self(id)
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() { None } else { Some(Self(s.to_string())) }
    }

    /// 获取前缀
    pub fn prefix(&self) -> &str {
        self.0.split_once('_').map(|(p, _)| p).unwrap_or("")
    }

    /// 转为字符串
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for MoxId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<MoxId> for String {
    fn from(id: MoxId) -> Self { id.0 }
}

impl AsRef<str> for MoxId {
    fn as_ref(&self) -> &str { &self.0 }
}

/// 租户 ID
pub type TenantId = MoxId;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_id_has_prefix() {
        let id = MoxId::new("usr");
        assert!(id.as_str().starts_with("usr_"));
    }

    #[test]
    fn test_id_unique() {
        let id1 = MoxId::new("tst");
        let id2 = MoxId::new("tst");
        assert_ne!(id1, id2);
    }
}
