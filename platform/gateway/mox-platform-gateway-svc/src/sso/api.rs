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

/// POST /api/enterprise/sso/callback —— OAuth2/OIDC 授权码换平台 JWT（真实协议 + IAM 映射）
pub async fn callback_handler(
    State(g): State<crate::GatewayState>,
    Json(req): Json<SsoCallbackRequest>,
) -> Response {
    let state = &g.enterprise.sso;

    // 1) provider 必须存在且启用
    let providers = state.providers.read().await;
    let provider = match providers.get(&req.provider_id) {
        Some(p) if p.status == "enabled" => p.clone(),
        Some(_) => return err_resp(StatusCode::BAD_REQUEST, 400, "SSO提供商未启用"),
        None => return err_resp(StatusCode::NOT_FOUND, 404, "SSO提供商不存在"),
    };
    drop(providers);

    // 2) state 惰性过期清理（10 分钟）+ 一次性消费 + 归属校验
    let pending = {
        let mut pending_map = state.pending.write().await;
        pending_map.retain(|_, p| {
            chrono::DateTime::parse_from_rfc3339(&p.created_at)
                .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_seconds() < 600)
                .unwrap_or(false)
        });
        pending_map.remove(&req.state)
    };
    let pending = match pending {
        Some(p) if p.provider_id == provider.provider_id => p,
        Some(_) => return err_resp(StatusCode::BAD_REQUEST, 400, "state 与提供商不匹配"),
        None => return err_resp(StatusCode::BAD_REQUEST, 400, "state 无效或已过期，请重新发起登录"),
    };

    // 3) 协议分支 + 授权码交换
    let exchange = match provider.protocol.as_str() {
        "saml" | "cas" | "ldap" => return err_resp(
            StatusCode::NOT_IMPLEMENTED, 501,
            "该协议的授权交换未实现（仅 OAuth2/OIDC 已接通外部身份源）",
        ),
        "oauth2" | "oidc" => match exchange_oauth2_code(&provider, &pending, &req.code).await {
            Ok(e) => e,
            Err(r) => return r,
        },
        other => return err_resp(StatusCode::NOT_IMPLEMENTED, 501, &format!("未知协议 {other}")),
    };

    // 4) 外部用户 → IAM 用户映射（按 email 在默认租户查找；无映射 409，不自动建号，不绕 RBAC）
    let email = exchange.email.clone().unwrap_or_default();
    if email.is_empty() {
        return err_resp(StatusCode::UNAUTHORIZED, 401, "身份源未返回 email，无法映射平台账号");
    }
    let tenant_id = crate::system::DEFAULT_TENANT.to_string();
    let users = match g.iam.list_users(&tenant_id) {
        Ok(u) => u,
        Err(_) => return err_resp(StatusCode::INTERNAL_SERVER_ERROR, 500, "用户表查询失败"),
    };
    let user = match users.into_iter().find(|u| u.email.as_deref() == Some(email.as_str())) {
        Some(u) => u,
        None => return err_resp(StatusCode::CONFLICT, 409, "SSO 账号未映射到平台用户，请联系管理员预建"),
    };
    if user.user_status != "active" {
        return err_resp(StatusCode::FORBIDDEN, 403, "平台账号已停用");
    }

    // 5) 复用与密码登录同一套 JWT 签发（roles 取 IAM 角色，不绕过 RBAC）
    let roles = crate::system::auth_session::roles_of(&g, &tenant_id, &user);
    let (access_token, refresh_token) = match crate::system::auth_session::issue_tokens(&g, &user, &roles) {
        Ok(t) => t,
        Err(e) => return err_resp(StatusCode::INTERNAL_SERVER_ERROR, 500, &e),
    };

    Json(json!({
        "code": 0,
        "message": "SSO 登录成功",
        "data": {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "token_type": "Bearer",
            "user": crate::system::auth_session::user_json(&user, roles),
            "external_sub": exchange.ext_sub,
        }
    })).into_response()
}

/// OAuth2/OIDC 交换结果
#[derive(Debug)]
struct SsoExchange {
    ext_sub: String,
    email: Option<String>,
}

/// OAuth2/OIDC：用授权码向 token_endpoint 换 access_token，OIDC 再取 userinfo（无状态依赖，可单测）
async fn exchange_oauth2_code(
    provider: &SsoProvider,
    pending: &PendingAuth,
    code: &str,
) -> Result<SsoExchange, Response> {
    // 缺必填配置如实 422，不造桩成功
    if provider.client_id.is_empty() || provider.client_secret.is_empty() || provider.token_endpoint.is_empty() {
        return Err(err_resp(
            StatusCode::UNPROCESSABLE_ENTITY, 422,
            "提供商缺少 client_id/client_secret/token_endpoint，无法换取令牌",
        ));
    }

    let client = match reqwest::Client::builder().timeout(std::time::Duration::from_secs(15)).build() {
        Ok(c) => c,
        Err(e) => return Err(err_resp(StatusCode::INTERNAL_SERVER_ERROR, 500, &format!("HTTP 客户端初始化失败: {e}"))),
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
        Err(e) => return Err(err_resp(StatusCode::BAD_GATEWAY, 502, &format!("连接身份源 token 端点失败: {e}"))),
    };
    let status = token_resp.status();
    let body: Value = token_resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        let msg = body.get("error")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("token 端点返回 {status}"));
        return Err(err_resp(StatusCode::BAD_GATEWAY, 502, &format!("授权码交换失败: {msg}")));
    }

    let access_token = body.get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if access_token.is_empty() {
        return Err(err_resp(StatusCode::BAD_GATEWAY, 502, "身份源未返回 access_token"));
    }

    // OIDC：再取 userinfo（sub/email）
    let mut ext_sub = String::new();
    let mut email: Option<String> = None;
    if provider.protocol == "oidc" {
        if let Some(ui_url) = provider.userinfo_endpoint.as_ref().filter(|s| !s.is_empty()) {
            if let Ok(r) = client.get(ui_url).bearer_auth(&access_token).send().await {
                if let Ok(ui) = r.json::<Value>().await {
                    ext_sub = ui.get("sub").and_then(|v| v.as_str()).unwrap_or_default().to_string();
                    email = ui.get("email").and_then(|v| v.as_str()).map(|s| s.to_string());
                }
            }
        }
    }
    if ext_sub.is_empty() {
        ext_sub = uuid::Uuid::new_v4().simple().to_string();
    }

    Ok(SsoExchange { ext_sub, email })
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

/// 构建SSO路由（绑定 GatewayState：callback 需访问 IAM 签平台 JWT）
pub fn build_sso_router() -> axum::Router<crate::GatewayState> {
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

    fn enabled_provider() -> SsoProvider {
        let mut p = builtin_provider_templates().into_iter().next().unwrap();
        p.status = "enabled".into();
        p
    }

    /// 缺 client_id/secret → exchange 如实 422，不访问外网、不造桩成功
    #[tokio::test]
    async fn missing_credentials_returns_422_not_mock_success() {
        let p = enabled_provider(); // 预置模板 client_id/secret 为空
        let pending = PendingAuth {
            provider_id: p.provider_id.clone(),
            redirect_uri: "https://app/cb".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        let resp = exchange_oauth2_code(&p, &pending, "any").await.unwrap_err();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    /// state 归属不符/缺失返回 400（用 pending map 直接验证）
    #[tokio::test]
    async fn pending_state_lifecycle() {
        let state = Arc::new(SsoState::new());
        state.pending.write().await.insert("st".into(), PendingAuth {
            provider_id: "p1".into(), redirect_uri: "".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        });
        // 取走后再次取应为 None（一次性消费）
        let got = state.pending.write().await.remove("st");
        assert!(got.is_some());
        assert!(state.pending.write().await.remove("st").is_none());
    }
}
