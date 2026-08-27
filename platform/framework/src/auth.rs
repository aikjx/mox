// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 认证授权 — JWT + RBAC + API Key，零配置默认启用

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,       // 用户ID
    pub tenant_id: String, // 租户ID
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub exp: usize,
    pub iat: usize,
}

/// 认证中间件状态
#[derive(Clone)]
pub struct AuthState {
    pub jwt_secret: Arc<String>,
    pub enabled: bool,
}

impl AuthState {
    pub fn new(jwt_secret: impl Into<String>, enabled: bool) -> Self {
        Self {
            jwt_secret: Arc::new(jwt_secret.into()),
            enabled,
        }
    }
}

/// 生成 JWT Token
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

/// 验证 JWT Token
pub fn verify_token(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

/// 认证中间件（从Authorization头提取JWT，注入Claims到request extensions）
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

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = verify_token(&state.jwt_secret, token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

/// 从request extensions提取Claims
pub fn extract_claims(req: &Request) -> Option<&Claims> {
    req.extensions().get::<Claims>()
}

/// RBAC权限检查
pub fn has_permission(claims: &Claims, permission: &str) -> bool {
    claims.permissions.contains(&permission.to_string())
        || claims.roles.contains(&"admin".to_string())
}

/// RBAC角色检查
pub fn has_role(claims: &Claims, role: &str) -> bool {
    claims.roles.contains(&role.to_string()) || claims.roles.contains(&"admin".to_string())
}
