// =============================================================================
// MOX 统一认证授权核心（mox-auth-core）
// =============================================================================
//
// 企业级安全基础设施，提供：
//
// 1. **Token 认证**（jwt）— Token 签发/验证/刷新，HMAC-SHA256 签名
// 2. **RBAC 权限**（rbac）— 角色/权限/资源模型，细粒度访问控制
// 3. **用户模型**（user）— 用户/租户/会话管理
// 4. **密码哈希**（password）— PBKDF2 安全哈希
// 5. **认证中间件**（middleware）— 请求认证/权限校验
//
// 设计原则：
// - 安全优先：遵循 OWASP 安全规范
// - 无状态：Token 无状态认证，便于水平扩展
// - 细粒度：RBAC 支持角色+权限+资源+操作四维控制
// - 可审计：所有认证授权操作有审计日志
// - 轻量依赖：仅使用标准库 + serde + chrono + uuid
// =============================================================================

pub mod jwt;
pub mod rbac;
pub mod user;
pub mod password;
pub mod middleware;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use jwt::{JwtManager, JwtConfig, TokenType, Claims, RefreshToken};
pub use rbac::{Role, Permission, Resource, Action, AccessControl, Policy, PolicyEffect};
pub use user::{User, UserStatus, Tenant, Session, UserRepository};
pub use password::{PasswordManager, HashedPassword};
pub use middleware::{AuthMiddleware, AuthContext, RequirePermission};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-auth-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use serde::{Deserialize, Serialize};

/// 认证错误
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Token 无效: {0}")]
    InvalidToken(String),
    #[error("Token 已过期")]
    TokenExpired,
    #[error("Token 类型错误: 期望 {expected}, 实际 {actual}")]
    TokenTypeMismatch { expected: String, actual: String },
    #[error("认证失败: {0}")]
    AuthenticationFailed(String),
    #[error("权限不足: 需要 {required}, 实际 {actual}")]
    PermissionDenied { required: String, actual: String },
    #[error("用户不存在: {0}")]
    UserNotFound(String),
    #[error("用户已禁用: {0}")]
    UserDisabled(String),
    #[error("密码错误")]
    WrongPassword,
    #[error("会话已失效")]
    SessionInvalid,
    #[error("租户隔离冲突")]
    TenantMismatch,
    #[error("配置错误: {0}")]
    ConfigError(String),
    #[error("内部错误: {0}")]
    InternalError(String),
}

/// 认证结果类型
pub type AuthResult<T> = Result<T, AuthError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_display() {
        let err = AuthError::TokenExpired;
        assert!(format!("{}", err).contains("已过期"));
    }
}
