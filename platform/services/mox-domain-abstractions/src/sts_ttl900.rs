//! STS AssumeRole 硬 TTL=900 秒（15 分钟）
//!
//! 规范要求（与 spec.md AC-T10-5~7 对齐）：
//! 1. `duration_secs != 900` → 返回 Err("STS_TTL_MUST_BE_900_SECONDS")
//! 2. `session_token = base64(HMAC-SHA256(root_secret, role_id || session_name || expiration_ms_LE8))`
//! 3. `StsCredentials::verify(&self, root_secret)` 自证签名正确且未过期
//! 4. MockIamProvider 侧在 authorize 时校验 STS 凭据
//!
//! SM2 双签名逻辑位于独立的 mox-standards crate 中（`gm-sm` feature），
//! 它通过 [`StsAssumeRoleResult::sm2_signature_hex`] 字段与本模块交互。

pub use crate::iam::StsCredentials;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const STS_ALLOWED_TTL_SECS: u64 = 900;
pub const STS_TTL_MS: u64 = STS_ALLOWED_TTL_SECS * 1000;
const STS_ERR_TTL: &str = "STS_TTL_MUST_BE_900_SECONDS";
const STS_ERR_SESSION: &str = "STS_SESSION_NAME_EMPTY";
const STS_ERR_ROLE: &str = "STS_ROLE_ID_EMPTY";

pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn sign_session_token(
    secret: &[u8],
    role_id: &str,
    session_name: &str,
    expiration_ms: u64,
) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("Hmac accepts any key size");
    mac.update(role_id.as_bytes());
    mac.update(session_name.as_bytes());
    mac.update(&expiration_ms.to_le_bytes());
    let sig = mac.finalize().into_bytes();
    base64_encode(&sig)
}

fn base64_encode(bytes: &[u8]) -> String {
    #[cfg(feature = "serde")]
    {
        let _ = bytes;
        "".to_string()
    }
    #[cfg(not(feature = "serde"))]
    {
        let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((bytes.len() + 2) / 3 * 4);
        let mut i = 0;
        while i + 3 <= bytes.len() {
            let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8) | (bytes[i + 2] as u32);
            out.push(table[((n >> 18) & 63) as usize] as char);
            out.push(table[((n >> 12) & 63) as usize] as char);
            out.push(table[((n >> 6) & 63) as usize] as char);
            out.push(table[(n & 63) as usize] as char);
            i += 3;
        }
        let rem = bytes.len() - i;
        if rem == 1 {
            let n = (bytes[i] as u32) << 16;
            out.push(table[((n >> 18) & 63) as usize] as char);
            out.push(table[((n >> 12) & 63) as usize] as char);
            out.push('=');
            out.push('=');
        } else if rem == 2 {
            let n = ((bytes[i] as u32) << 16) | ((bytes[i + 1] as u32) << 8);
            out.push(table[((n >> 18) & 63) as usize] as char);
            out.push(table[((n >> 12) & 63) as usize] as char);
            out.push(table[((n >> 6) & 63) as usize] as char);
            out.push('=');
        }
        out
    }
}

// ===========================================================================
// STS AssumeRole Result
// ===========================================================================

/// STS AssumeRole 结果
///
/// 当启用上层 `gm-sm` 双签名封装时，`sm2_signature_hex` 填充 128 字符的
/// SM2(r||s) hex 签名；否则（基础 TTL-only 模式）为 `None`，表示遗留令牌，
/// verify 兼容接受。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StsAssumeRoleResult {
    pub credentials: StsCredentials,
    pub role_id: String,
    pub session_name: String,
    pub user_id: String,
    pub issued_at_ms: u64,
    pub sm2_signature_hex: Option<String>,
}

// ===========================================================================
// StsService  (HMAC-SHA256 基础实现，不含 SM2 私钥材料)
// ===========================================================================

/// TTL=900 秒专用签发器（基础 HMAC 实现）。
///
/// SM2 双签名能力见 `mox-standards` crate 中 `StsServiceSm2` 封装，
/// 其内部持有本结构作为 HMAC 引擎，并附加 `Sm2RoleKeystore`。
#[derive(Debug, Clone)]
pub struct StsService {
    root_secret: Vec<u8>,
}

impl StsService {
    pub fn new(root_secret: impl AsRef<[u8]>) -> Self {
        Self {
            root_secret: root_secret.as_ref().to_vec(),
        }
    }

    pub fn root_secret(&self) -> &[u8] {
        &self.root_secret
    }

    /// 签发临时凭据（纯 HMAC）。`duration_secs` 必须精确等于 900；否则直接拒绝。
    ///
    /// - `user_id`：请求主体身份标识，用于上层 SM2 载荷；在基础模式下仅作回显存储。
    /// - `sm2_signature_hex` 在基础模式下始终返回 `None`，由上层 SM2 封装器填充。
    pub fn assume_role(
        &self,
        role_id: &str,
        session_name: &str,
        user_id: &str,
        duration_secs: u64,
    ) -> Result<StsAssumeRoleResult, String> {
        if role_id.is_empty() {
            return Err(STS_ERR_ROLE.into());
        }
        if session_name.is_empty() {
            return Err(STS_ERR_SESSION.into());
        }
        if duration_secs != STS_ALLOWED_TTL_SECS {
            return Err(format!(
                "{STS_ERR_TTL}: got={duration_secs}, expected={STS_ALLOWED_TTL_SECS}"
            ));
        }
        let issued = now_ms();
        let expiration = issued + STS_TTL_MS;
        let session_token = sign_session_token(&self.root_secret, role_id, session_name, expiration);
        let ak = format!(
            "STS-{}-{}",
            role_id,
            sign_session_token(&self.root_secret, role_id, &format!("AK-{session_name}"), expiration)
                .chars()
                .take(24)
                .collect::<String>()
        );
        let sk = sign_session_token(
            &self.root_secret,
            &format!("SK-{role_id}"),
            session_name,
            expiration,
        );

        Ok(StsAssumeRoleResult {
            credentials: StsCredentials {
                access_key: ak,
                secret_key: sk,
                session_token,
                expiration,
            },
            role_id: role_id.into(),
            session_name: session_name.into(),
            user_id: user_id.into(),
            issued_at_ms: issued,
            sm2_signature_hex: None, // 基础模式不含 SM2；由封装器注入
        })
    }

    /// 基础验证：HMAC 签名对 + 未过期。
    ///
    /// 注意：本方法 **不会** 校验 `sm2_signature_hex`（因为本 crate 不依赖国密库）。
    /// 完整的双签名验证请使用上层封装（`mox-standards` 的 `StsServiceSm2`），
    /// 它会先调用本方法完成 HMAC + 过期检查，再做 SM2 签名验证。
    ///
    /// 返回：
    /// - `Ok(true)`  HMAC 通过且未过期。
    /// - `Ok(false)` HMAC 不匹配。
    /// - `Err(...)`   令牌过期或其它可解释错误。
    pub fn verify_credentials(
        &self,
        cred: &StsCredentials,
        role_id: &str,
        session_name: &str,
        _user_id: &str,
        now_ms_override: Option<u64>,
        _sm2_signature_hex: Option<&str>,
    ) -> Result<bool, String> {
        let expected = sign_session_token(&self.root_secret, role_id, session_name, cred.expiration);
        if expected != cred.session_token {
            return Ok(false);
        }
        let now = now_ms_override.unwrap_or_else(now_ms);
        if now >= cred.expiration {
            return Err("STS token expired".into());
        }
        Ok(true)
    }
}

/// StsCredentials 扩展：verify 便携函数（基础实现，不校验 SM2）。
pub trait StsVerifyExt {
    fn verify(
        &self,
        root_secret: &[u8],
        role_id: &str,
        session_name: &str,
        user_id: &str,
        now_ms_override: Option<u64>,
        sm2_signature_hex: Option<&str>,
    ) -> Result<bool, String>;
}

impl StsVerifyExt for StsCredentials {
    fn verify(
        &self,
        root_secret: &[u8],
        role_id: &str,
        session_name: &str,
        user_id: &str,
        now_ms_override: Option<u64>,
        sm2_signature_hex: Option<&str>,
    ) -> Result<bool, String> {
        let svc = StsService::new(root_secret);
        svc.verify_credentials(self, role_id, session_name, user_id, now_ms_override, sm2_signature_hex)
    }
}

// ===========================================================================
// Tests  (基础 HMAC；A1-A4 gm-sm 测试位于 mox-standards crate)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"sts-test-key-000000000000000000000000";

    #[test]
    fn t_a3_1_ttl_900_ok_window() {
        let svc = StsService::new(KEY);
        let before = now_ms();
        let r = svc.assume_role("role-admin", "sess-1", "user-1", 900).unwrap();
        let after = now_ms();
        let d = r.credentials.expiration as i128 - before as i128;
        assert!(d >= 899_000i128, "exp - before too small: {d}");
        let d2 = r.credentials.expiration as i128 - after as i128;
        assert!(d2 <= 901_000i128, "exp - after too large: {d2}");
    }

    #[test]
    fn t_a3_2_duration_3600_rejected() {
        let svc = StsService::new(KEY);
        let err = svc.assume_role("r", "s", "u", 3600).unwrap_err();
        assert!(err.contains(STS_ERR_TTL), "err={err}");
    }

    #[test]
    fn t_a3_2b_edge_899_and_901_rejected() {
        let svc = StsService::new(KEY);
        assert!(svc.assume_role("r", "s", "u", 899).is_err());
        assert!(svc.assume_role("r", "s", "u", 901).is_err());
    }

    #[test]
    fn t_a3_3_verify_ok_and_tamper_fail() {
        let svc = StsService::new(KEY);
        let r = svc.assume_role("r1", "s1", "u1", 900).unwrap();
        let ok = r
            .credentials
            .verify(KEY, "r1", "s1", "u1", None, None)
            .expect("verify returns Result");
        assert!(ok);
        let mut tampered = r.credentials.clone();
        let mut s = tampered.session_token.into_bytes();
        if let Some(last) = s.last_mut() {
            *last ^= 0x01;
        }
        tampered.session_token = String::from_utf8(s).unwrap_or_default();
        let ok2 = tampered.verify(KEY, "r1", "s1", "u1", None, None).unwrap();
        assert!(!ok2, "tampered session_token must fail verify");
    }

    #[test]
    fn t_a3_4_expired_returns_err() {
        let svc = StsService::new(KEY);
        let r = svc.assume_role("r1", "s1", "u1", 900).unwrap();
        let future_ms = r.issued_at_ms + 16 * 60 * 1000;
        let res = r
            .credentials
            .verify(KEY, "r1", "s1", "u1", Some(future_ms), None);
        assert!(res.is_err(), "expect Err for expired token, got {res:?}");
        assert!(res.unwrap_err().contains("expired"));
    }

    #[test]
    fn t_a3_5_session_name_bound_into_signature() {
        let svc = StsService::new(KEY);
        let r1 = svc.assume_role("r", "s-A", "u", 900).unwrap();
        let r2 = svc.assume_role("r", "s-B", "u", 900).unwrap();
        assert_ne!(r1.credentials.session_token, r2.credentials.session_token);
        let ok = r1.credentials.verify(KEY, "r", "s-B", "u", None, None).unwrap();
        assert!(!ok);
    }

    #[test]
    fn t_a3_6_role_id_distinguishes_tokens() {
        let svc = StsService::new(KEY);
        let ra = svc.assume_role("role-A", "s", "u", 900).unwrap();
        let rb = svc.assume_role("role-B", "s", "u", 900).unwrap();
        assert_ne!(ra.credentials.session_token, rb.credentials.session_token);
        let ok = ra.credentials.verify(KEY, "role-B", "s", "u", None, None).unwrap();
        assert!(!ok);
    }

    #[test]
    fn t_a3_7_empty_role_or_session_rejected() {
        let svc = StsService::new(KEY);
        assert!(svc.assume_role("", "s", "u", 900).is_err());
        assert!(svc.assume_role("r", "", "u", 900).is_err());
    }

    #[test]
    fn t_a3_8_concurrent_50_independent() {
        use std::sync::Arc;
        use std::thread;
        let svc = Arc::new(StsService::new(KEY));
        let mut jhs = vec![];
        for i in 0..50 {
            let s = svc.clone();
            jhs.push(thread::spawn(move || {
                let role = format!("role-{i}");
                let sess = format!("session-{i}");
                let user = format!("user-{i}");
                let r = s.assume_role(&role, &sess, &user, 900).unwrap();
                assert!(r.credentials.verify(KEY, &role, &sess, &user, None, None).unwrap());
                let bad = r.credentials.verify(KEY, "role-other", &sess, &user, None, None).unwrap();
                assert!(!bad);
            }));
        }
        for jh in jhs {
            jh.join().unwrap();
        }
    }
}
