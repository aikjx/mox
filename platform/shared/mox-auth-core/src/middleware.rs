// =============================================================================
// 认证中间件模块
// =============================================================================

use crate::rbac::{AccessControl, Action, Resource};
use crate::{AuthError, AuthResult, Claims, JwtManager};
use std::sync::Arc;

// =============================================================================
// 认证上下文
// =============================================================================

/// 认证上下文（请求级别的认证信息）
#[derive(Debug, Clone)]
pub struct AuthContext {
    /// 用户ID
    pub user_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 用户名
    pub username: String,
    /// 角色列表
    pub roles: Vec<String>,
    /// Token ID（用于审计）
    pub token_id: String,
    /// 是否已认证
    pub authenticated: bool,
}

impl AuthContext {
    /// 创建未认证上下文
    pub fn anonymous() -> Self {
        Self {
            user_id: String::new(),
            tenant_id: String::new(),
            username: "anonymous".to_string(),
            roles: vec![],
            token_id: String::new(),
            authenticated: false,
        }
    }

    /// 从 Claims 创建
    pub fn from_claims(claims: &Claims) -> Self {
        Self {
            user_id: claims.sub.clone(),
            tenant_id: claims.tenant_id.clone(),
            username: claims.username.clone(),
            roles: claims.roles.clone(),
            token_id: claims.jti.clone(),
            authenticated: true,
        }
    }

    /// 检查是否有指定角色
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// 检查是否是管理员
    pub fn is_admin(&self) -> bool {
        self.has_role("admin")
    }

    /// 验证租户匹配
    pub fn require_tenant(&self, tenant_id: &str) -> AuthResult<()> {
        if self.tenant_id != tenant_id {
            return Err(AuthError::TenantMismatch);
        }
        Ok(())
    }
}

// =============================================================================
// 权限要求
// =============================================================================

/// 权限要求
#[derive(Debug, Clone)]
pub struct RequirePermission {
    /// 资源
    pub resource: Resource,
    /// 操作
    pub action: Action,
}

impl RequirePermission {
    pub fn new(resource: Resource, action: Action) -> Self {
        Self { resource, action }
    }

    /// 创建读取权限要求
    pub fn read(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::new(Resource::new(domain, resource_type), Action::Read)
    }

    /// 创建写入权限要求
    pub fn write(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::new(Resource::new(domain, resource_type), Action::Create)
    }

    /// 创建删除权限要求
    pub fn delete(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::new(Resource::new(domain, resource_type), Action::Delete)
    }

    /// 创建执行权限要求
    pub fn execute(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::new(Resource::new(domain, resource_type), Action::Execute)
    }

    /// 创建管理权限要求
    pub fn admin(domain: impl Into<String>, resource_type: impl Into<String>) -> Self {
        Self::new(Resource::new(domain, resource_type), Action::Admin)
    }
}

// =============================================================================
// 认证中间件
// =============================================================================

/// 认证中间件
#[derive(Clone)]
pub struct AuthMiddleware {
    jwt_manager: Arc<JwtManager>,
    access_control: Arc<AccessControl>,
    /// 最大登录失败次数
    pub max_login_attempts: u32,
    /// 是否启用租户隔离
    pub enforce_tenant_isolation: bool,
}

impl AuthMiddleware {
    /// 创建新的认证中间件
    pub fn new(jwt_manager: JwtManager, access_control: AccessControl) -> Self {
        Self {
            jwt_manager: Arc::new(jwt_manager),
            access_control: Arc::new(access_control),
            max_login_attempts: 5,
            enforce_tenant_isolation: true,
        }
    }

    /// 从 Authorization header 认证
    pub fn authenticate_header(&self, auth_header: &str) -> AuthResult<AuthContext> {
        let token = JwtManager::extract_bearer_token(auth_header)?;
        self.authenticate_token(&token)
    }

    /// 从 Token 认证
    pub fn authenticate_token(&self, token: &str) -> AuthResult<AuthContext> {
        let claims = self.jwt_manager.verify_access_token(token)?;
        let context = AuthContext::from_claims(&claims);

        tracing::debug!(
            user_id = %context.user_id,
            username = %context.username,
            tenant_id = %context.tenant_id,
            "用户认证成功"
        );

        Ok(context)
    }

    /// 授权检查（需要认证+权限）
    pub fn authorize(
        &self,
        context: &AuthContext,
        permission: &RequirePermission,
    ) -> AuthResult<()> {
        // 必须已认证
        if !context.authenticated {
            return Err(AuthError::AuthenticationFailed(
                "用户未认证".to_string(),
            ));
        }

        // 管理员跳过权限检查
        if context.is_admin() {
            return Ok(());
        }

        // RBAC 权限检查
        self.access_control.check_access(
            &context.user_id,
            &permission.resource,
            permission.action,
        )?;

        Ok(())
    }

    /// 认证并授权（一步完成）
    pub fn authenticate_and_authorize(
        &self,
        auth_header: &str,
        permission: &RequirePermission,
    ) -> AuthResult<AuthContext> {
        let context = self.authenticate_header(auth_header)?;
        self.authorize(&context, permission)?;
        Ok(context)
    }

    /// 验证租户隔离
    pub fn check_tenant(&self, context: &AuthContext, tenant_id: &str) -> AuthResult<()> {
        if self.enforce_tenant_isolation {
            context.require_tenant(tenant_id)?;
        }
        Ok(())
    }

    /// 获取访问控制器
    pub fn access_control(&self) -> &AccessControl {
        &self.access_control
    }

    /// 获取 JWT 管理器
    pub fn jwt_manager(&self) -> &JwtManager {
        &self.jwt_manager
    }
}

impl std::fmt::Debug for AuthMiddleware {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthMiddleware")
            .field("max_login_attempts", &self.max_login_attempts)
            .field("enforce_tenant_isolation", &self.enforce_tenant_isolation)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JwtConfig;

    fn setup_middleware() -> AuthMiddleware {
        let config = JwtConfig::new("test-secret-middleware");
        let jwt = JwtManager::new(config).unwrap();
        let ac = AccessControl::new();
        AuthMiddleware::new(jwt, ac)
    }

    #[test]
    fn test_auth_context_anonymous() {
        let ctx = AuthContext::anonymous();
        assert!(!ctx.authenticated);
        assert_eq!(ctx.username, "anonymous");
        assert!(ctx.roles.is_empty());
    }

    #[test]
    fn test_auth_context_from_claims() {
        let config = JwtConfig::new("test");
        let claims = Claims::access(
            "user-001",
            "tenant-001",
            "alice",
            vec!["admin".to_string()],
            &config,
        );

        let ctx = AuthContext::from_claims(&claims);
        assert!(ctx.authenticated);
        assert_eq!(ctx.user_id, "user-001");
        assert_eq!(ctx.tenant_id, "tenant-001");
        assert_eq!(ctx.username, "alice");
        assert!(ctx.has_role("admin"));
        assert!(ctx.is_admin());
    }

    #[test]
    fn test_require_permission_helpers() {
        let read = RequirePermission::read("ai", "task");
        assert_eq!(read.action, Action::Read);
        assert_eq!(read.resource.domain, "ai");

        let write = RequirePermission::write("ai", "task");
        assert_eq!(write.action, Action::Create);

        let delete = RequirePermission::delete("ai", "task");
        assert_eq!(delete.action, Action::Delete);

        let execute = RequirePermission::execute("ai", "task");
        assert_eq!(execute.action, Action::Execute);

        let admin = RequirePermission::admin("admin", "user");
        assert_eq!(admin.action, Action::Admin);
    }

    #[test]
    fn test_authenticate_success() {
        let middleware = setup_middleware();
        let ac = middleware.access_control();
        ac.assign_role("user-001", "user").unwrap();

        let token = middleware
            .jwt_manager()
            .issue_access_token("user-001", "tenant-001", "alice", vec!["user".to_string()])
            .unwrap();

        let auth_header = format!("Bearer {}", token);
        let ctx = middleware.authenticate_header(&auth_header).unwrap();

        assert!(ctx.authenticated);
        assert_eq!(ctx.user_id, "user-001");
        assert_eq!(ctx.username, "alice");
    }

    #[test]
    fn test_authenticate_invalid_header() {
        let middleware = setup_middleware();
        assert!(middleware.authenticate_header("Invalid").is_err());
        assert!(middleware.authenticate_header("Basic abc").is_err());
        assert!(middleware.authenticate_header("Bearer invalid.token").is_err());
    }

    #[test]
    fn test_authorize_admin_bypass() {
        let middleware = setup_middleware();
        let ctx = AuthContext {
            user_id: "admin-001".to_string(),
            tenant_id: "tenant-001".to_string(),
            username: "admin".to_string(),
            roles: vec!["admin".to_string()],
            token_id: "token-001".to_string(),
            authenticated: true,
        };

        // 管理员可以访问任何资源
        let perm = RequirePermission::delete("admin", "user");
        assert!(middleware.authorize(&ctx, &perm).is_ok());
    }

    #[test]
    fn test_authorize_user_permission() {
        let middleware = setup_middleware();
        let ac = middleware.access_control();
        ac.assign_role("user-001", "user").unwrap();

        let ctx = AuthContext {
            user_id: "user-001".to_string(),
            tenant_id: "tenant-001".to_string(),
            username: "alice".to_string(),
            roles: vec!["user".to_string()],
            token_id: "token-001".to_string(),
            authenticated: true,
        };

        // 普通用户可以读取AI任务
        let read_perm = RequirePermission::read("ai", "task");
        assert!(middleware.authorize(&ctx, &read_perm).is_ok());

        // 普通用户不能删除
        let delete_perm = RequirePermission::delete("ai", "task");
        assert!(middleware.authorize(&ctx, &delete_perm).is_err());
    }

    #[test]
    fn test_authorize_unauthenticated() {
        let middleware = setup_middleware();
        let ctx = AuthContext::anonymous();
        let perm = RequirePermission::read("ai", "task");
        assert!(middleware.authorize(&ctx, &perm).is_err());
    }

    #[test]
    fn test_authenticate_and_authorize() {
        let middleware = setup_middleware();
        let ac = middleware.access_control();
        ac.assign_role("user-001", "user").unwrap();

        let token = middleware
            .jwt_manager()
            .issue_access_token("user-001", "tenant-001", "alice", vec!["user".to_string()])
            .unwrap();

        let auth_header = format!("Bearer {}", token);
        let perm = RequirePermission::read("ai", "task");

        let ctx = middleware
            .authenticate_and_authorize(&auth_header, &perm)
            .unwrap();
        assert!(ctx.authenticated);
    }

    #[test]
    fn test_tenant_isolation() {
        let middleware = setup_middleware();
        let ctx = AuthContext {
            user_id: "user-001".to_string(),
            tenant_id: "tenant-001".to_string(),
            username: "alice".to_string(),
            roles: vec![],
            token_id: "token-001".to_string(),
            authenticated: true,
        };

        assert!(middleware.check_tenant(&ctx, "tenant-001").is_ok());
        assert!(middleware.check_tenant(&ctx, "tenant-002").is_err());
    }

    #[test]
    fn test_auth_context_require_tenant() {
        let ctx = AuthContext {
            user_id: "u".to_string(),
            tenant_id: "t1".to_string(),
            username: "u".to_string(),
            roles: vec![],
            token_id: "t".to_string(),
            authenticated: true,
        };

        assert!(ctx.require_tenant("t1").is_ok());
        assert!(matches!(ctx.require_tenant("t2"), Err(AuthError::TenantMismatch)));
    }
}
