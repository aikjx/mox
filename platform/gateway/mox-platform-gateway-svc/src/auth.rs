// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Authentication middleware.
//!
//! Provides JWT token validation, API key authentication, and role-based access control.
//! Depends only on L2 `mox_platform_api` trait contracts.
//!
//! # P0-1 安全闭环
//! `validate_token` 现做**真实 HMAC-SHA256 (HS256) 验签**：仅接受 `alg=HS256`、
//! 恒定时间比较签名、校验 `iss` / `exp` 声明。此前仅做 base64 结构解码（任何伪造
//! 3 段 base64 均可通过），属严重认证绕过漏洞，现已修复。
//!
//! # P1-04 路由级鉴权
//! `auth_middleware` 在放行前把校验后的 `UserInfo` 注入请求扩展；
//! handler 可通过 `ApiAuth` extractor 取出当前用户身份，做细粒度（角色）校验。

use crate::config::AuthConfig;
use axum::{
    extract::{FromRequestParts, Request},
    http::{header, request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use mox_platform_api::UserInfo;
use sha2::{Digest, Sha256};
use hmac::{Hmac, Mac};
use std::sync::Arc;

/// HMAC-SHA256 类型别名（JWT `HS256` 验签）。
type HmacSha256 = Hmac<Sha256>;

/// Authentication middleware that validates requests before they reach handlers.
pub struct AuthMiddleware {
    config: AuthConfig,
    iam: Option<Arc<mox_platform_iam_core::IamRepository>>,
    /// Standalone registration helper; never used by HTTP authentication.
    api_keys: Arc<parking_lot::RwLock<std::collections::HashMap<String, String>>>,
    /// 令牌黑名单 + 活跃会话注册表（P0-2 登出/踢人）。
    pub(crate) blacklist: Arc<crate::token_blacklist::TokenBlacklist>,
}

impl AuthMiddleware {
    /// Create a new auth middleware with the given configuration.
    pub fn new(config: AuthConfig) -> Self {
        Self {
            blacklist: Arc::new(crate::token_blacklist::TokenBlacklist::new()),
            config,
            iam: None,
            api_keys: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_iam(mut self, iam: Arc<mox_platform_iam_core::IamRepository>) -> Self {
        self.iam = Some(iam);
        self
    }

    /// Register an API key for a user.
    pub fn register_api_key(&self, user_id: &str, api_key: &str) {
        let hash = hash_api_key(api_key);
        self.api_keys.write().insert(hash, user_id.to_string());
    }

    /// Revoke an API key.
    pub fn revoke_api_key(&self, api_key: &str) {
        let hash = hash_api_key(api_key);
        self.api_keys.write().remove(&hash);
    }

    /// Check if a path is public (doesn't require auth).
    pub fn is_public_path(&self, path: &str) -> bool {
        self.config.public_paths.iter().any(|p| path.starts_with(p))
    }

    /// Validate a JWT token and return the user info.
    ///
    /// 真实验签（P0-1 安全闭环）：仅接受 `alg=HS256`，使用 `config.jwt_secret`
    /// 做 HMAC-SHA256 签名校验（恒定时间比较），并校验 `iss` / `exp` 声明。
    /// 任何伪造、篡改或算法降级（如 `alg=none`）的令牌都将被拒绝。
    pub fn validate_token(&self, token: &str) -> Option<UserInfo> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 { return None; }
        let (header_b64, payload_b64, sig_b64) = (parts[0], parts[1], parts[2]);

        // 解码 header 并强制算法白名单：仅允许 HS256，拒绝 alg=none / RS* 等绕过。
        let header_bytes = base64_decode(header_b64)?;
        let header: serde_json::Value = serde_json::from_slice(&header_bytes).ok()?;
        let alg = header.get("alg").and_then(|v| v.as_str()).unwrap_or("");
        if alg != "HS256" {
            return None;
        }

        // 验签：signing_input 必须是令牌原始的第 1/2 段（不能重新编码，否则 pad 差异导致验签失败）。
        let signing_input = format!("{}.{}", header_b64, payload_b64);
        let sig_bytes = base64_decode(sig_b64)
            .or_else(|| {
                use base64::Engine;
                base64::engine::general_purpose::URL_SAFE.decode(sig_b64).ok()
            })?;
        let mut mac = HmacSha256::new_from_slice(self.config.jwt_secret.as_bytes()).ok()?;
        mac.update(signing_input.as_bytes());
        // verify_slice 做恒定时间比较，避免时序侧信道。
        if mac.verify_slice(&sig_bytes).is_err() {
            return None;
        }

        // 解码 payload 并校验核心声明。
        let payload_bytes = base64_decode(payload_b64)?;
        let claims: serde_json::Value = serde_json::from_slice(&payload_bytes).ok()?;

        // iss 校验（若存在）：防止跨签发方令牌冒用。
        if let Some(iss) = claims.get("iss").and_then(|v| v.as_str()) {
            if iss != self.config.token_issuer {
                return None;
            }
        }
        // exp 校验（若存在且已过期）：拒绝过期令牌。
        if let Some(exp) = claims.get("exp").and_then(|v| v.as_i64()) {
            if exp < chrono::Utc::now().timestamp() {
                return None;
            }
        }

        let user_id = claims.get("sub")?.as_str()?.to_string();
        let tenant_id = claims.get("tenant_id").and_then(|v| v.as_str()).unwrap_or("default").to_string();
        let roles = claims.get("roles")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // P0-2：jti 命中黑名单即拒绝；验签成功登记活跃会话（供管理员列出/踢人）。
        let jti = claims.get("jti").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !jti.is_empty() && self.blacklist.is_revoked(&jti) {
            return None;
        }
        let exp_ts = claims.get("exp").and_then(|v| v.as_i64()).unwrap_or(0);
        let sub = claims.get("sub").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if !jti.is_empty() {
            self.blacklist.record(&jti, &sub, exp_ts);
        }

        Some(UserInfo {
            id: user_id,
            username: claims.get("username").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
            email: claims.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            tenant_id,
            roles,
            enabled: true,
            created_at: String::new(),
        })
    }

    /// Validate an API key and return the user ID.
    pub fn validate_api_key(&self, api_key: &str) -> Option<String> {
        let hash = hash_api_key(api_key);
        self.api_keys.read().get(&hash).cloned()
    }

    /// P0-2：吊销指定会话（登出/管理员踢人）。
    pub fn revoke_session(&self, jti: &str, exp: i64) {
        self.blacklist.revoke(jti, exp);
    }

    /// P0-2：列出某用户活跃会话。
    pub fn list_user_sessions(&self, sub: &str) -> Vec<crate::token_blacklist::SessionRec> {
        self.blacklist.list_user_sessions(sub)
    }
}

/// Axum middleware function for authentication.
pub async fn auth_middleware(
    auth: Arc<AuthMiddleware>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !auth.config.enabled || auth.is_public_path(request.uri().path()) {
        return Ok(next.run(request).await);
    }

    // Try Bearer token first
    let user_info = if let Some(auth_header) = request.headers().get(header::AUTHORIZATION) {
        let auth_str = auth_header.to_str().unwrap_or("");
        if let Some(token) = auth_str.strip_prefix("Bearer ") {
            // Only the documented loopback development token gets the development identity.
            // Arbitrary invalid tokens must fail even when development mode is enabled.
            if auth.config.dev_mode && !auth.config.disable_dev_token && token == "dev-secret-token" {
                auth.validate_token(token).or_else(|| Some(UserInfo {
                    id: "dev-user".into(),
                    username: "dev-user".into(),
                    email: String::new(),
                    tenant_id: "default".into(),
                    roles: vec!["admin".into()],
                    enabled: true,
                    created_at: String::new(),
                }))
            } else {
                auth.validate_token(token)
            }
        } else {
            None
        }
    } else {
        // Try API key
        if let Some(api_key) = request.headers().get("X-API-Key") {
            let key = api_key.to_str().unwrap_or("").to_string();
            let iam = auth.iam.clone().ok_or(StatusCode::UNAUTHORIZED)?;
            let principal = tokio::task::spawn_blocking(move || iam.authenticate_api_key(&key))
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .map_err(|error| {
                    tracing::error!(%error, "API key authentication storage failed");
                    StatusCode::SERVICE_UNAVAILABLE
                })?;
            if principal.is_some() && !api_key_route_allowed(request.method(), request.uri().path()) {
                return Err(StatusCode::FORBIDDEN);
            }
            principal.map(|principal| UserInfo {
                id: principal.id,
                username: principal.username,
                email: principal.email,
                tenant_id: principal.tenant_id,
                roles: vec!["api".into()],
                enabled: true,
                created_at: String::new(),
            })
        } else {
            None
        }
    };

    if user_info.is_some() {
        // P1-04：注入 UserInfo 到请求扩展，供下游 `ApiAuth` extractor 提取（路由级鉴权）。
        if let Some(user) = user_info.as_ref() {
            request.extensions_mut().insert(user.clone());
        }
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Only audited inbox workflows accept machine credentials; administrative APIs require JWT.
pub fn api_key_route_allowed(method: &axum::http::Method, path: &str) -> bool {
    use axum::http::Method;
    if method == Method::GET {
        return path == "/api/notifications"
            || path == "/api/notifications/unread-count"
            || matches!(
                path,
                "/api/enterprise/message/messages" | "/api/enterprise/message/stats"
            )
            || path
                .strip_prefix("/api/enterprise/message/messages/")
                .is_some_and(|id| !id.is_empty() && !id.contains('/'));
    }
    if method == Method::POST {
        return path == "/api/enterprise/message/send"
            || path
                .strip_prefix("/api/enterprise/message/messages/")
                .and_then(|id| id.strip_suffix("/read"))
                .is_some_and(|id| !id.is_empty() && !id.contains('/'));
    }
    method == Method::PUT
        && (path == "/api/notifications/read-all"
            || path
                .strip_prefix("/api/notifications/")
                .and_then(|id| id.strip_suffix("/read"))
                .is_some_and(|id| !id.is_empty() && !id.contains('/')))
}
/// 路由级鉴权 extractor（P1-04）。
///
/// 从 `auth_middleware` 注入到请求扩展的 `UserInfo` 中提取当前用户身份，
/// 供 handler 做细粒度（角色）校验。仅对经过认证中间件且已注入用户的请求有效；
/// 公开路径或未认证请求使用本 extractor 将收到 401。
///
/// # 用法
/// ```ignore
/// async fn create_role(ApiAuth(user): ApiAuth, Json(body): Json<RoleReq>) -> ApiResponse<Value> {
///     if !user.roles.iter().any(|r| r == "admin") {
///         return api_error(StatusCode::FORBIDDEN, "需要 admin 角色");
///     }
///     // ...
/// }
/// ```
/// 路由级鉴权 extractor（P1-04）。
///
/// 从 `auth_middleware` 注入到请求扩展的 `UserInfo` 中提取当前用户身份，
/// 供 handler 做细粒度（角色）校验。仅对经过认证中间件且已注入用户的请求有效；
/// 公开路径或未认证请求使用本 extractor 将收到 401。
///
/// 注：axum 0.7 的 `FromRequestParts` 由 `#[async_trait]` 定义。为避免引入额外的
/// `async_trait` 直接依赖（离线环境下无法从 registry 解析），此处手写其展开后的等价
/// 签名（沿用 `async_trait` 0.1 生成的 `'life0` / `'life1` / `'async_trait` 生命周期命名），
/// 语义完全一致。
pub struct ApiAuth(pub UserInfo);

impl<S> FromRequestParts<S> for ApiAuth
where
    S: Send + Sync + 'static,
{
    type Rejection = StatusCode;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        S: 'async_trait,
    {
        Box::pin(async move {
            parts
                .extensions
                .get::<UserInfo>()
                .cloned()
                .map(ApiAuth)
                .ok_or(StatusCode::UNAUTHORIZED)
        })
    }
}

fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hex::encode(hasher.finalize())
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    /// 用给定 secret 生成一枚 HS256 签名的 JWT（测试夹具）。
    fn sign_jwt(secret: &str, header_json: &str, payload_json: &str) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header_json);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
        let signing_input = format!("{}.{}", header, payload);
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(signing_input.as_bytes());
        let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        format!("{}.{}.{}", header, payload, sig)
    }

    fn auth_with_secret(secret: &str) -> AuthMiddleware {
        let mut cfg = AuthConfig::default();
        cfg.jwt_secret = secret.to_string();
        AuthMiddleware::new(cfg)
    }

    #[test]
    fn test_public_path() {
        let auth = AuthMiddleware::new(AuthConfig::default());
        assert!(auth.is_public_path("/health"));
        assert!(auth.is_public_path("/api/auth/login"));
        assert!(!auth.is_public_path("/api/data/records"));
    }

    #[test]
    fn machine_credentials_require_exact_workflow_method_and_path() {
        use axum::http::Method;
        for (method, path) in [
            (Method::GET, "/api/notifications"),
            (Method::GET, "/api/enterprise/message/messages/id"),
            (Method::POST, "/api/enterprise/message/send"),
            (Method::POST, "/api/enterprise/message/messages/id/read"),
            (Method::PUT, "/api/notifications/id/read"),
            (Method::PUT, "/api/notifications/read-all"),
        ] {
            assert!(api_key_route_allowed(&method, path), "{method} {path}");
        }
        for (method, path) in [
            (Method::GET, "/api/system/iam/permissions"),
            (Method::POST, "/api/security/api-keys"),
            (Method::GET, "/api/notifications-archive"),
            (Method::POST, "/api/notifications/read-all"),
            (Method::DELETE, "/api/enterprise/message/messages/id"),
            (Method::GET, "/api/enterprise/message/messages/id/read"),
            (Method::POST, "/api/enterprise/message/messages/a/b/read"),
            (Method::GET, "/api/enterprise/message/messages/"),
        ] {
            assert!(!api_key_route_allowed(&method, path), "{method} {path}");
        }
    }

    #[test]
    fn test_api_key_registration() {
        let auth = AuthMiddleware::new(AuthConfig::default());
        auth.register_api_key("user1", "secret-key-123");
        assert_eq!(auth.validate_api_key("secret-key-123"), Some("user1".to_string()));
        assert_eq!(auth.validate_api_key("wrong-key"), None);
        auth.revoke_api_key("secret-key-123");
        assert_eq!(auth.validate_api_key("secret-key-123"), None);
    }

    // ===== P0-1 真验签回归测试 =====

    #[test]
    fn test_validate_token_valid_hs256() {
        let auth = auth_with_secret("unit-test-secret");
        let token = sign_jwt(
            "unit-test-secret",
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"u1","username":"alice","tenant_id":"t1","roles":["admin"],"iss":"mox-platform","exp":9999999999}"#,
        );
        let u = auth.validate_token(&token).expect("valid token must pass");
        assert_eq!(u.id, "u1");
        assert_eq!(u.roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_validate_token_rejects_tampered_sig() {
        let auth = auth_with_secret("unit-test-secret");
        let mut token = sign_jwt(
            "unit-test-secret",
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"u1","iss":"mox-platform","exp":9999999999}"#,
        );
        let parts: Vec<&str> = token.split('.').collect();
        let fake_sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("forged");
        token = format!("{}.{}.{}", parts[0], parts[1], fake_sig);
        assert!(auth.validate_token(&token).is_none(), "tampered signature must be rejected");
    }

    #[test]
    fn test_validate_token_rejects_alg_none() {
        let auth = auth_with_secret("unit-test-secret");
        let token = sign_jwt(
            "unit-test-secret",
            r#"{"alg":"none","typ":"JWT"}"#,
            r#"{"sub":"u1","iss":"mox-platform","exp":9999999999}"#,
        );
        assert!(auth.validate_token(&token).is_none(), "alg=none must be rejected");
    }

    #[test]
    fn test_validate_token_rejects_wrong_secret() {
        let auth = auth_with_secret("correct-secret");
        // 用错误密钥签名
        let token = sign_jwt(
            "wrong-secret",
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"u1","iss":"mox-platform","exp":9999999999}"#,
        );
        assert!(auth.validate_token(&token).is_none(), "wrong-secret signature must be rejected");
    }

    #[test]
    fn test_validate_token_rejects_expired() {
        let auth = auth_with_secret("unit-test-secret");
        let token = sign_jwt(
            "unit-test-secret",
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"u1","iss":"mox-platform","exp":1}"#,
        );
        assert!(auth.validate_token(&token).is_none(), "expired token must be rejected");
    }

    #[test]
    fn test_validate_token_rejects_wrong_issuer() {
        let auth = auth_with_secret("unit-test-secret");
        let token = sign_jwt(
            "unit-test-secret",
            r#"{"alg":"HS256","typ":"JWT"}"#,
            r#"{"sub":"u1","iss":"evil-issuer","exp":9999999999}"#,
        );
        assert!(auth.validate_token(&token).is_none(), "wrong issuer must be rejected");
    }

    #[test]
    fn test_validate_token_rejects_malformed() {
        let auth = auth_with_secret("unit-test-secret");
        assert!(auth.validate_token("not-a-jwt").is_none());
        assert!(auth.validate_token("a.b").is_none());
        assert!(auth.validate_token("").is_none());
    }

    // ===== P1-04 ApiAuth extractor 测试 =====

    #[tokio::test]
    async fn api_auth_extracts_user_from_extensions() {
        // http::request::Parts 字段私有，无法直接字面量构造；
        // 改用 http::Request 构造并拆分出 parts，再注入 UserInfo 到 extensions。
        let mut req = axum::http::Request::builder()
            .uri("/api/system/roles")
            .body(())
            .unwrap();
        let user = UserInfo {
            id: "u1".into(),
            username: "alice".into(),
            email: String::new(),
            tenant_id: "t1".into(),
            roles: vec!["admin".into()],
            enabled: true,
            created_at: String::new(),
        };
        req.extensions_mut().insert(user);

        let (mut parts, _body) = req.into_parts();
        let auth = ApiAuth::from_request_parts(&mut parts, &()).await.unwrap();
        assert_eq!(auth.0.id, "u1");
        assert_eq!(auth.0.roles, vec!["admin".to_string()]);
    }

    #[tokio::test]
    async fn api_auth_rejects_when_no_user_injected() {
        let req = axum::http::Request::builder().uri("/x").body(()).unwrap();
        let (mut parts, _body) = req.into_parts();
        assert!(
            ApiAuth::from_request_parts(&mut parts, &()).await.is_err(),
            "未注入 UserInfo 时必须拒绝"
        );
    }

    #[tokio::test]
    async fn dev_mode_rejects_arbitrary_bearer_tokens() {
        use tower::ServiceExt;
        let mut config = AuthConfig::default();
        config.enabled = true;
        config.dev_mode = true;
        config.public_paths.clear();
        let auth = Arc::new(AuthMiddleware::new(config));
        let app = axum::Router::new().route("/api/auth/me", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn(move |req, next| auth_middleware(auth.clone(), req, next)));
        for (token, expected) in [("invalid-test-token", StatusCode::UNAUTHORIZED), ("dev-secret-token", StatusCode::OK)] {
            let request = axum::http::Request::builder().uri("/api/auth/me")
                .header("Authorization", format!("Bearer {token}"))
                .body(axum::body::Body::empty()).unwrap();
            assert_eq!(app.clone().oneshot(request).await.unwrap().status(), expected);
        }
    }
}
