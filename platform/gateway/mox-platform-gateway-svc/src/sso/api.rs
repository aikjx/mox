//! SSO 单点登录 API 端点
//!
//! OAuth2/OIDC 授权码交换真实实现（reqwest 直连身份源 token_endpoint）；
//! SAML/CAS/LDAP 保持 501 并如实说明原因。

use crate::sso::*;
use axum::response::IntoResponse;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Response,
    Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// SSO 状态
pub struct SsoState {
    pub providers: Arc<RwLock<HashMap<String, SsoProvider>>>,
    pub sessions: Arc<RwLock<HashMap<String, SsoSession>>>,
    /// 发起登录时暂存的 state 凭证（回调校验 + 一次性消费，防 CSRF）
    pub pending: Arc<RwLock<HashMap<String, PendingAuth>>>,
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
            pending: Arc::new(RwLock::new(HashMap::new())),
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
    let result: Vec<Value> = protocols.iter().map(|(code, name, desc)| {
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
    list.sort_by_key(|a| a.sort_order);
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

/// POST /api/enterprise/sso/login —— 发起SSO登录（返回授权URL，并存 state 凭证）
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
            drop(providers);
            // 暂存 state → {provider_id, redirect_uri}，供回调一次性校验
            state.pending.write().await.insert(state_param.clone(), PendingAuth {
                provider_id: req.provider_id.clone(),
                redirect_uri: redirect_uri.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            Json(json!({
                "code": 0,
                "data": { "auth_url": auth_url, "state": state_param }
            })).into_response()
        }
        Some(_) => (StatusCode::BAD_REQUEST, Json(json!({ "code": 400, "message": "SSO提供商未启用" }))).into_response(),
        None => (StatusCode::NOT_FOUND, Json(json!({ "code": 404, "message": "SSO提供商不存在" }))).into_response(),
    }
}

fn err_resp(status: StatusCode, code: i64, message: &str) -> Response {
    (status, Json(json!({ "code": code, "message": message }))).into_response()
}

/// POST /api/enterprise/sso/callback —— OAuth2/OIDC 授权码换平台会话（真实协议实现）
pub async fn callback_handler(
    State(state): State<Arc<SsoState>>,
    Json(req): Json<SsoCallbackRequest>,
) -> Response {
    // 1) provider 必须存在且启用
    let providers = state.providers.read().await;
    let provider = match providers.get(&req.provider_id) {
        Some(p) if p.status == "enabled" => p.clone(),
        Some(_) => return err_resp(StatusCode::BAD_REQUEST, 400, "SSO提供商未启用"),
        None => return err_resp(StatusCode::NOT_FOUND, 404, "SSO提供商不存在"),
    };
    drop(providers);

    // 2) state 一次性消费 + 归属校验（防 CSRF/伪造回调）
    let pending = state.pending.write().await.remove(&req.state);
    let pending = match pending {
        Some(p) if p.provider_id == provider.provider_id => p,
        Some(_) => return err_resp(StatusCode::BAD_REQUEST, 400, "state 与提供商不匹配"),
        None => return err_resp(StatusCode::BAD_REQUEST, 400, "state 无效或已过期，请重新发起登录"),
    };

    // 3) 协议分支
    match provider.protocol.as_str() {
        "saml" | "cas" | "ldap" => err_resp(
            StatusCode::NOT_IMPLEMENTED, 501,
            "该协议的授权交换未实现（仅 OAuth2/OIDC 已接通外部身份源）",
        ),
        "oauth2" | "oidc" => exchange_oauth2_code(&state, &provider, &pending, &req.code).await,
        other => err_resp(StatusCode::NOT_IMPLEMENTED, 501, &format!("未知协议 {other}")),
    }
}

/// OAuth2/OIDC：用授权码向 token_endpoint 换 access_token，OIDC 再取 userinfo
async fn exchange_oauth2_code(
    state: &Arc<SsoState>,
    provider: &SsoProvider,
    pending: &PendingAuth,
    code: &str,
) -> Response {
    // 缺必填配置如实 422，不造桩成功
    if provider.client_id.is_empty() || provider.client_secret.is_empty() || provider.token_endpoint.is_empty() {
        return err_resp(
            StatusCode::UNPROCESSABLE_ENTITY, 422,
            "提供商缺少 client_id/client_secret/token_endpoint，无法换取令牌",
        );
    }

    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build() {
        Ok(c) => c,
        Err(e) => return err_resp(StatusCode::INTERNAL_SERVER_ERROR, 500, &format!("HTTP 客户端初始化失败: {e}")),
    };

    // POST token_endpoint，Basic 认证 + form body（RFC 6749 §4.1.3）
    let token_resp = client
        .post(&provider.token_endpoint)
        .basic_auth(&provider.client_id, Some(&provider.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &pending.redirect_uri),
        ])
        .send()
        .await;

    let token_resp = match token_resp {
        Ok(r) => r,
        Err(e) => return err_resp(StatusCode::BAD_GATEWAY, 502, &format!("连接身份源 token 端点失败: {e}")),
    };
    let status = token_resp.status();
    let body: Value = token_resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = body.get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("token 端点返回 {status}"));
        return err_resp(StatusCode::BAD_GATEWAY, 502, &format!("授权码交换失败: {msg}"));
    }

    let access_token = body.get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if access_token.is_empty() {
        return err_resp(StatusCode::BAD_GATEWAY, 502, "身份源未返回 access_token");
    }
    let refresh_token = body.get("refresh_token").and_then(|v| v.as_str()).map(|s| s.to_string());
    let expires_in = body.get("expires_in").and_then(|v| v.as_i64());

    // OIDC：再取 userinfo（sub/email/name）
    let mut ext_sub = String::new();
    let mut email: Option<String> = None;
    let mut name: Option<String> = None;
    if provider.protocol == "oidc" {
        if let Some(ui_url) = provider.userinfo_endpoint.as_ref().filter(|s| !s.is_empty()) {
            if let Ok(r) = client.get(ui_url).bearer_auth(&access_token).send().await {
                if let Ok(ui) = r.json::<Value>().await {
                    ext_sub = ui.get("sub").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    email = ui.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
                    name = ui.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }
        }
    }
    if ext_sub.is_empty() {
        ext_sub = uuid::Uuid::new_v4().simple().to_string();
    }

    // 建立平台会话（内存态，与既有 sessions map 一致）
    let session_id = format!("sso_sess_{}", uuid::Uuid::new_v4().simple());
    let session = SsoSession {
        session_id: session_id.clone(),
        provider_id: provider.provider_id.clone(),
        user_id: ext_sub.clone(),
        external_user_id: ext_sub.clone(),
        access_token: access_token.clone(),
        refresh_token: refresh_token.clone(),
        login_at: chrono::Utc::now().to_rfc3339(),
        expires_at: None,
        ip_address: None,
        user_agent: None,
        status: "active".to_string(),
    };
    state.sessions.write().await.insert(session_id.clone(), session);

    Json(json!({
        "code": 0,
        "message": "SSO 登录成功",
        "data": {
            "session_id": session_id,
            "access_token": access_token,
            "expires_in": expires_in,
            "user": { "external_id": ext_sub, "email": email, "name": name }
        }
    })).into_response()
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
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/protocols", get(list_protocols_handler))
        .route("/providers", get(list_providers_handler).post(create_provider_handler))
        .route("/providers/:id", get(get_provider_handler).put(update_provider_handler).delete(delete_provider_handler))
        .route("/login", post(login_handler))
        .route("/callback", post(callback_handler))
        .route("/logout", post(logout_handler))
}

#[cfg(test)]
mod callback_tests {
    use super::*;

    /// 伪造 state（未经过 /login）必须 400，且不建会话
    #[tokio::test]
    async fn forged_state_rejected_and_no_session() {
        let state = Arc::new(SsoState::new());
        let pid = {
            let mut ps = state.providers.write().await;
            let p = ps.values_mut().next().unwrap();
            p.status = "enabled".into();
            p.provider_id.clone()
        };
        let resp = callback_handler(State(state.clone()), Json(SsoCallbackRequest {
            provider_id: pid, code: "x".into(), state: "forged-state".into(),
        })).await;
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert!(state.sessions.read().await.is_empty());
    }

    /// 真实 state 但提供商缺 client_id/secret → 422，不造桩成功
    #[tokio::test]
    async fn missing_credentials_returns_422_not_mock_success() {
        let state = Arc::new(SsoState::new());
        let pid = {
            let mut ps = state.providers.write().await;
            let p = ps.values_mut().next().unwrap();
            p.status = "enabled".into();
            p.provider_id.clone()
        };
        // 经 login 合法拿 state
        let login = login_handler(State(state.clone()), Json(SsoLoginRequest {
            provider_id: pid.clone(), redirect_uri: None, state: None,
        })).await;
        assert_eq!(login.status(), StatusCode::OK);
        // 取出 state
        let bytes = axum::body::to_bytes(login.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        let state_param = body["data"]["state"].as_str().unwrap().to_string();

        let resp = callback_handler(State(state.clone()), Json(SsoCallbackRequest {
            provider_id: pid, code: "any".into(), state: state_param,
        })).await;
        // 预置模板 client_id/secret 为空 → 422（不访问外网）
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert!(state.sessions.read().await.is_empty());
    }

    /// SAML/CAS/LDAP 协议如实 501
    #[tokio::test]
    async fn saml_protocol_still_not_implemented() {
        let state = Arc::new(SsoState::new());
        // 造一个 enabled 的 saml provider（含凭据，走到协议分支）
        let pid: String = {
            let mut ps = state.providers.write().await;
            let mut p = ps.values().next().unwrap().clone();
            p.provider_id = "t_saml".into();
            p.protocol = "saml".into();
            p.status = "enabled".into();
            p.client_id = "cid".into();
            p.client_secret = "sec".into();
            p.token_endpoint = "https://idp.example/token".into();
            ps.insert("t_saml".into(), p);
            "t_saml".into()
        };
        let login = login_handler(State(state.clone()), Json(SsoLoginRequest {
            provider_id: pid.clone(), redirect_uri: None, state: Some("st".into()),
        })).await;
        assert_eq!(login.status(), StatusCode::OK);
        let resp = callback_handler(State(state), Json(SsoCallbackRequest {
            provider_id: pid, code: "x".into(), state: "st".into(),
        })).await;
        assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);
    }
}
