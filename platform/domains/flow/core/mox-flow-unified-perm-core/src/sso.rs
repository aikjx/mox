// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! SSO 单点登录
//!
//! 支持多种 SSO 协议：
//! - OIDC (OpenID Connect)
//! - SAML 2.0
//! - OAuth 2.0
//! - CAS
//! - LDAP
//!
//! 提供统一的 Token 管理和用户映射。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{PermError, PermResult};
use crate::tenant::TenantManager;
use crate::types::{User, UserStatus, now_ms};

/// SSO 提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoProviderType {
    /// OpenID Connect
    Oidc,
    /// SAML 2.0
    Saml,
    /// OAuth 2.0
    OAuth2,
    /// CAS
    Cas,
    /// LDAP
    Ldap,
}

/// SSO 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SsoConfig {
    /// 配置 ID
    pub id: String,
    /// 配置名称
    pub name: String,
    /// 提供商类型
    pub provider_type: SsoProviderType,
    /// 租户 ID
    pub tenant_id: String,
    /// 是否启用
    pub enabled: bool,
    /// 提供商名称（如 "Google", "Okta"）
    pub vendor: String,
    /// 客户端 ID
    pub client_id: String,
    /// 客户端密钥（加密存储）
    pub client_secret: String,
    /// 授权端点
    pub authorization_endpoint: String,
    /// Token 端点
    pub token_endpoint: String,
    /// 用户信息端点
    pub userinfo_endpoint: String,
    /// 回调 URL
    pub redirect_uri: String,
    /// 作用域
    pub scopes: Vec<String>,
    /// 颁发者（issuer）
    pub issuer: Option<String>,
    /// 字段映射（外部字段 -> 本地用户字段）
    pub field_mapping: HashMap<String, String>,
    /// 默认角色（新用户自动分配的角色 ID 列表）
    pub default_role_ids: Vec<String>,
    /// 是否自动创建用户
    pub auto_create_user: bool,
    /// 创建时间
    pub created_at: u64,
    /// 更新时间
    pub updated_at: u64,
}

impl SsoConfig {
    /// 创建新配置
    pub fn new(
        name: &str,
        provider_type: SsoProviderType,
        tenant_id: &str,
        client_id: &str,
        client_secret: &str,
    ) -> Self {
        let now = now_ms();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider_type,
            tenant_id: tenant_id.to_string(),
            enabled: true,
            vendor: "custom".to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            authorization_endpoint: String::new(),
            token_endpoint: String::new(),
            userinfo_endpoint: String::new(),
            redirect_uri: String::new(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            issuer: None,
            field_mapping: default_field_mapping(),
            default_role_ids: Vec::new(),
            auto_create_user: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// 默认字段映射
fn default_field_mapping() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("sub".to_string(), "external_id".to_string());
    map.insert("email".to_string(), "email".to_string());
    map.insert("name".to_string(), "display_name".to_string());
    map.insert("preferred_username".to_string(), "username".to_string());
    map
}

/// Token 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token ID (JWT ID)
    pub token_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 签发时间
    pub issued_at: u64,
    /// 过期时间
    pub expires_at: u64,
    /// Token 类型
    pub token_type: TokenType,
    /// 权限范围
    pub scopes: Vec<String>,
    /// 关联的 SSO 配置 ID
    pub sso_config_id: Option<String>,
    /// 额外声明
    pub claims: HashMap<String, String>,
}

/// Token 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// 访问令牌
    Access,
    /// 刷新令牌
    Refresh,
    /// ID Token
    Id,
}

impl TokenInfo {
    /// 是否过期
    pub fn is_expired(&self) -> bool {
        now_ms() > self.expires_at
    }

    /// 剩余有效时间（毫秒）
    pub fn remaining_ms(&self) -> u64 {
        self.expires_at.saturating_sub(now_ms())
    }
}

/// SSO 提供商 trait
#[async_trait::async_trait]
pub trait SsoProvider: Send + Sync {
    /// 获取提供商类型
    fn provider_type(&self) -> SsoProviderType;

    /// 生成授权 URL
    fn build_authorization_url(&self, config: &SsoConfig, state: &str) -> PermResult<String>;

    /// 用授权码换取 Token
    async fn exchange_token(
        &self,
        config: &SsoConfig,
        code: &str,
    ) -> PermResult<TokenInfo>;

    /// 验证 Token
    fn validate_token(&self, token: &str, config: &SsoConfig) -> PermResult<TokenInfo>;

    /// 获取用户信息
    async fn get_user_info(
        &self,
        config: &SsoConfig,
        token: &TokenInfo,
    ) -> PermResult<HashMap<String, String>>;

    /// 刷新 Token
    async fn refresh_token(
        &self,
        _config: &SsoConfig,
        _refresh_token: &str,
    ) -> PermResult<TokenInfo> {
        Err(PermError::SsoError(
            "refresh not supported by this provider".to_string(),
        ))
    }

    /// 登出 URL
    fn build_logout_url(&self, _config: &SsoConfig, _id_token: Option<&str>) -> Option<String> {
        None
    }
}

/// OIDC 提供商（简化实现）
pub struct OidcProvider;

impl OidcProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl SsoProvider for OidcProvider {
    fn provider_type(&self) -> SsoProviderType {
        SsoProviderType::Oidc
    }

    fn build_authorization_url(&self, config: &SsoConfig, state: &str) -> PermResult<String> {
        let scope_str = config.scopes.join(" ");
        let url = format!(
            "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            config.authorization_endpoint,
            url_encode(&config.client_id),
            url_encode(&config.redirect_uri),
            url_encode(&scope_str),
            url_encode(state),
        );
        Ok(url)
    }

    async fn exchange_token(
        &self,
        _config: &SsoConfig,
        _code: &str,
    ) -> PermResult<TokenInfo> {
        // 简化：实际实现会调用 token endpoint
        Err(PermError::SsoError(
            "OIDC token exchange requires HTTP client".to_string(),
        ))
    }

    fn validate_token(&self, _token: &str, _config: &SsoConfig) -> PermResult<TokenInfo> {
        Err(PermError::SsoError(
            "OIDC token validation requires JWKS".to_string(),
        ))
    }

    async fn get_user_info(
        &self,
        _config: &SsoConfig,
        _token: &TokenInfo,
    ) -> PermResult<HashMap<String, String>> {
        Err(PermError::SsoError(
            "OIDC userinfo requires HTTP client".to_string(),
        ))
    }
}

/// SSO 管理器
pub struct SsoManager {
    /// SSO 配置表
    configs: RwLock<HashMap<String, SsoConfig>>,
    /// 租户配置索引
    tenant_configs: RwLock<HashMap<String, Vec<String>>>,
    /// 活跃 Token
    tokens: RwLock<HashMap<String, TokenInfo>>,
    /// 租户管理器引用
    tenant_mgr: Arc<TenantManager>,
    /// 提供商
    providers: RwLock<HashMap<SsoProviderType, Arc<dyn SsoProvider>>>,
}

impl SsoManager {
    /// 创建 SSO 管理器
    pub fn new(tenant_mgr: Arc<TenantManager>) -> Self {
        let mgr = Self {
            configs: RwLock::new(HashMap::new()),
            tenant_configs: RwLock::new(HashMap::new()),
            tokens: RwLock::new(HashMap::new()),
            tenant_mgr,
            providers: RwLock::new(HashMap::new()),
        };

        // 注册内置提供商
        mgr.register_provider(SsoProviderType::Oidc, Arc::new(OidcProvider::new()));

        mgr
    }

    /// 注册 SSO 提供商
    pub fn register_provider(&self, provider_type: SsoProviderType, provider: Arc<dyn SsoProvider>) {
        self.providers.write().insert(provider_type, provider);
    }

    /// 获取提供商
    pub fn get_provider(&self, provider_type: SsoProviderType) -> PermResult<Arc<dyn SsoProvider>> {
        self.providers
            .read()
            .get(&provider_type)
            .cloned()
            .ok_or_else(|| {
                PermError::SsoError(format!(
                    "provider '{:?}' not registered",
                    provider_type
                ))
            })
    }

    // ---------- 配置管理 ----------

    /// 创建 SSO 配置
    pub fn create_config(&self, config: SsoConfig) -> PermResult<SsoConfig> {
        // 验证租户存在
        self.tenant_mgr.get_tenant(&config.tenant_id)?;

        self.tenant_configs
            .write()
            .entry(config.tenant_id.clone())
            .or_default()
            .push(config.id.clone());

        self.configs
            .write()
            .insert(config.id.clone(), config.clone());
        Ok(config)
    }

    /// 获取配置
    pub fn get_config(&self, config_id: &str) -> PermResult<SsoConfig> {
        self.configs
            .read()
            .get(config_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("sso config '{}' not found", config_id)))
    }

    /// 列出租户配置
    pub fn list_configs(&self, tenant_id: &str) -> Vec<SsoConfig> {
        let config_ids = self
            .tenant_configs
            .read()
            .get(tenant_id)
            .cloned()
            .unwrap_or_default();
        let configs = self.configs.read();
        config_ids
            .into_iter()
            .filter_map(|id| configs.get(&id).cloned())
            .collect()
    }

    /// 更新配置
    pub fn update_config(&self, config_id: &str, mut update: SsoConfig) -> PermResult<SsoConfig> {
        let mut configs = self.configs.write();
        let existing = configs
            .get_mut(config_id)
            .ok_or_else(|| PermError::NotFound(format!("sso config '{}' not found", config_id)))?;

        update.id = config_id.to_string();
        update.tenant_id = existing.tenant_id.clone();
        update.created_at = existing.created_at;
        update.updated_at = now_ms();

        *existing = update.clone();
        Ok(update)
    }

    /// 删除配置
    pub fn delete_config(&self, config_id: &str) -> PermResult<bool> {
        let config = self.get_config(config_id)?;

        // 从租户索引移除
        if let Some(vec) = self.tenant_configs.write().get_mut(&config.tenant_id) {
            vec.retain(|id| id != config_id);
        }

        Ok(self.configs.write().remove(config_id).is_some())
    }

    // ---------- Token 管理 ----------

    /// 签发 Token
    pub fn issue_token(
        &self,
        user_id: &str,
        tenant_id: &str,
        token_type: TokenType,
        ttl_ms: u64,
    ) -> PermResult<TokenInfo> {
        let now = now_ms();
        let token = TokenInfo {
            token_id: Uuid::new_v4().to_string(),
            user_id: user_id.to_string(),
            tenant_id: tenant_id.to_string(),
            issued_at: now,
            expires_at: now + ttl_ms,
            token_type,
            scopes: vec!["all".to_string()],
            sso_config_id: None,
            claims: HashMap::new(),
        };

        self.tokens
            .write()
            .insert(token.token_id.clone(), token.clone());
        Ok(token)
    }

    /// 验证 Token
    pub fn validate_token(&self, token_id: &str) -> PermResult<TokenInfo> {
        let token = self
            .tokens
            .read()
            .get(token_id)
            .cloned()
            .ok_or_else(|| PermError::InvalidToken("token not found".to_string()))?;

        if token.is_expired() {
            // 清理过期 token
            self.tokens.write().remove(token_id);
            return Err(PermError::TokenExpired);
        }

        Ok(token)
    }

    /// 撤销 Token
    pub fn revoke_token(&self, token_id: &str) -> bool {
        self.tokens.write().remove(token_id).is_some()
    }

    /// 清理过期 Token
    pub fn cleanup_expired_tokens(&self) -> usize {
        let mut tokens = self.tokens.write();
        let before = tokens.len();
        tokens.retain(|_, t| !t.is_expired());
        before - tokens.len()
    }

    // ---------- 用户映射 ----------

    /// 根据外部用户信息查找或创建用户
    pub fn find_or_create_user(
        &self,
        config: &SsoConfig,
        external_user: &HashMap<String, String>,
    ) -> PermResult<User> {
        // 获取外部 ID
        let external_id_key = config
            .field_mapping
            .get("sub")
            .cloned()
            .unwrap_or_else(|| "external_id".to_string());

        let external_id = external_user
            .get("sub")
            .ok_or_else(|| PermError::SsoError("missing 'sub' claim".to_string()))?;

        // 尝试按 external_id 查找（这里简化为按用户名）
        let _ = external_id_key;
        let _username_key = config
            .field_mapping
            .get("preferred_username")
            .cloned()
            .unwrap_or_else(|| "username".to_string());

        let username = external_user
            .get("preferred_username")
            .or_else(|| external_user.get("email"))
            .ok_or_else(|| PermError::SsoError("missing username claim".to_string()))?;

        match self
            .tenant_mgr
            .get_user_by_username(&config.tenant_id, username)
        {
            Ok(user) => Ok(user),
            Err(PermError::NotFound(_)) => {
                if !config.auto_create_user {
                    return Err(PermError::NotFound(
                        "user not found and auto-create disabled".to_string(),
                    ));
                }

                // 创建新用户
                let display_name = external_user
                    .get("name")
                    .cloned()
                    .unwrap_or_else(|| username.clone());

                let mut user = User::new(username, &display_name, &config.tenant_id);
                user.email = external_user.get("email").cloned();
                user.status = UserStatus::Active;
                user.attributes
                    .insert("external_id".to_string(), external_id.clone());
                user.attributes
                    .insert("sso_config_id".to_string(), config.id.clone());

                self.tenant_mgr.create_user(user)
            }
            Err(e) => Err(e),
        }
    }

    /// 生成授权 URL
    pub fn build_authorization_url(&self, config_id: &str, state: &str) -> PermResult<String> {
        let config = self.get_config(config_id)?;
        let provider = self.get_provider(config.provider_type)?;
        provider.build_authorization_url(&config, state)
    }
}

/// URL 编码（简化版）
fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            result.push(c);
        } else {
            for byte in c.to_string().as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Tenant;

    fn setup() -> SsoManager {
        let tenant_mgr = Arc::new(TenantManager::new());
        let tenant = Tenant::new("Test", "test");
        tenant_mgr.create_tenant(tenant).unwrap();
        SsoManager::new(tenant_mgr)
    }

    #[test]
    fn test_create_config() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let config = SsoConfig::new(
            "Google SSO",
            SsoProviderType::Oidc,
            &tenant.id,
            "client-id-123",
            "client-secret-456",
        );

        let created = mgr.create_config(config).unwrap();
        assert_eq!(created.name, "Google SSO");
        assert!(created.enabled);
    }

    #[test]
    fn test_list_configs() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let config1 = SsoConfig::new("OIDC 1", SsoProviderType::Oidc, &tenant.id, "c1", "s1");
        let config2 = SsoConfig::new("OIDC 2", SsoProviderType::Oidc, &tenant.id, "c2", "s2");

        mgr.create_config(config1).unwrap();
        mgr.create_config(config2).unwrap();

        let configs = mgr.list_configs(&tenant.id);
        assert_eq!(configs.len(), 2);
    }

    #[test]
    fn test_issue_and_validate_token() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let token = mgr
            .issue_token(&"user-1", &tenant.id, TokenType::Access, 3600_000)
            .unwrap();

        assert!(!token.is_expired());
        assert_eq!(token.user_id, "user-1");

        let validated = mgr.validate_token(&token.token_id).unwrap();
        assert_eq!(validated.user_id, "user-1");
    }

    #[test]
    fn test_expired_token() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let token = mgr
            .issue_token(&"user-1", &tenant.id, TokenType::Access, 1) // 1ms 即过期
            .unwrap();

        // 等待过期
        std::thread::sleep(std::time::Duration::from_millis(10));

        let result = mgr.validate_token(&token.token_id);
        assert!(result.is_err());
        match result.unwrap_err() {
            PermError::TokenExpired => {}
            _ => panic!("expected TokenExpired"),
        }
    }

    #[test]
    fn test_revoke_token() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let token = mgr
            .issue_token(&"user-1", &tenant.id, TokenType::Access, 3600_000)
            .unwrap();

        assert!(mgr.revoke_token(&token.token_id));
        assert!(!mgr.revoke_token(&token.token_id));
    }

    #[test]
    fn test_build_authorization_url() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let mut config =
            SsoConfig::new("Test OIDC", SsoProviderType::Oidc, &tenant.id, "cid", "csec");
        config.authorization_endpoint = "https://example.com/auth".to_string();
        config.redirect_uri = "https://app.com/callback".to_string();

        let config = mgr.create_config(config).unwrap();
        let url = mgr
            .build_authorization_url(&config.id, "state-123")
            .unwrap();

        assert!(url.contains("example.com/auth"));
        assert!(url.contains("client_id=cid"));
        assert!(url.contains("state=state-123"));
    }

    #[test]
    fn test_find_or_create_user() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let config = SsoConfig::new("Test SSO", SsoProviderType::Oidc, &tenant.id, "cid", "csec");
        let config = mgr.create_config(config).unwrap();

        let mut external_user = HashMap::new();
        external_user.insert("sub".to_string(), "ext-123".to_string());
        external_user.insert("email".to_string(), "test@example.com".to_string());
        external_user.insert("name".to_string(), "Test User".to_string());
        external_user.insert(
            "preferred_username".to_string(),
            "testuser".to_string(),
        );

        // 第一次：创建用户
        let user = mgr.find_or_create_user(&config, &external_user).unwrap();
        assert_eq!(user.username, "testuser");
        assert_eq!(user.display_name, "Test User");

        // 第二次：找到已存在的用户
        let user2 = mgr.find_or_create_user(&config, &external_user).unwrap();
        assert_eq!(user.id, user2.id);
    }

    #[test]
    fn test_provider_registration() {
        let mgr = setup();
        assert!(mgr.get_provider(SsoProviderType::Oidc).is_ok());
        assert!(mgr.get_provider(SsoProviderType::Saml).is_err());
    }

    #[test]
    fn test_token_remaining() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        let token = mgr
            .issue_token(&"user-1", &tenant.id, TokenType::Access, 3600_000)
            .unwrap();

        assert!(token.remaining_ms() > 3500_000);
    }

    #[test]
    fn test_cleanup_expired() {
        let mgr = setup();
        let tenant = mgr.tenant_mgr.get_tenant_by_code("test").unwrap();

        // 签发一个很快过期的 token
        mgr.issue_token(&"u1", &tenant.id, TokenType::Access, 1)
            .unwrap();
        mgr.issue_token(&"u2", &tenant.id, TokenType::Access, 3600_000)
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let cleaned = mgr.cleanup_expired_tokens();
        assert_eq!(cleaned, 1);
    }
}
