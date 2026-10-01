// ====================================================================
// system/mfa.rs — TOTP 多因素认证（RFC 6238，HMAC-SHA1 6 位码，30s 窗口）
// ====================================================================
// 真实实现：
// - TOTP：HMAC-SHA1(key=base32 secret, counter=unix_time/30)，动态截断取 6 位数字；
//   允许 ±1 窗口防时钟漂移。
// - 恢复码：10 个一次性 16 进制短码，SHA-256 落库，用后即焚。
// - 用户 MFA 状态：内存态（HashMap<user_id, MfaRecord>），重启后需重新绑定。
//
// 流程：
// - /api/auth/login：密码校验通过后若该用户已启用 MFA → 返回 mfa_required + mfa_token（短时 JWT，无业务 claims）
// - /api/auth/mfa/verify：{mfa_token, code} → 校验 TOTP/恢复码 → 签正式 JWT
// - /api/auth/mfa/bind：{username,password} 重新认证 → 生成 secret + otpauth:// URI（不落 enabled）
// - /api/auth/mfa/confirm：{username,password,code} → 校验绑定码成功后启用，返回恢复码明文（仅此一次）
// - /api/auth/mfa/unbind：{username,password,code} → 关闭 MFA

use crate::GatewayState;
use crate::system::auth_session::{issue_tokens, roles_of, user_json, verify_password};
use crate::system::{DEFAULT_TENANT, resolve_tenant};
use axum::{extract::State, Json};
use base64::Engine;
use hmac::{Hmac, Mac};
use mox_api_protocol::{ApiResponse, api_error, api_ok};
use serde::Deserialize;
use serde_json::{Value, json};
use rand::RngCore;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;
use tokio::sync::RwLock;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

const TOTP_PERIOD: u64 = 30;
const TOTP_DIGITS: u32 = 6;
const MFA_TOKEN_TTL_SECS: i64 = 5 * 60;

// ======================== 内存态 ========================

#[derive(Clone, Default)]
pub(crate) struct MfaRecord {
    /// base32 secret（解码后存原始字节也可；这里直接存 base32 字符串，方便 TOTP 计算）
    pub secret_base32: String,
    pub enabled: bool,
    /// SHA-256 恢复码摘要列表（用后移除）
    pub recovery_hashes: Vec<String>,
}

fn mfa_store() -> &'static RwLock<std::collections::HashMap<String, MfaRecord>> {
    static STORE: OnceLock<RwLock<std::collections::HashMap<String, MfaRecord>>> = OnceLock::new();
    STORE.get_or_init(|| RwLock::new(std::collections::HashMap::new()))
}

// ======================== TOTP 核心（RFC 6238） ========================

/// 生成随机 base32 secret（160 bit = 32 base32 字符）
pub(crate) fn generate_totp_secret() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 20];
    rand::thread_rng().fill_bytes(&mut buf);
    base32_encode(&buf)
}

fn base32_encode(data: &[u8]) -> String {
    const ALPH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u32;
    let mut acc = 0u32;
    let mut out = String::new();
    for &b in data {
        acc = (acc << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            let idx = ((acc >> bits) & 31) as usize;
            out.push(ALPH[idx] as char);
        }
    }
    if bits > 0 {
        let idx = ((acc << (5 - bits)) & 31) as usize;
        out.push(ALPH[idx] as char);
    }
    out
}

fn base32_decode(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).map(|c| c.to_ascii_uppercase()).collect();
    let mut bits = 0u32;
    let mut acc = 0u32;
    let mut out = Vec::new();
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'2'..=b'7' => c - b'2' + 26,
            _ => return None,
        };
        acc = (acc << 5) | v as u32;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-'|b'_'|b'.'|b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// otpauth:// URI（供 QR 扫描）
pub(crate) fn otpauth_uri(secret_base32: &str, account: &str, issuer: &str) -> String {
    let label = pct_encode(&format!("{}:{}", issuer, account));
    format!(
        "otpauth://totp/{}?secret={}&issuer={}&algorithm=SHA1&digits=6&period=30",
        label, secret_base32, pct_encode(issuer)
    )
}

/// 计算某时刻对应的 TOTP 6 位码
pub(crate) fn totp_code_at(secret_bytes: &[u8], timestamp_secs: u64) -> Option<String> {
    let counter = timestamp_secs / TOTP_PERIOD;
    hmac_totp(secret_bytes, counter)
}

fn hmac_totp(key: &[u8], counter: u64) -> Option<String> {
    let mut mac = HmacSha1::new_from_slice(key).ok()?;
    mac.update(&counter.to_be_bytes());
    let raw = mac.finalize().into_bytes();
    let offset = (raw[raw.len() - 1] & 0x0f) as usize;
    let bin_code = u32::from_be_bytes([raw[offset], raw[offset + 1], raw[offset + 2], raw[offset + 3]]) & 0x7fff_ffff;
    let code = bin_code % (10u32.pow(TOTP_DIGITS));
    Some(format!("{:0width$}", code, width = TOTP_DIGITS as usize))
}

/// 校验 TOTP 码：允许当前 ±1 窗口
pub(crate) fn verify_totp(secret_base32: &str, code: &str, now_secs: u64) -> bool {
    let Some(key) = base32_decode(secret_base32) else { return false };
    let code = code.trim();
    if code.len() != TOTP_DIGITS as usize || !code.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    for drift in [-1i64, 0, 1] {
        let t = (now_secs as i64 + drift * TOTP_PERIOD as i64) as u64;
        if let Some(expect) = hmac_totp(&key, t / TOTP_PERIOD) {
            if constant_time_eq(&expect, code) {
                return true;
            }
        }
    }
    false
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() { return false; }
    let mut d = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) { d |= x ^ y; }
    d == 0
}

// ======================== 恢复码 ========================

fn generate_recovery_codes() -> Vec<String> {
    use rand::RngCore;
    let mut out = Vec::new();
    for _ in 0..10 {
        let mut b = [0u8; 5];
        rand::thread_rng().fill_bytes(&mut b);
        out.push(b.iter().map(|x| format!("{:02x}", x)).collect::<Vec<_>>().join(""));
    }
    out
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

// ======================== mfa_token（短时 JWT） ========================

fn sign_mfa_token(s: &GatewayState, user_id: &str) -> String {
    let claims = json!({"sub": user_id, "purpose": "mfa"});
    sign_short_lived(&s.config.auth.jwt_secret, &s.config.auth.token_issuer, claims, MFA_TOKEN_TTL_SECS)
}

fn verify_mfa_token(s: &GatewayState, token: &str) -> Option<String> {
    let payload = verify_short_lived(&s.config.auth.jwt_secret, token)?;
    if payload.get("purpose").and_then(|v| v.as_str()) != Some("mfa") { return None; }
    payload.get("sub").and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn b64u(data: &[u8]) -> String { base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data) }
fn b64u_decode(data: &str) -> Option<Vec<u8>> { base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(data).ok() }

fn sign_short_lived(secret: &str, issuer: &str, mut claims: Value, ttl: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    claims.as_object_mut().unwrap().insert("iss".into(), json!(issuer));
    claims.as_object_mut().unwrap().insert("iat".into(), json!(now));
    claims.as_object_mut().unwrap().insert("exp".into(), json!(now + ttl));
    let header = json!({"alg":"HS256","typ":"JWT"});
    let h = b64u(&serde_json::to_vec(&header).unwrap());
    let p = b64u(&serde_json::to_vec(&claims).unwrap());
    let input = format!("{}.{}", h, p);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(input.as_bytes());
    format!("{}.{}", input, b64u(&mac.finalize().into_bytes()))
}

fn verify_short_lived(secret: &str, token: &str) -> Option<Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 { return None; }
    let sig = b64u_decode(parts[2])?;
    let input = format!("{}.{}", parts[0], parts[1]);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(input.as_bytes());
    mac.verify_slice(&sig).ok()?;
    let payload: Value = serde_json::from_slice(&b64u_decode(parts[1])?).ok()?;
    if let Some(exp) = payload.get("exp").and_then(|v| v.as_i64()) {
        if exp < chrono::Utc::now().timestamp() { return None; }
    }
    Some(payload)
}

// ======================== 请求体 ========================

#[derive(Deserialize)]
pub struct AuthReauthReq {
    username: Option<String>,
    password: Option<String>,
    tenant_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfirmReq {
    username: Option<String>,
    password: Option<String>,
    code: Option<String>,
    tenant_id: Option<String>,
}

#[derive(Deserialize)]
pub struct VerifyReq {
    mfa_token: Option<String>,
    code: Option<String>,
}

// ======================== 内部：用户名密码重新认证 ========================

fn reauth<'a>(s: &GatewayState, body: &'a AuthReauthReq) -> Result<mox_platform_iam_core::IamUser, ApiResponse<Value>> {
    let username = body.username.clone().unwrap_or_default();
    let password = body.password.clone().unwrap_or_default();
    let tenant_input = body.tenant_id.clone().unwrap_or_default();
    let tenant = if tenant_input.is_empty() || tenant_input == "default" { DEFAULT_TENANT.to_string() } else { tenant_input };
    let tenant_id = resolve_tenant(s, &tenant).unwrap_or(DEFAULT_TENANT.to_string());
    let user = s.iam.find_user_by_tenant_username(&tenant_id, &username)
        .ok_or_else(|| api_error(401, "用户名或密码错误"))?;
    let stored = user.password_hash.as_deref().unwrap_or("");
    if !verify_password(&password, stored) {
        return Err(api_error(401, "用户名或密码错误"));
    }
    Ok(user)
}

// ======================== Handler ========================

/// POST /api/auth/mfa/bind —— 生成 TOTP secret + otpauth URI（未启用）
pub(crate) async fn bind_handler(
    State(s): State<GatewayState>,
    Json(body): Json<AuthReauthReq>,
) -> ApiResponse<Value> {
    let user = match reauth(&s, &body) { Ok(u) => u, Err(e) => return e };
    let secret = generate_totp_secret();
    let uri = otpauth_uri(&secret, &user.username, "mox-platform");
    // 暂存到 store（enabled=false），confirm 后才真正启用
    mfa_store().write().await.insert(user.user_id.clone(), MfaRecord {
        secret_base32: secret.clone(),
        enabled: false,
        recovery_hashes: vec![],
    });
    api_ok(json!({
        "secret": secret,
        "otpauth_uri": uri,
        "message": "请用 TOTP 应用扫码，然后输入 6 位码确认绑定",
    }))
}

/// POST /api/auth/mfa/confirm —— 校验绑定码成功后启用，返回恢复码明文
pub(crate) async fn confirm_handler(
    State(s): State<GatewayState>,
    Json(body): Json<ConfirmReq>,
) -> ApiResponse<Value> {
    let user = match reauth(&s, &AuthReauthReq {
        username: body.username.clone(), password: body.password.clone(), tenant_id: body.tenant_id.clone(),
    }) { Ok(u) => u, Err(e) => return e };
    let code = body.code.unwrap_or_default();
    let store = mfa_store().read().await;
    let rec = match store.get(&user.user_id) {
    Some(r) => r.clone(),
    None => return api_error(400, "请先调用 bind 获取 secret"),
};
    if rec.enabled {
        return api_error(409, "MFA 已启用，如需重置请先解绑");
    }
    let now = chrono::Utc::now().timestamp() as u64;
    if !verify_totp(&rec.secret_base32, &code, now) {
        return api_error(400, "TOTP 码不正确");
    }
    drop(store);

    let recovery = generate_recovery_codes();
    let hashes = recovery.iter().map(|c| sha256_hex(c)).collect::<Vec<_>>();
    mfa_store().write().await.entry(user.user_id.clone()).and_modify(|r| {
        r.enabled = true;
        r.recovery_hashes = hashes;
    });
    api_ok(json!({
        "enabled": true,
        "recovery_codes": recovery,
        "message": "MFA 已启用，恢复码仅此一次展示，请妥善保存",
    }))
}

/// POST /api/auth/mfa/unbind —— 关闭 MFA（需当前 TOTP 码）
pub(crate) async fn unbind_handler(
    State(s): State<GatewayState>,
    Json(body): Json<ConfirmReq>,
) -> ApiResponse<Value> {
    let user = match reauth(&s, &AuthReauthReq {
        username: body.username.clone(), password: body.password.clone(), tenant_id: body.tenant_id.clone(),
    }) { Ok(u) => u, Err(e) => return e };
    let code = body.code.unwrap_or_default();
    let store = mfa_store().read().await;
    let rec = match store.get(&user.user_id) {
        Some(r) if r.enabled => r.clone(),
        Some(_) => return api_error(400, "MFA 未启用"),
        None => return api_error(400, "MFA 未启用"),
    };
    let now = chrono::Utc::now().timestamp() as u64;
    let ok = verify_totp(&rec.secret_base32, &code, now)
        || try_consume_recovery_code(&user.user_id, &code).await;
    if !ok {
        return api_error(400, "MFA 校验码不正确");
    }
    mfa_store().write().await.remove(&user.user_id);
    api_ok(json!({"unbound": true}))
}

/// POST /api/auth/mfa/verify —— 登录第二步：用 mfa_token + TOTP/恢复码 换正式 JWT
pub(crate) async fn verify_handler(
    State(s): State<GatewayState>,
    Json(body): Json<VerifyReq>,
) -> ApiResponse<Value> {
    let mfa_token = body.mfa_token.unwrap_or_default();
    let code = body.code.unwrap_or_default();
    let user_id = match verify_mfa_token(&s, &mfa_token) {
    Some(u) => u,
    None => return api_error(401, "MFA 会话无效或已过期，请重新登录"),
};
    let user = match s.iam.get_user(&user_id) {
        Ok(Some(u)) => u,
        _ => return api_error(401, "用户不存在"),
    };
    if user.user_status != "active" {
        return api_error(403, "账号已停用");
    }
    let rec = match mfa_store().read().await.get(&user.user_id).cloned() {
        Some(r) if r.enabled => r,
        _ => return api_error(400, "该用户未启用 MFA"),
    };
    let now = chrono::Utc::now().timestamp() as u64;
    let totp_ok = verify_totp(&rec.secret_base32, &code, now);
    let rec_ok = if !totp_ok { try_consume_recovery_code(&user.user_id, &code).await } else { false };
    if !totp_ok && !rec_ok {
        return api_error(400, "MFA 校验码不正确");
    }
    let roles = roles_of(&s, &user.tenant_id, &user);
    let (access_token, refresh_token) = match issue_tokens(&s, &user, &roles) {
        Ok(t) => t,
        Err(e) => return api_error(500, &e),
    };
    api_ok(json!({
        "access_token": access_token,
        "refresh_token": refresh_token,
        "token_type": "Bearer",
        "user": user_json(&user, roles),
    }))
}

/// 尝试用恢复码；成功则从 store 移除该 hash（一次性）
async fn try_consume_recovery_code(user_id: &str, code: &str) -> bool {
    let h = sha256_hex(code.trim());
    let mut w = mfa_store().write().await;
    let rec = match w.get_mut(user_id) { Some(r) => r, None => return false };
    if let Some(pos) = rec.recovery_hashes.iter().position(|x| x == &h) {
        rec.recovery_hashes.remove(pos);
        true
    } else {
        false
    }
}

/// 登录流程钩子：密码校验通过后调用；返回 Some(mfa_token) 表示需要 MFA 二次校验
pub(crate) fn mfa_challenge_or_issue(
    s: &GatewayState,
    user: &mox_platform_iam_core::IamUser,
) -> Result<Option<Value>, ApiResponse<Value>> {
    let enabled = mfa_store().try_read()
        .map(|g| g.get(&user.user_id).map(|r| r.enabled).unwrap_or(false))
        .unwrap_or(false);
    if !enabled {
        return Ok(None);
    }
    let token = sign_mfa_token(s, &user.user_id);
    Ok(Some(json!({
        "mfa_required": true,
        "mfa_token": token,
        "message": "请输入 6 位 TOTP 码完成二次校验",
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totp_known_vector_rfc6238_sha1() {
        // RFC 6238 测试向量：secret = 3132333435363738393031323334353637383930 (ASCII "12345678901234567890")
        let secret = base32_encode(b"12345678901234567890");
        // time=59s → counter=1 → 94287082（8 位），截 6 位 = 287082
        let code = hmac_totp(b"12345678901234567890", 1).unwrap();
        assert_eq!(code, "287082");
        // 自校验往返
        assert!(verify_totp(&secret, &code, 59));
        assert!(!verify_totp(&secret, "000000", 59));
    }

    #[tokio::test]
    async fn recovery_code_one_time_use() {
        let user = "utest_user_mfa";
        let codes = generate_recovery_codes();
        let hashes: Vec<String> = codes.iter().map(|c| sha256_hex(c)).collect();
        mfa_store().write().await.insert(user.into(), MfaRecord {
            secret_base32: generate_totp_secret(),
            enabled: true,
            recovery_hashes: hashes,
        });
        let first = codes[0].clone();
        assert!(try_consume_recovery_code(user, &first).await);
        assert!(!try_consume_recovery_code(user, &first).await);
        mfa_store().write().await.remove(user);
    }

    #[test]
    fn secret_base32_roundtrip() {
        let s = generate_totp_secret();
        let raw = base32_decode(&s).unwrap();
        assert_eq!(base32_encode(&raw), s);
    }
}
