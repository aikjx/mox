// =============================================================================
// 认证路由
// =============================================================================

use crate::app_state::AppState;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub tenant_id: Option<String>,
}

/// 登录响应
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: UserInfo,
}

/// 用户信息
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
}

/// 刷新令牌请求
#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// 登录（简化实现，实际应查询数据库验证用户）
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // 简化实现：验证密码长度，实际应查询数据库
    if req.password.len() < 8 {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "密码错误"})),
        );
    }

    let tenant_id = req.tenant_id.unwrap_or_else(|| "default".to_string());
    let user_id = format!("user-{}", req.username);
    let roles = vec!["user".to_string()];

    // 签发令牌
    match state.jwt.issue_token_pair(
        &user_id,
        &tenant_id,
        &req.username,
        roles.clone(),
    ) {
        Ok(tokens) => {
            let response = LoginResponse {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                token_type: tokens.token_type,
                expires_in: tokens.expires_in,
                user: UserInfo {
                    id: user_id,
                    username: req.username,
                    tenant_id,
                    roles,
                },
            };
            (StatusCode::OK, Json(serde_json::to_value(response).unwrap()))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("令牌签发失败: {}", e)})),
        ),
    }
}

/// 刷新令牌
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> impl IntoResponse {
    match state
        .jwt
        .refresh_access_token(&req.refresh_token, vec!["user".to_string()])
    {
        Ok(tokens) => (StatusCode::OK, Json(serde_json::to_value(tokens).unwrap())),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": format!("刷新失败: {}", e)})),
        ),
    }
}

/// 获取当前用户信息
pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let auth_header = match headers.get("authorization") {
        Some(h) => h.to_str().unwrap_or(""),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "缺少 Authorization header"})),
            );
        }
    };

    let token = match mox_auth_core::JwtManager::extract_bearer_token(auth_header) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": format!("{}", e)})),
            );
        }
    };

    match state.jwt.verify_access_token(&token) {
        Ok(claims) => {
            let user_info = UserInfo {
                id: claims.sub,
                username: claims.username,
                tenant_id: claims.tenant_id,
                roles: claims.roles,
            };
            (StatusCode::OK, Json(serde_json::to_value(user_info).unwrap()))
        }
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": format!("{}", e)})),
        ),
    }
}
