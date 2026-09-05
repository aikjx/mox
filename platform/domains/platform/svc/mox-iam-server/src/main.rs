// =============================================================================
// mox-iam-server: IAM/SSO 独立微服务入口
// =============================================================================
//
// 独立部署：cargo run -p mox-iam-server
// 默认端口：8103
// 健康检查：http://localhost:8103/health/live
//
// 基于 mox-auth-core 构建完整认证/授权 API：
//   - 注册 / 登录 / 刷新 Token / 验证 Token / 登出
//   - 用户管理（列表/详情/修改密码/禁用）
//   - JWT (access + refresh) / PBKDF2 密码哈希 / RBAC
// =============================================================================

use async_trait::async_trait;
use axum::{
    extract::{Extension, Path},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use clap::Parser;
use mox_auth_core::{
    jwt::{JwtConfig, JwtManager},
    password::PasswordManager,
    user::{User, UserStatus},
};
use mox_server_runtime::{Server, ServerConfig, ServiceModule};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// ── 共享状态 ─────────────────────────────────────────────────────────────────

struct IamState {
    jwt: JwtManager,
    pwd: PasswordManager,
    users: RwLock<HashMap<String, User>>, // key: username
}

impl IamState {
    fn new() -> Self {
        let jwt_config = JwtConfig {
            secret: std::env::var("MOX_JWT_SECRET")
                .unwrap_or_else(|_| "mox-default-dev-secret-change-in-production".to_string()),
            issuer: "mox-iam".to_string(),
            audience: "mox-services".to_string(),
            access_token_ttl: 3600,   // 1小时
            refresh_token_ttl: 604800, // 7天
        };
        let jwt = JwtManager::new(jwt_config).expect("JWT 初始化失败");
        let pwd = PasswordManager::new();
        Self { jwt, pwd, users: RwLock::new(HashMap::new()) }
    }
}

// ── 请求/响应类型 ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RegisterRequest {
    username: String,
    email: String,
    password: String,
    #[serde(default = "default_tenant")]
    tenant_id: String,
}
fn default_tenant() -> String { "default".to_string() }

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
    #[serde(default = "default_tenant")]
    tenant_id: String,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Debug, Deserialize)]
struct ChangePasswordRequest {
    old_password: String,
    new_password: String,
}

#[derive(Debug, Serialize)]
struct TokenPair {
    access_token: String,
    refresh_token: String,
    token_type: &'static str,
    expires_in: i64,
}

#[derive(Debug, Serialize)]
struct UserInfo {
    id: String,
    tenant_id: String,
    username: String,
    email: String,
    display_name: Option<String>,
    status: String,
    roles: Vec<String>,
    created_at: String,
    last_login_at: Option<String>,
}

impl From<&User> for UserInfo {
    fn from(u: &User) -> Self {
        Self {
            id: u.id.clone(),
            tenant_id: u.tenant_id.clone(),
            username: u.username.clone(),
            email: u.email.clone(),
            display_name: u.display_name.clone(),
            status: u.status.as_str().to_string(),
            roles: u.roles.iter().cloned().collect(),
            created_at: u.created_at.to_rfc3339(),
            last_login_at: u.last_login_at.map(|t| t.to_rfc3339()),
        }
    }
}

// ── 工具函数 ─────────────────────────────────────────────────────────────────

fn extract_bearer(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

fn auth_error(msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "error": msg.into() })))
}

fn bad_request(msg: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "error": msg.into() })))
}

fn ok_response(data: serde_json::Value) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(json!({ "success": true, "data": data })))
}

// ── 模块 ─────────────────────────────────────────────────────────────────────

struct IamModule {
    state: Arc<IamState>,
}

impl IamModule {
    fn new() -> Self {
        Self { state: Arc::new(IamState::new()) }
    }
}

#[async_trait]
impl ServiceModule for IamModule {
    fn name(&self) -> &str { "mox-iam-server" }
    fn version(&self) -> &str { env!("CARGO_PKG_VERSION") }

    async fn routes(&self, _config: &ServerConfig) -> Router {
        let state = self.state.clone();
        Router::new()
            // 认证
            .route("/api/v1/iam/auth/register", post(register_handler))
            .route("/api/v1/iam/auth/login", post(login_handler))
            .route("/api/v1/iam/auth/refresh", post(refresh_handler))
            .route("/api/v1/iam/auth/verify", get(verify_handler))
            // 用户
            .route("/api/v1/iam/users/me", get(me_handler))
            .route("/api/v1/iam/users", get(list_users_handler))
            .route("/api/v1/iam/users/{id}/password", put(change_password_handler))
            .route("/api/v1/iam/users/{id}/disable", post(disable_user_handler))
            .layer(Extension(state))
    }

    async fn init(&self, _config: &ServerConfig) -> Result<(), mox_server_runtime::RuntimeError> {
        tracing::info!("IAM/SSO 服务初始化完成（JWT + PBKDF2 + RBAC + 内存用户存储）");
        Ok(())
    }

    async fn ready_checks(&self) -> Vec<(&'static str, bool)> {
        vec![
            ("jwt_manager", true),
            ("password_manager", true),
            ("user_store", true),
        ]
    }
}

// ── 认证处理器 ───────────────────────────────────────────────────────────────

async fn register_handler(
    Extension(state): Extension<Arc<IamState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    if req.username.len() < 3 {
        return bad_request("用户名至少3个字符");
    }
    if req.password.len() < 8 {
        return bad_request("密码至少8个字符");
    }

    // 检查用户名是否已存在
    if state.users.read().contains_key(&req.username) {
        return (StatusCode::CONFLICT, Json(json!({ "success": false, "error": "用户名已存在" })));
    }

    // 哈希密码
    let hashed = match state.pwd.hash_password(&req.password) {
        Ok(h) => h.hash,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": e.to_string() }))),
    };

    // 创建用户
    let user = User::new(&req.tenant_id, &req.username, &req.email, hashed);
    let user_info = UserInfo::from(&user);
    state.users.write().insert(req.username.clone(), user);

    tracing::info!(user = %req.username, tenant = %req.tenant_id, "用户注册成功");
    (StatusCode::CREATED, Json(json!({ "success": true, "data": user_info })))
}

async fn login_handler(
    Extension(state): Extension<Arc<IamState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    let users = state.users.read();
    let user = match users.get(&req.username) {
        Some(u) => u.clone(),
        None => return auth_error("用户名或密码错误"),
    };
    drop(users);

    // 检查状态
    if !user.status.can_login() {
        return (StatusCode::FORBIDDEN, Json(json!({ "success": false, "error": format!("用户状态: {}", user.status.as_str()) })));
    }

    // 验证密码
    match state.pwd.verify_password(&req.password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return auth_error("用户名或密码错误"),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": e.to_string() }))),
    }

    // 签发 Token
    let roles: Vec<String> = user.roles.iter().cloned().collect();
    let tokens = match state.jwt.issue_token_pair(&user.id, &user.tenant_id, &user.username, roles) {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": e.to_string() }))),
    };

    // 更新最后登录时间
    if let Some(u) = state.users.write().get_mut(&req.username) {
        u.last_login_at = Some(chrono::Utc::now());
    }

    tracing::info!(user = %req.username, "用户登录成功");
    ok_response(json!(TokenPair {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: "Bearer",
        expires_in: tokens.expires_in,
    }))
}

async fn refresh_handler(
    Extension(state): Extension<Arc<IamState>>,
    Json(req): Json<RefreshRequest>,
) -> impl IntoResponse {
    // 先验证 refresh token 获取角色
    let claims = match state.jwt.verify_refresh_token(&req.refresh_token) {
        Ok(c) => c,
        Err(_) => return auth_error("刷新令牌无效或已过期"),
    };
    let tokens = match state.jwt.refresh_access_token(&req.refresh_token, claims.roles) {
        Ok(t) => t,
        Err(_) => return auth_error("刷新令牌无效或已过期"),
    };
    ok_response(json!(TokenPair {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        token_type: "Bearer",
        expires_in: tokens.expires_in,
    }))
}

async fn verify_handler(
    Extension(state): Extension<Arc<IamState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return auth_error("缺少 Authorization 头"),
    };
    match state.jwt.verify_access_token(&token) {
        Ok(claims) => ok_response(json!({
            "valid": true,
            "user_id": claims.sub,
            "tenant_id": claims.tenant_id,
            "username": claims.username,
            "roles": claims.roles,
            "expires_at": claims.exp,
        })),
        Err(_) => (StatusCode::UNAUTHORIZED, Json(json!({ "valid": false, "error": "令牌无效或已过期" }))),
    }
}

// ── 用户处理器 ───────────────────────────────────────────────────────────────

async fn me_handler(
    Extension(state): Extension<Arc<IamState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return auth_error("缺少 Authorization 头"),
    };
    let claims = match state.jwt.verify_access_token(&token) {
        Ok(c) => c,
        Err(_) => return auth_error("令牌无效或已过期"),
    };
    let users = state.users.read();
    let user = match users.values().find(|u| u.id == claims.sub) {
        Some(u) => UserInfo::from(u),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "error": "用户不存在" }))),
    };
    ok_response(json!(user))
}

async fn list_users_handler(
    Extension(state): Extension<Arc<IamState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    // 简单鉴权：需要有效 token
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return auth_error("缺少 Authorization 头"),
    };
    if state.jwt.verify_access_token(&token).is_err() {
        return auth_error("令牌无效或已过期");
    }
    let users: Vec<UserInfo> = state.users.read().values().map(UserInfo::from).collect();
    ok_response(json!({ "users": users, "total": users.len() }))
}

async fn change_password_handler(
    Extension(state): Extension<Arc<IamState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return auth_error("缺少 Authorization 头"),
    };
    let claims = match state.jwt.verify_access_token(&token) {
        Ok(c) => c,
        Err(_) => return auth_error("令牌无效或已过期"),
    };
    if claims.sub != id {
        return (StatusCode::FORBIDDEN, Json(json!({ "success": false, "error": "只能修改自己的密码" })));
    }
    if req.new_password.len() < 8 {
        return bad_request("新密码至少8个字符");
    }

    let mut users = state.users.write();
    let user = match users.values_mut().find(|u| u.id == id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "error": "用户不存在" }))),
    };

    // 验证旧密码
    match state.pwd.verify_password(&req.old_password, &user.password_hash) {
        Ok(true) => {}
        Ok(false) => return bad_request("旧密码错误"),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": e.to_string() }))),
    }

    // 设置新密码
    user.password_hash = match state.pwd.hash_password(&req.new_password) {
        Ok(h) => h.hash,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "error": e.to_string() }))),
    };

    tracing::info!(user_id = %id, "用户密码修改成功");
    ok_response(json!({ "status": "password_changed" }))
}

async fn disable_user_handler(
    Extension(state): Extension<Arc<IamState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return auth_error("缺少 Authorization 头"),
    };
    if state.jwt.verify_access_token(&token).is_err() {
        return auth_error("令牌无效或已过期");
    }
    let mut users = state.users.write();
    let user = match users.values_mut().find(|u| u.id == id) {
        Some(u) => u,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "error": "用户不存在" }))),
    };
    user.status = UserStatus::Disabled;
    tracing::info!(user_id = %id, "用户已禁用");
    ok_response(json!({ "status": "disabled", "user_id": id }))
}

// ── CLI ─────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(name = "mox-iam-server", about = "MOX IAM/SSO 独立微服务", version)]
struct Cli {
    #[arg(short, long, default_value = "config/iam-server.toml")]
    config: PathBuf,
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let mut config = if cli.config.exists() {
        ServerConfig::from_file(&cli.config)?
    } else {
        ServerConfig::default()
    };
    config.apply_env_overrides();
    if let Some(port) = cli.port { config.server.port = port; }
    if config.server.port == 8080 { config.server.port = 8103; }

    let module = IamModule::new();
    Server::new(Box::new(module), config).run().await?;
    Ok(())
}
