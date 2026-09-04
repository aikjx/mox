// =============================================================================
// Token 认证模块（轻量 JWT 实现）
// =============================================================================

use crate::{AuthError, AuthResult};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

// =============================================================================
// Token 类型
// =============================================================================

/// Token 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenType {
    /// 访问令牌（短期）
    Access,
    /// 刷新令牌（长期）
    Refresh,
}

impl TokenType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TokenType::Access => "access",
            TokenType::Refresh => "refresh",
        }
    }
}

// =============================================================================
// JWT 配置
// =============================================================================

/// JWT 配置
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// 密钥
    pub secret: String,
    /// 签发者
    pub issuer: String,
    /// 受众
    pub audience: String,
    /// 访问令牌有效期（秒）
    pub access_token_ttl: i64,
    /// 刷新令牌有效期（秒）
    pub refresh_token_ttl: i64,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: "mox-default-secret-change-in-production".to_string(),
            issuer: "mox-platform".to_string(),
            audience: "mox-users".to_string(),
            access_token_ttl: 3600,
            refresh_token_ttl: 604800,
        }
    }
}

impl JwtConfig {
    pub fn new(secret: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            ..Default::default()
        }
    }

    pub fn with_issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = issuer.into();
        self
    }

    pub fn with_access_ttl(mut self, seconds: i64) -> Self {
        self.access_token_ttl = seconds;
        self
    }

    pub fn with_refresh_ttl(mut self, seconds: i64) -> Self {
        self.refresh_token_ttl = seconds;
        self
    }
}

// =============================================================================
// Claims
// =============================================================================

/// Token Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// 主体（用户ID）
    pub sub: String,
    /// 签发者
    pub iss: String,
    /// 受众
    pub aud: String,
    /// 签发时间
    pub iat: i64,
    /// 过期时间
    pub exp: i64,
    /// Token ID
    pub jti: String,
    /// Token 类型
    pub token_type: TokenType,
    /// 租户ID
    pub tenant_id: String,
    /// 角色列表
    pub roles: Vec<String>,
    /// 用户名
    pub username: String,
}

impl Claims {
    pub fn access(
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        roles: Vec<String>,
        config: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.into(),
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(config.access_token_ttl)).timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Access,
            tenant_id: tenant_id.into(),
            roles,
            username: username.into(),
        }
    }

    pub fn refresh(
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        config: &JwtConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id.into(),
            iss: config.issuer.clone(),
            aud: config.audience.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(config.refresh_token_ttl)).timestamp(),
            jti: Uuid::new_v4().to_string(),
            token_type: TokenType::Refresh,
            tenant_id: tenant_id.into(),
            roles: vec![],
            username: username.into(),
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    pub fn remaining_seconds(&self) -> i64 {
        self.exp - Utc::now().timestamp()
    }
}

// =============================================================================
// 刷新令牌响应
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
}

// =============================================================================
// JWT 管理器
// =============================================================================

/// JWT 管理器
#[derive(Clone)]
pub struct JwtManager {
    config: JwtConfig,
}

impl JwtManager {
    pub fn new(config: JwtConfig) -> AuthResult<Self> {
        if config.secret.len() < 16 {
            return Err(AuthError::ConfigError(
                "JWT 密钥长度至少16位".to_string(),
            ));
        }
        Ok(Self { config })
    }

    /// 签发访问令牌
    pub fn issue_access_token(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        roles: Vec<String>,
    ) -> AuthResult<String> {
        let claims = Claims::access(user_id, tenant_id, username, roles, &self.config);
        self.encode(&claims)
    }

    /// 签发刷新令牌
    pub fn issue_refresh_token(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        username: impl Into<String>,
    ) -> AuthResult<String> {
        let claims = Claims::refresh(user_id, tenant_id, username, &self.config);
        self.encode(&claims)
    }

    /// 签发令牌对
    pub fn issue_token_pair(
        &self,
        user_id: impl Into<String>,
        tenant_id: impl Into<String>,
        username: impl Into<String>,
        roles: Vec<String>,
    ) -> AuthResult<RefreshToken> {
        let user_id = user_id.into();
        let tenant_id = tenant_id.into();
        let username = username.into();

        let access_token = self.issue_access_token(
            user_id.clone(),
            tenant_id.clone(),
            username.clone(),
            roles,
        )?;
        let refresh_token = self.issue_refresh_token(user_id, tenant_id, username)?;

        Ok(RefreshToken {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_ttl,
        })
    }

    /// 验证令牌
    pub fn verify_token(&self, token: &str, expected_type: TokenType) -> AuthResult<Claims> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken("Token 格式错误".to_string()));
        }

        // 验证签名
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let expected_sig = self.sign(&signing_input);
        let actual_sig = parts[2];

        if expected_sig != actual_sig {
            return Err(AuthError::InvalidToken("签名验证失败".to_string()));
        }

        // 解码 payload
        let payload_json = URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| AuthError::InvalidToken(format!("Payload 解码失败: {}", e)))?;

        let claims: Claims = serde_json::from_slice(&payload_json)
            .map_err(|e| AuthError::InvalidToken(format!("Claims 解析失败: {}", e)))?;

        // 验证签发者
        if claims.iss != self.config.issuer {
            return Err(AuthError::InvalidToken(format!(
                "签发者不匹配: 期望 {}, 实际 {}",
                self.config.issuer, claims.iss
            )));
        }

        // 验证受众
        if claims.aud != self.config.audience {
            return Err(AuthError::InvalidToken(format!(
                "受众不匹配: 期望 {}, 实际 {}",
                self.config.audience, claims.aud
            )));
        }

        // 验证过期
        if claims.is_expired() {
            return Err(AuthError::TokenExpired);
        }

        // 验证类型
        if claims.token_type != expected_type {
            return Err(AuthError::TokenTypeMismatch {
                expected: expected_type.as_str().to_string(),
                actual: claims.token_type.as_str().to_string(),
            });
        }

        Ok(claims)
    }

    pub fn verify_access_token(&self, token: &str) -> AuthResult<Claims> {
        self.verify_token(token, TokenType::Access)
    }

    pub fn verify_refresh_token(&self, token: &str) -> AuthResult<Claims> {
        self.verify_token(token, TokenType::Refresh)
    }

    /// 刷新访问令牌
    pub fn refresh_access_token(
        &self,
        refresh_token: &str,
        roles: Vec<String>,
    ) -> AuthResult<RefreshToken> {
        let claims = self.verify_refresh_token(refresh_token)?;

        let new_access = self.issue_access_token(
            claims.sub.clone(),
            claims.tenant_id.clone(),
            claims.username.clone(),
            roles,
        )?;

        Ok(RefreshToken {
            access_token: new_access,
            refresh_token: refresh_token.to_string(),
            token_type: "Bearer".to_string(),
            expires_in: self.config.access_token_ttl,
        })
    }

    /// 从 Authorization header 提取 Bearer token
    pub fn extract_bearer_token(auth_header: &str) -> AuthResult<String> {
        let parts: Vec<&str> = auth_header.split_whitespace().collect();
        if parts.len() != 2 || parts[0].to_lowercase() != "bearer" {
            return Err(AuthError::InvalidToken(
                "Authorization header 格式错误，期望 'Bearer <token>'".to_string(),
            ));
        }
        Ok(parts[1].to_string())
    }

    // ── 内部方法 ──────────────────────────────────────────────────────────

    fn encode(&self, claims: &Claims) -> AuthResult<String> {
        // Header
        let header = serde_json::json!({
            "alg": "HS256",
            "typ": "JWT"
        });
        let header_b64 = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());

        // Payload
        let payload_b64 = URL_SAFE_NO_PAD.encode(
            serde_json::to_vec(claims)
                .map_err(|e| AuthError::InternalError(format!("Claims 序列化失败: {}", e)))?,
        );

        // Signature
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let signature = self.sign(&signing_input);

        Ok(format!("{}.{}", signing_input, signature))
    }

    fn sign(&self, data: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(self.config.secret.as_bytes())
            .expect("HMAC 密钥长度错误");
        mac.update(data.as_bytes());
        let result = mac.finalize();
        URL_SAFE_NO_PAD.encode(result.into_bytes())
    }
}

impl std::fmt::Debug for JwtManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtManager")
            .field("issuer", &self.config.issuer)
            .field("access_ttl", &self.config.access_token_ttl)
            .field("refresh_ttl", &self.config.refresh_token_ttl)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_manager() -> JwtManager {
        let config = JwtConfig::new("test-secret-key-for-testing-only-12345")
            .with_access_ttl(3600)
            .with_refresh_ttl(604800);
        JwtManager::new(config).unwrap()
    }

    #[test]
    fn test_issue_and_verify_access_token() {
        let manager = setup_manager();
        let roles = vec!["admin".to_string(), "user".to_string()];

        let token = manager
            .issue_access_token("user-001", "tenant-001", "alice", roles.clone())
            .unwrap();

        assert!(!token.is_empty());
        assert!(token.starts_with("eyJ"));

        let claims = manager.verify_access_token(&token).unwrap();
        assert_eq!(claims.sub, "user-001");
        assert_eq!(claims.tenant_id, "tenant-001");
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.roles, roles);
        assert_eq!(claims.token_type, TokenType::Access);
        assert!(!claims.is_expired());
    }

    #[test]
    fn test_issue_and_verify_refresh_token() {
        let manager = setup_manager();

        let token = manager
            .issue_refresh_token("user-001", "tenant-001", "alice")
            .unwrap();

        let claims = manager.verify_refresh_token(&token).unwrap();
        assert_eq!(claims.sub, "user-001");
        assert_eq!(claims.token_type, TokenType::Refresh);
        assert!(claims.roles.is_empty());
    }

    #[test]
    fn test_token_type_mismatch() {
        let manager = setup_manager();

        let access_token = manager
            .issue_access_token("user-001", "tenant-001", "alice", vec![])
            .unwrap();

        let result = manager.verify_refresh_token(&access_token);
        assert!(matches!(result, Err(AuthError::TokenTypeMismatch { .. })));
    }

    #[test]
    fn test_invalid_token() {
        let manager = setup_manager();
        assert!(manager.verify_access_token("invalid.token.here").is_err());
        assert!(manager.verify_access_token("not-a-jwt").is_err());
    }

    #[test]
    fn test_tampered_signature() {
        let manager = setup_manager();

        let token = manager
            .issue_access_token("user-001", "tenant-001", "alice", vec![])
            .unwrap();

        // 篡改签名
        let parts: Vec<&str> = token.split('.').collect();
        let tampered = format!("{}.{}.tampered", parts[0], parts[1]);

        assert!(manager.verify_access_token(&tampered).is_err());
    }

    #[test]
    fn test_expired_token() {
        let config = JwtConfig::new("test-secret-key-1234567890").with_access_ttl(-100);
        let manager = JwtManager::new(config).unwrap();

        let token = manager
            .issue_access_token("user-001", "tenant-001", "alice", vec![])
            .unwrap();

        let result = manager.verify_access_token(&token);
        assert!(matches!(result, Err(AuthError::TokenExpired)));
    }

    #[test]
    fn test_token_pair() {
        let manager = setup_manager();
        let roles = vec!["user".to_string()];

        let pair = manager
            .issue_token_pair("user-001", "tenant-001", "alice", roles)
            .unwrap();

        assert!(!pair.access_token.is_empty());
        assert!(!pair.refresh_token.is_empty());
        assert_eq!(pair.token_type, "Bearer");
        assert_eq!(pair.expires_in, 3600);

        assert!(manager.verify_access_token(&pair.access_token).is_ok());
        assert!(manager.verify_refresh_token(&pair.refresh_token).is_ok());
    }

    #[test]
    fn test_refresh_access_token() {
        let manager = setup_manager();
        let roles = vec!["user".to_string()];

        let pair = manager
            .issue_token_pair("user-001", "tenant-001", "alice", roles.clone())
            .unwrap();

        let new_pair = manager
            .refresh_access_token(&pair.refresh_token, roles)
            .unwrap();

        assert!(!new_pair.access_token.is_empty());
        assert!(manager.verify_access_token(&new_pair.access_token).is_ok());
    }

    #[test]
    fn test_extract_bearer_token() {
        assert_eq!(
            JwtManager::extract_bearer_token("Bearer abc123").unwrap(),
            "abc123"
        );
        assert_eq!(
            JwtManager::extract_bearer_token("bearer abc123").unwrap(),
            "abc123"
        );
        assert!(JwtManager::extract_bearer_token("Basic abc123").is_err());
        assert!(JwtManager::extract_bearer_token("invalid").is_err());
    }

    #[test]
    fn test_short_secret_rejected() {
        let config = JwtConfig::new("short");
        assert!(JwtManager::new(config).is_err());
    }

    #[test]
    fn test_claims_remaining_seconds() {
        let config = JwtConfig::new("test-secret-key-1234567890").with_access_ttl(3600);
        let claims = Claims::access("u", "t", "n", vec![], &config);
        let remaining = claims.remaining_seconds();
        assert!(remaining > 3500);
        assert!(remaining <= 3600);
    }
}
