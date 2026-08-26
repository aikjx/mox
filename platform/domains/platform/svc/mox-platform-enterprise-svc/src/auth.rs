//! 企业级真源服务 — JWT 自实现（200 行内，零 framework 依赖）：
//! Claims = {sub,tenant_id,roles,permissions,exp,iat}
//! 使用 jsonwebtoken 9，默认 HS256

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Clone)]
pub struct AuthState {
    pub jwt_secret: String,
    pub enabled: bool,
}
impl AuthState {
    pub fn new(secret: impl Into<String>, enabled: bool) -> Self {
        Self { jwt_secret: secret.into(), enabled }
    }
}

pub fn generate_token(
    secret: &str,
    user_id: &str,
    tenant_id: &str,
    roles: Vec<String>,
    permissions: Vec<String>,
    expiry_secs: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = Claims {
        sub: user_id.into(),
        tenant_id: tenant_id.into(),
        roles,
        permissions,
        exp: now + expiry_secs as usize,
        iat: now,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
}

pub fn verify_token(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let d = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())?;
    Ok(d.claims)
}

pub async fn auth_middleware(
    axum::extract::State(state): axum::extract::State<AuthState>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if !state.enabled {
        return Ok(next.run(req).await);
    }
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let token = auth_header.strip_prefix("Bearer ").ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = verify_token(&state.jwt_secret, token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub fn extract_claims(req: &Request) -> Option<&Claims> {
    req.extensions().get::<Claims>()
}

/// 超级简化版的日志初始化：不破坏框架 logger 的前提下，给 tracing 默认挂 FMT。
pub fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};
    let _ = fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,enterprise_svc_lib=debug")))
        .with_target(true)
        .try_init();
}
