// ====================================================================
// system/auth_session.rs — 认证会话端点（登录 / 注册 / 刷新令牌）
// ====================================================================
// 网关原生承载 IAM 会话（种子 T001 租户 + admin 用户），签发 HS256 JWT：
// - POST /api/auth/login   {username, password, tenant_id} → {access_token, refresh_token, user}
// - POST /api/auth/register {username, email, password, tenant_id} → 新建用户（密码 SHA-256 落库）
// - POST /api/auth/refresh {refresh_token} → 校验刷新令牌并续签 access_token
//
// 密码口径：入库统一为 SHA-256 十六进制；历史明文存量（管理员控制台早期写入）
// 通过 verify_password 的明文兜底兼容，登录成功后建议重置。
// JWT 口径与 auth.rs validate_token 对齐：HS256 + base64url(no pad)，iss=mox-platform。

use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, ok, resolve_tenant};
use axum::{extract::State, Json};
use base64::Engine;
use hmac::{Hmac, Mac};
use mox_api_protocol::{ApiResponse, api_error, api_ok};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const ACCESS_TTL_SECS: i64 = 8 * 3600;       // 访问令牌 8 小时
const REFRESH_TTL_SECS: i64 = 7 * 24 * 3600; // 刷新令牌 7 天

#[derive(Deserialize)]
pub(crate) struct LoginReq {
    username: Option<String>,
    password: Option<String>,
    tenant_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RegisterReq {
    username: Option<String>,
    email: Option<String>,
    password: Option<String>,
    tenant_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RefreshReq {
    refresh_token: Option<String>,
}

// ======================== 密码工具 ========================

/// SHA-256 十六进制摘要（入库口径）
pub(crate) fn hash_password(pwd: &str) -> String {
    let mut h = Sha256::new();
    h.update(pwd.as_bytes());
    hex_lower(&h.finalize().to_vec())
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// 口令校验：优先 SHA-256 十六进制恒定时间比较；兼容历史明文存量。
fn verify_password(pwd: &str, stored: &str) -> bool {
    if stored.is_empty() {
        return false;
    }
    if stored.len() == 64 && stored.chars().all(|c| c.is_ascii_hexdigit()) {
        let digest = hash_password(pwd);
        let a = digest.as_bytes();
        let b = stored.as_bytes();
        let mut diff = 0u8;
        for i in 0..a.len() {
            diff |= a[i] ^ b[i];
        }
        return diff == 0;
    }
    // TODO(security): 存量明文迁移后移除兜底
    stored == pwd
}

// ======================== JWT 工具 ========================

fn b64u(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn b64u_decode(data: &str) -> Option<Vec<u8>> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data).ok()
}

/// 签发 HS256 JWT（与 auth.rs validate_token 的验签口径严格一致）
fn sign_jwt(secret: &str, issuer: &str, claims: Value, ttl_secs: i64) -> Option<String> {
    let mut payload = claims.as_object()?.clone();
    payload.insert("iss".into(), json!(issuer));
    payload.insert("iat".into(), json!(chrono::Utc::now().timestamp()));
    payload.insert("exp".into(), json!(chrono::Utc::now().timestamp() + ttl_secs));
    let header = json!({"alg":"HS256","typ":"JWT"});
    let h = b64u(&serde_json::to_vec(&header).ok()?);
    let p = b64u(&serde_json::to_vec(&Value::Object(payload)).ok()?);
    let signing_input = format!("{}.{}", h, p);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(signing_input.as_bytes());
    let sig = b64u(&mac.finalize().into_bytes());
    Some(format!("{}.{}", signing_input, sig))
}

/// 校验 JWT 签名 + exp，返回 payload（用于 refresh 端点）
fn verify_jwt(secret: &str, token: &str) -> Option<Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let sig_bytes = b64u_decode(parts[2])?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(signing_input.as_bytes());
    if mac.verify_slice(&sig_bytes).is_err() {
        return None;
    }
    let payload: Value = serde_json::from_slice(&b64u_decode(parts[1])?).ok()?;
    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        if exp < chrono::Utc::now().timestamp() {
            return None;
        }
    }
    Some(payload)
}

// ======================== 通用装配 ========================

fn tenant_input_of(raw: Option<String>) -> String {
    let t = raw.unwrap_or_default().trim().to_string();
    if t.is_empty() || t == "default" {
        DEFAULT_TENANT.to_string()
    } else {
        t
    }
}

fn roles_of(s: &GatewayState, tenant_id: &str, user: &mox_platform_iam_core::IamUser) -> Vec<String> {
    let mut roles: Vec<String> = s
        .iam
        .get_user_roles(tenant_id, &user.user_id)
        .unwrap_or_default()
        .iter()
        .map(|r| r.role_code.clone())
        .collect();
    // seed 每次启动会重复插入角色绑定，这里去重保持令牌/响应整洁
    roles.sort();
    roles.dedup();
    if user.is_superuser == 1 && !roles.iter().any(|r| r == "admin") {
        roles.push("admin".to_string());
    }
    roles
}

fn user_json(user: &mox_platform_iam_core::IamUser, roles: Vec<String>) -> Value {
    json!({
        "id": user.user_id,
        "username": user.username,
        "email": user.email,
        "tenant_id": user.tenant_id,
        "roles": roles,
        "enabled": user.user_status == "active",
        "is_superuser": user.is_superuser == 1,
        "created_at": user.created_at,
    })
}

fn issue_tokens(
    s: &GatewayState,
    user: &mox_platform_iam_core::IamUser,
    roles: &[String],
) -> Result<(String, String), String> {
    let secret = &s.config.auth.jwt_secret;
    let issuer = &s.config.auth.token_issuer;
    let access_claims = json!({
        "sub": user.user_id,
        "username": user.username,
        "email": user.email,
        "tenant_id": user.tenant_id,
        "roles": roles,
    });
    let access = sign_jwt(secret, issuer, access_claims, ACCESS_TTL_SECS)
        .ok_or_else(|| "访问令牌签发失败".to_string())?;
    let refresh_claims = json!({
        "sub": user.user_id,
        "username": user.username,
        "tenant_id": user.tenant_id,
        "typ": "refresh",
    });
    let refresh = sign_jwt(secret, issuer, refresh_claims, REFRESH_TTL_SECS)
        .ok_or_else(|| "刷新令牌签发失败".to_string())?;
    Ok((access, refresh))
}

// ======================== 处理器 ========================

/// POST /api/auth/login
pub(crate) async fn login_handler(
    State(s): State<GatewayState>,
    Json(body): Json<LoginReq>,
) -> ApiResponse<Value> {
    let username = body.username.unwrap_or_default().trim().to_string();
    let password = body.password.unwrap_or_default();
    if username.is_empty() || password.is_empty() {
        return api_error(400, "用户名和密码不能为空");
    }
    if password.len() < 8 {
        return api_error(400, "密码长度至少 8 个字符");
    }

    let tenant_input = tenant_input_of(body.tenant_id);
    let tenant_id = match resolve_tenant(&s, &tenant_input) {
        Ok(t) => t,
        Err(_) => DEFAULT_TENANT.to_string(),
    };

    let user = match s.iam.find_user_by_tenant_username(&tenant_id, &username) {
        Some(u) => u,
        None => return api_error(401, "用户名或密码错误"),
    };
    if user.user_status != "active" {
        return api_error(403, "账号已停用，请联系管理员");
    }

    let stored = user.password_hash.as_deref().unwrap_or("");
    if stored.is_empty() {
        // 首次引导：种子用户（admin 超级管理员）密码为 NULL，dev 模式下首次登录即写入初始密码
        if s.config.auth.dev_mode && user.is_superuser == 1 {
            let _ = s.iam.reset_password(&user.user_id, &hash_password(&password));
        } else {
            return api_error(403, "用户尚未设置密码，请联系管理员重置");
        }
    } else if !verify_password(&password, stored) {
        return api_error(401, "用户名或密码错误");
    }

    let roles = roles_of(&s, &tenant_id, &user);
    let (access_token, refresh_token) = match issue_tokens(&s, &user, &roles) {
        Ok(t) => t,
        Err(e) => return api_error(500, &e),
    };

    // 登录日志（尽力而为，失败不阻断登录）
    let _ = s.iam.create_login_log(
        &tenant_id,
        Some(&user.username),
        Some("127.0.0.1"),
        None,
        None,
        Some("success"),
        Some("密码登录"),
    );

    ok(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TTL_SECS,
        "user": user_json(&user, roles),
    }))
}

/// POST /api/auth/register
pub(crate) async fn register_handler(
    State(s): State<GatewayState>,
    Json(body): Json<RegisterReq>,
) -> ApiResponse<Value> {
    let username = body.username.unwrap_or_default().trim().to_string();
    let email = body.email.unwrap_or_default().trim().to_string();
    let password = body.password.unwrap_or_default();

    if username.len() < 3 || username.len() > 50 {
        return api_error(400, "用户名长度需在 3-50 个字符");
    }
    if password.len() < 8 {
        return api_error(400, "密码长度至少 8 个字符");
    }
    if !email.contains('@') {
        return api_error(400, "请输入有效邮箱地址");
    }

    let tenant_input = tenant_input_of(body.tenant_id);
    let tenant_id = match resolve_tenant(&s, &tenant_input) {
        Ok(t) => t,
        Err(_) => DEFAULT_TENANT.to_string(),
    };

    if s.iam.find_user_by_tenant_username(&tenant_id, &username).is_some() {
        return api_error(409, "用户名已存在");
    }

    let user_code = format!("U{}", chrono::Utc::now().timestamp_millis());
    let user = match s.iam.create_user(
        &tenant_id,
        &user_code,
        &username,
        Some(&username),
        Some(&hash_password(&password)),
        None,
        false,
    ) {
        Ok(u) => u,
        Err(e) => return api_error(500, &format!("创建用户失败: {e}")),
    };

    // 分配默认角色 tenant_user（找不到角色时不阻断注册）
    if let Ok(roles) = s.iam.list_roles(&tenant_id) {
        if let Some(r) = roles.iter().find(|r| r.role_code == "tenant_user") {
            let _ = s.iam.assign_role_to_user(&tenant_id, &user.user_id, &r.role_id, None);
        }
    }

    api_ok(json!({
        "id": user.user_id,
        "username": user.username,
        "email": user.email,
        "tenant_id": user.tenant_id,
    }))
}

/// POST /api/auth/refresh
pub(crate) async fn refresh_handler(
    State(s): State<GatewayState>,
    Json(body): Json<RefreshReq>,
) -> ApiResponse<Value> {
    let token = body.refresh_token.unwrap_or_default();
    if token.is_empty() {
        return api_error(400, "缺少 refresh_token");
    }

    let payload = match verify_jwt(&s.config.auth.jwt_secret, &token) {
        Some(p) => p,
        None => return api_error(401, "刷新令牌无效或已过期"),
    };
    if payload.get("typ").and_then(|v| v.as_str()) != Some("refresh") {
        return api_error(401, "令牌类型错误，请重新登录");
    }

    let sub = payload.get("sub").and_then(|v| v.as_str()).unwrap_or_default();
    let user = match s.iam.get_user(sub) {
        Ok(Some(u)) => u,
        _ => return api_error(401, "用户不存在，请重新登录"),
    };
    if user.user_status != "active" {
        return api_error(403, "账号已停用");
    }

    let roles = roles_of(&s, &user.tenant_id, &user);
    let (access_token, _) = match issue_tokens(&s, &user, &roles) {
        Ok(t) => t,
        Err(e) => return api_error(500, &e),
    };

    api_ok(json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TTL_SECS,
        "user": user_json(&user, roles),
    }))
}
