// =============================================================================
// 用户模型模块
// =============================================================================

use crate::{AuthError, AuthResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

// =============================================================================
// 用户状态
// =============================================================================

/// 用户状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// 活跃
    Active,
    /// 未激活
    Inactive,
    /// 已禁用
    Disabled,
    /// 已锁定（多次登录失败）
    Locked,
}

impl UserStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserStatus::Active => "active",
            UserStatus::Inactive => "inactive",
            UserStatus::Disabled => "disabled",
            UserStatus::Locked => "locked",
        }
    }

    pub fn can_login(&self) -> bool {
        matches!(self, UserStatus::Active)
    }
}

// =============================================================================
// 用户
// =============================================================================

/// 用户实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// 用户ID
    pub id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 用户名
    pub username: String,
    /// 邮箱
    pub email: String,
    /// 密码哈希（不序列化到响应）
    #[serde(skip_serializing)]
    pub password_hash: String,
    /// 显示名称
    pub display_name: Option<String>,
    /// 头像URL
    pub avatar_url: Option<String>,
    /// 状态
    pub status: UserStatus,
    /// 角色ID列表
    pub roles: BTreeSet<String>,
    /// 失败登录次数
    pub failed_login_count: u32,
    /// 最后登录时间
    pub last_login_at: Option<DateTime<Utc>>,
    /// 密码最后修改时间
    pub password_changed_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// 创建新用户
    pub fn new(
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        email: impl Into<String>,
        password_hash: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.into(),
            username: username.into(),
            email: email.into(),
            password_hash: password_hash.into(),
            display_name: None,
            avatar_url: None,
            status: UserStatus::Active,
            roles: BTreeSet::new(),
            failed_login_count: 0,
            last_login_at: None,
            password_changed_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    /// 添加角色
    pub fn add_role(&mut self, role_id: impl Into<String>) {
        self.roles.insert(role_id.into());
    }

    /// 移除角色
    pub fn remove_role(&mut self, role_id: &str) {
        self.roles.remove(role_id);
    }

    /// 检查是否可以登录
    pub fn can_login(&self) -> AuthResult<()> {
        if !self.status.can_login() {
            return Err(match self.status {
                UserStatus::Disabled => AuthError::UserDisabled(self.username.clone()),
                UserStatus::Locked => {
                    AuthError::AuthenticationFailed("账户已被锁定，请联系管理员".to_string())
                }
                UserStatus::Inactive => {
                    AuthError::AuthenticationFailed("账户未激活".to_string())
                }
                _ => AuthError::AuthenticationFailed("账户状态异常".to_string()),
            });
        }
        Ok(())
    }

    /// 记录登录成功
    pub fn record_login_success(&mut self) {
        self.last_login_at = Some(Utc::now());
        self.failed_login_count = 0;
        self.updated_at = Utc::now();
    }

    /// 记录登录失败
    pub fn record_login_failure(&mut self, max_attempts: u32) {
        self.failed_login_count += 1;
        if self.failed_login_count >= max_attempts {
            self.status = UserStatus::Locked;
        }
        self.updated_at = Utc::now();
    }

    /// 解锁账户
    pub fn unlock(&mut self) {
        self.status = UserStatus::Active;
        self.failed_login_count = 0;
        self.updated_at = Utc::now();
    }

    /// 修改密码
    pub fn change_password(&mut self, new_hash: impl Into<String>) {
        self.password_hash = new_hash.into();
        self.password_changed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 公共信息（不含敏感字段）
    pub fn public_info(&self) -> UserPublicInfo {
        UserPublicInfo {
            id: self.id.clone(),
            tenant_id: self.tenant_id.clone(),
            username: self.username.clone(),
            email: self.email.clone(),
            display_name: self.display_name.clone(),
            avatar_url: self.avatar_url.clone(),
            status: self.status,
            roles: self.roles.clone(),
            created_at: self.created_at,
            last_login_at: self.last_login_at,
        }
    }
}

/// 用户公共信息（不含密码哈希等敏感字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublicInfo {
    pub id: String,
    pub tenant_id: String,
    pub username: String,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub status: UserStatus,
    pub roles: BTreeSet<String>,
    pub created_at: DateTime<Utc>,
    pub last_login_at: Option<DateTime<Utc>>,
}

// =============================================================================
// 租户
// =============================================================================

/// 租户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    /// 租户ID
    pub id: String,
    /// 租户名称
    pub name: String,
    /// 租户描述
    pub description: Option<String>,
    /// 是否活跃
    pub is_active: bool,
    /// 用户配额
    pub user_quota: u32,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
}

impl Tenant {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            is_active: true,
            user_quota: 100,
            created_at: now,
            updated_at: now,
        }
    }
}

// =============================================================================
// 会话
// =============================================================================

/// 用户会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// 会话ID
    pub id: String,
    /// 用户ID
    pub user_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 刷新令牌哈希
    pub refresh_token_hash: String,
    /// 用户代理
    pub user_agent: Option<String>,
    /// IP地址
    pub ip_address: Option<String>,
    /// 过期时间
    pub expires_at: DateTime<Utc>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后使用时间
    pub last_used_at: DateTime<Utc>,
    /// 是否被撤销
    pub revoked: bool,
}

impl Session {
    pub fn new(
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        refresh_token_hash: impl Into<String>,
        ttl_seconds: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            tenant_id: tenant_id.into(),
            refresh_token_hash: refresh_token_hash.into(),
            user_agent: None,
            ip_address: None,
            expires_at: now + chrono::Duration::seconds(ttl_seconds),
            created_at: now,
            last_used_at: now,
            revoked: false,
        }
    }

    /// 是否有效
    pub fn is_valid(&self) -> bool {
        !self.revoked && Utc::now() < self.expires_at
    }

    /// 续期
    pub fn touch(&mut self) {
        self.last_used_at = Utc::now();
    }

    /// 撤销
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}

// =============================================================================
// 用户仓储 trait
// =============================================================================

/// 用户仓储 trait
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// 根据ID查找用户
    async fn find_by_id(&self, id: &str) -> AuthResult<Option<User>>;

    /// 根据用户名查找用户
    async fn find_by_username(&self, tenant_id: &str, username: &str) -> AuthResult<Option<User>>;

    /// 根据邮箱查找用户
    async fn find_by_email(&self, tenant_id: &str, email: &str) -> AuthResult<Option<User>>;

    /// 创建用户
    async fn create(&self, user: &User) -> AuthResult<User>;

    /// 更新用户
    async fn update(&self, user: &User) -> AuthResult<User>;

    /// 删除用户
    async fn delete(&self, id: &str) -> AuthResult<()>;

    /// 列出租户用户
    async fn list_by_tenant(
        &self,
        tenant_id: &str,
        page: u32,
        page_size: u32,
    ) -> AuthResult<(Vec<User>, u64)>;

    /// 验证用户名密码
    async fn authenticate(
        &self,
        tenant_id: &str,
        username: &str,
        password: &str,
    ) -> AuthResult<User>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_new() {
        let user = User::new("tenant-001", "alice", "alice@example.com", "hash123");
        assert_eq!(user.tenant_id, "tenant-001");
        assert_eq!(user.username, "alice");
        assert_eq!(user.status, UserStatus::Active);
        assert!(user.roles.is_empty());
        assert_eq!(user.failed_login_count, 0);
    }

    #[test]
    fn test_user_roles() {
        let mut user = User::new("t", "u", "e", "h");
        user.add_role("admin");
        user.add_role("user");

        assert!(user.roles.contains("admin"));
        assert!(user.roles.contains("user"));
        assert_eq!(user.roles.len(), 2);

        user.remove_role("admin");
        assert!(!user.roles.contains("admin"));
    }

    #[test]
    fn test_user_can_login() {
        let mut user = User::new("t", "u", "e", "h");
        assert!(user.can_login().is_ok());

        user.status = UserStatus::Disabled;
        assert!(matches!(user.can_login(), Err(AuthError::UserDisabled(_))));

        user.status = UserStatus::Locked;
        assert!(user.can_login().is_err());
    }

    #[test]
    fn test_login_attempts() {
        let mut user = User::new("t", "u", "e", "h");

        // 失败5次后锁定
        for _ in 0..5 {
            user.record_login_failure(5);
        }
        assert_eq!(user.status, UserStatus::Locked);
        assert_eq!(user.failed_login_count, 5);

        // 解锁
        user.unlock();
        assert_eq!(user.status, UserStatus::Active);
        assert_eq!(user.failed_login_count, 0);
    }

    #[test]
    fn test_login_success_resets_count() {
        let mut user = User::new("t", "u", "e", "h");
        user.record_login_failure(5);
        user.record_login_failure(5);
        assert_eq!(user.failed_login_count, 2);

        user.record_login_success();
        assert_eq!(user.failed_login_count, 0);
        assert!(user.last_login_at.is_some());
    }

    #[test]
    fn test_change_password() {
        let mut user = User::new("t", "u", "e", "old_hash");
        user.change_password("new_hash");
        assert_eq!(user.password_hash, "new_hash");
        assert!(user.password_changed_at.is_some());
    }

    #[test]
    fn test_public_info_no_password() {
        let user = User::new("t", "u", "e", "secret_hash");
        let public = user.public_info();

        // 公共信息不应包含密码哈希
        let json = serde_json::to_string(&public).unwrap();
        assert!(!json.contains("secret_hash"));
        assert!(!json.contains("password_hash"));
    }

    #[test]
    fn test_tenant_new() {
        let tenant = Tenant::new("Test Corp");
        assert_eq!(tenant.name, "Test Corp");
        assert!(tenant.is_active);
        assert_eq!(tenant.user_quota, 100);
    }

    #[test]
    fn test_session() {
        let session = Session::new("user-001", "tenant-001", "token_hash", 3600);
        assert!(session.is_valid());
        assert!(!session.revoked);

        let mut session = session;
        session.revoke();
        assert!(!session.is_valid());
        assert!(session.revoked);
    }

    #[test]
    fn test_user_status_can_login() {
        assert!(UserStatus::Active.can_login());
        assert!(!UserStatus::Inactive.can_login());
        assert!(!UserStatus::Disabled.can_login());
        assert!(!UserStatus::Locked.can_login());
    }
}
