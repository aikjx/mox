//! 多租户支持
//!
//! 所有业务操作必须携带租户上下文，确保数据隔离

use crate::id::MoxId;

/// 租户上下文
#[derive(Debug, Clone)]
pub struct TenantContext {
    /// 租户 ID
    pub tenant_id: MoxId,
    /// 请求 ID（用于链路追踪）
    pub request_id: String,
    /// 当前用户 ID（可选）
    pub user_id: Option<MoxId>,
}

impl TenantContext {
    /// 创建系统级上下文（内部操作用）
    pub fn system(tenant_id: MoxId) -> Self {
        Self {
            tenant_id,
            request_id: format!("sys_{}", uuid::Uuid::new_v4().simple()),
            user_id: None,
        }
    }
}

impl Default for TenantContext {
    fn default() -> Self {
        Self {
            tenant_id: MoxId::parse("tnt_default").unwrap(),
            request_id: format!("req_{}", uuid::Uuid::new_v4().simple()),
            user_id: None,
        }
    }
}
