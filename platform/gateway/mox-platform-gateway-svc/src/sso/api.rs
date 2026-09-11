//! SSO 单点登录 API 端点
//!
//! 支持 OAuth2 / OIDC / SAML / CAS / LDAP 等协议的登录、回调、登出、用户信息

use crate::sso::*;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SSO 状态
pub struct SsoState {
    pub providers: Arc<RwLock<HashMap<String, SsoProvider>>>,
    pub sessions: Arc<RwLock<HashMap<String, SsoSession>>>,
}

impl SsoState {
    pub fn new() -> Self {
        let mut providers = HashMap::new();
        // 预置3个模板提供商
        for tpl in builtin_provider_templates() {
            providers.insert(tpl.provider_id.clone(), tpl);
        }
        Self {
            providers: Arc::new(RwLock::new(providers)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for SsoState {
    fn default() -> Self {
        Self::new()
    }
}

/// GET /api/enterprise/sso/protocols —— 获取支持的协议列表
pub async fn list_protocols_handler() -> Response {
    let protocols = supported_protocols();
    let result: Vec<serde_json::Value> = protocols.iter().map(|(code, name, desc)| {
        json!({ "code": code, "name": name, "description": desc })
    }).collect();
    Json(json!({ "code": 0, "data": result, "total": result.len() })).into_response()
}

/// GET /api/enterprise/sso/providers —— 获取SSO提供商列表
pub async fn list_providers_handler(
    State(state): State<Arc<SsoState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    let providers = state.providers.read().await;
    let mut list: Vec<&SsoProvider> = providers.values().collect();
    if let Some(status) = params.get("status") {
        list.retain(|p| p.status == *status);
    }
    if let Some(proto) = params.get("protocol") {
        list.retain(|p| p.protocol == *proto);
    }
    list.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
    Json(json!({ "code": 0, "data": list, "total": list.len() })).into_response()
}

/// GET /api/enterprise/sso/providers/:id —— 获取SSO提供商详情
pub async fn get_provider_handler(
    State(state): State<Arc<SsoState>>,
    Path(id): Path<String>,
) -> Response {
    let providers = state.providers.read().await;
    match providers.get(&id) {
        Some(p) => Json(json!({ "code": 0, "data": p })).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response(),
    }
}

/// POST /api/enterprise/sso/providers —— 创建SSO提供商
pub async fn create_provider_handler(
    State(state): State<Arc<SsoState>>,
    Json(req): Json<CreateSsoProviderRequest>,
) -> Response {
    let provider_id = format!("sso_{}", uuid::Uuid::new_v4().simple());
    let now = chrono::Utc::now().to_rfc3339();
    let provider = SsoProvider {
        provider_id: provider_id.clone(),
        tenant_id: "default".to_string(),
        name: req.name,
        protocol: req.protocol,
        status: "disabled".to_string(),
        client_id: req.client_id,
        client_secret: req.client_secret,
        auth_endpoint: req.auth_endpoint,
        token_endpoint: req.token_endpoint,
        userinfo_endpoint: req.userinfo_endpoint,
        logout_endpoint: req.logout_endpoint,
        redirect_uri: req.redirect_uri,
        scopes: req.scopes.unwrap_or_default(),
        field_mapping: req.field_mapping.unwrap_or_default(),
        extra_config: req.extra_config.unwrap_or_default(),
        is_default: req.is_default.unwrap_or(false),
        sort_order: req.sort_order.unwrap_or(0),
        created_at: now.clone(),
        updated_at: now,
    };
    state.providers.write().await.insert(provider_id.clone(), provider);
    Json(json!({ "code": 0, "message": "SSO提供商创建成功", "data": { "provider_id": provider_id } })).into_response()
}

/// PUT /api/enterprise/sso/providers/:id —— 更新SSO提供商
pub async fn update_provider_handler(
    State(state): State<Arc<SsoState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateSsoProviderRequest>,
) -> Response {
    let mut providers = state.providers.write().await;
    match providers.get_mut(&id) {
        Some(p) => {
            if let Some(name) = req.name { p.name = name; }
            if let Some(status) = req.status { p.status = status; }
            if let Some(cid) = req.client_id { p.client_id = cid; }
            if let Some(cs) = req.client_secret { p.client_secret = cs; }
            if let Some(ae) = req.auth_endpoint { p.auth_endpoint = ae; }
            if let Some(te) = req.token_endpoint { p.token_endpoint = te; }
            if let Some(ue) = req.userinfo_endpoint { p.userinfo_endpoint = Some(ue); }
            if let Some(le) = req.logout_endpoint { p.logout_endpoint = Some(le); }
            if let Some(ru) = req.redirect_uri { p.redirect_uri = ru; }
            if let Some(scopes) = req.scopes { p.scopes = scopes; }
            if let Some(fm) = req.field_mapping { p.field_mapping = fm; }
            if let Some(ec) = req.extra_config { p.extra_config = ec; }
            if let Some(id_default) = req.is_default { p.is_default = id_default; }
            if let Some(so) = req.sort_order { p.sort_order = so; }
            p.updated_at = chrono::Utc::now().to_rfc3339();
            Json(json!({ "code": 0, "message": "SSO提供商更新成功" })).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response(),
    }
}

/// DELETE /api/enterprise/sso/providers/:id —— 删除SSO提供商
pub async fn delete_provider_handler(
    State(state): State<Arc<SsoState>>,
    Path(id): Path<String>,
) -> Response {
    let mut providers = state.providers.write().await;
    if providers.remove(&id).is_some() {
        Json(json!({ "code": 0, "message": "SSO提供商删除成功" })).into_response()
    } else {
        (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response()
    }
}

/// POST /api/enterprise/sso/login —— 发起SSO登录（返回授权URL）
pub async fn login_handler(
    State(state): State<Arc<SsoState>>,
    Json(req): Json<SsoLoginRequest>,
) -> Response {
    let providers = state.providers.read().await;
    match providers.get(&req.provider_id) {
        Some(p) if p.status == "enabled" => {
            let state_param = req.state.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let redirect_uri = req.redirect_uri.unwrap_or_else(|| p.redirect_uri.clone());
            let scope = p.scopes.join(" ");
            let auth_url = format!(
                "{}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
                p.auth_endpoint, p.client_id, redirect_uri, scope, state_param
            );
            Json(json!({
                "code": 0,
                "data": { "auth_url": auth_url, "state": state_param }
            })).into_response()
        }
        Some(_) => (StatusCode::BAD_REQUEST, Json(json!({ "code": 400, "message": "SSO提供商未启用" }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response(),
    }
}

/// POST /api/enterprise/sso/callback —— SSO登录回调（处理授权码）
pub async fn callback_handler(
    State(state): State<Arc<SsoState>>,
    Json(req): Json<SsoCallbackRequest>,
) -> Response {
    let providers = state.providers.read().await;
    match providers.get(&req.provider_id) {
        Some(p) if p.status == "enabled" => {
            // 实际实现中需要：
            // 1. 用授权码交换token（调用token_endpoint）
            // 2. 用access_token获取用户信息（调用userinfo_endpoint）
            // 3. 映射字段，查找或创建本地用户
            // 4. 创建本地会话，返回JWT token
            // 当前为框架实现，返回模拟结果
            let session_id = format!("sess_{}", uuid::Uuid::new_v4().simple());
            let now = chrono::Utc::now().to_rfc3339();
            let session = SsoSession {
                session_id: session_id.clone(),
                provider_id: req.provider_id.clone(),
                user_id: "user_001".to_string(),
                external_user_id: "external_001".to_string(),
                access_token: "mock_access_token".to_string(),
                refresh_token: Some("mock_refresh_token".to_string()),
                login_at: now.clone(),
                expires_at: Some((chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339()),
                ip_address: None,
                user_agent: None,
                status: "active".to_string(),
            };
            state.sessions.write().await.insert(session_id.clone(), session);
            Json(json!({
                "code": 0,
                "message": "登录成功",
                "data": {
                    "session_id": session_id,
                    "token": "mock_jwt_token",
                    "user": { "user_id": "user_001", "username": "demo_user" }
                }
            })).into_response()
        }
        Some(_) => (StatusCode::BAD_REQUEST, Json(json!({ "code": 400, "message": "SSO提供商未启用" }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response(),
    }
}

/// POST /api/enterprise/sso/logout —— SSO登出
pub async fn logout_handler(
    State(state): State<Arc<SsoState>>,
    Json(params): Json<HashMap<String, String>>,
) -> Response {
    if let Some(session_id) = params.get("session_id") {
        let mut sessions = state.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.status = "revoked".to_string();
        }
    }
    Json(json!({ "code": 0, "message": "登出成功" })).into_response()
}

/// 构建SSO路由（泛型版本）
pub fn build_sso_router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
    Arc<SsoState>: axum::extract::FromRef<S>,
{
    use axum::routing::{get, post, delete};

    axum::Router::new()
        .route("/protocols", get(list_protocols_handler))
        .route("/providers", get(list_providers_handler).post(create_provider_handler))
        .route("/providers/:id", get(get_provider_handler).put(update_provider_handler).delete(delete_provider_handler))
        .route("/login", post(login_handler))
        .route("/callback", post(callback_handler))
        .route("/logout", post(logout_handler))
}
