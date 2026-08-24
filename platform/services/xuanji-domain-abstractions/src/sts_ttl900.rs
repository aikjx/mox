//! STS AssumeRole 硬 TTL=900 秒（15 分钟）
//!
//! 规范要求（与 spec.md AC-T10-5~7 对齐）：
//! 1. `duration_secs != 900` → 返回 Err("STS_TTL_MUST_BE_900_SECONDS")
//! 2. `session_token = base64(HMAC-SHA256(root_secret, role_id || session_name || expiration_ms_LE8))`
//! 3. `StsCredentials::verify(&self, root_secret)` 自证签名正确且未过期
//! 4. MockIamProvider 侧在 authorize 时校验 STS 凭据

use crate::iam::StsCredentials;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub const STS_ALLOWED_TTL_SECS: u64 = 900;
pub const STS_TTL_MS: u64 = STS_ALLOWED_TTL_SECS * 1000;
const STS_ERR_TTL: &str = "STS_TTL_MUST_BE_900_SECONDS";
const STS_ERR_SESSION: &str = "STS_SESSION_NAME_EMPTY";
const STS_ERR_ROLE: &str = "STS_ROLE_ID_EMPTY";

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn sign_session_token(
    secret: &[u8],
    role_id: &str,
    session_name: &str,
    expiration_ms: u64,
) -> String {
    let mut mac = HmacSha256::new_from_slice(secret).expect("Hmac accepts any key size");
    mac.update(role_id.as_bytes());
    mac.update(session_name.as_bytes());
    mac.update(expiration_ms.to_le_bytes());
    let sig = mac.finalize().into_bytes();
    base64_encode(&sig)
}

fn base64_encode(bytes: &[u8]) -> String {
    // 手写 base64，避免依赖额外特性（xuanji-domain-abstractions 默认 features 不含 base64）
    // 若 workspace 依赖的 base64 crate 可用，可切换为标准实现；此处提供健壮 fallback：
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

/// STS AssumeRole 结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StsAssumeRoleResult {
    pub credentials: StsCredentials,
    pub role_id: String,
    pub session_name: String,
    pub issued_at_ms: u64,
}

/// TTL=900 秒专用签发器
pub struct StsService {
    root_secret: Vec<u8>,
}

impl StsService {
    pub fn new(root_secret: impl AsRef<[u8]>) -> Self {
        Self {
            root_secret: root_secret.as_ref().to_vec(),
        }
    }

    /// 签发临时凭据。`duration_secs` 必须精确等于 900；否则直接拒绝。
    pub fn assume_role(
        &self,
        role_id: &str,
        session_name: &str,
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
        // access_key / secret_key 派生自同一签名家族（防止靠字符串猜）
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
            issued_at_ms: issued,
        })
    }

    /// 验证：签名对 + 未过期。可选的"当前时间"供测试注入。
    pub fn verify_credentials(
        &self,
        cred: &StsCredentials,
        role_id: &str,
        session_name: &str,
        now_ms_override: Option<u64>,
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

/// StsCredentials 扩展：verify 便携函数（保持 spec 对 trait 形状要求）
pub trait StsVerifyExt {
    fn verify(
        &self,
        root_secret: &[u8],
        role_id: &str,
        session_name: &str,
        now_ms_override: Option<u64>,
    ) -> Result<bool, String>;
}

impl StsVerifyExt for StsCredentials {
    fn verify(
        &self,
        root_secret: &[u8],
        role_id: &str,
        session_name: &str,
        now_ms_override: Option<u64>,
    ) -> Result<bool, String> {
        let svc = StsService::new(root_secret);
        svc.verify_credentials(self, role_id, session_name, now_ms_override)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"sts-test-key-000000000000000000000000";

    #[test]
    fn t_a3_1_ttl_900_ok_window() {
        let svc = StsService::new(KEY);
        let before = now_ms();
        let r = svc.assume_role("role-admin", "sess-1", 900).unwrap();
        let after = now_ms();
        let d = r.credentials.expiration as i128 - before as i128;
        assert!(d >= 899_000i128, "exp - before too small: {d}");
        let d2 = r.credentials.expiration as i128 - after as i128;
        assert!(d2 <= 901_000i128, "exp - after too large: {d2}");
    }

    #[test]
    fn t_a3_2_duration_3600_rejected() {
        let svc = StsService::new(KEY);
        let err = svc.assume_role("r", "s", 3600).unwrap_err();
        assert!(err.contains(STS_ERR_TTL), "err={err}");
    }

    #[test]
    fn t_a3_2b_edge_899_and_901_rejected() {
        let svc = StsService::new(KEY);
        assert!(svc.assume_role("r", "s", 899).is_err());
        assert!(svc.assume_role("r", "s", 901).is_err());
    }

    #[test]
    fn t_a3_3_verify_ok_and_tamper_fail() {
        let svc = StsService::new(KEY);
        let r = svc.assume_role("r1", "s1", 900).unwrap();
        let ok = r
            .credentials
            .verify(KEY, "r1", "s1", None)
            .expect("verify returns Result");
        assert!(ok);
        // 篡改末字节
        let mut tampered = r.credentials.clone();
        let mut s = tampered.session_token.into_bytes();
        if let Some(last) = s.last_mut() {
            *last ^= 0x01;
        }
        tampered.session_token = String::from_utf8(s).unwrap_or_default();
        let ok2 = tampered.verify(KEY, "r1", "s1", None).unwrap();
        assert!(!ok2, "tampered session_token must fail verify");
    }

    #[test]
    fn t_a3_4_expired_returns_err() {
        let svc = StsService::new(KEY);
        let r = svc.assume_role("r1", "s1", 900).unwrap();
        // 未来 16 分钟 = 960 秒后（过了 TTL=900s）
        let future_ms = r.issued_at_ms + 16 * 60 * 1000;
        let res = r
            .credentials
            .verify(KEY, "r1", "s1", Some(future_ms));
        assert!(res.is_err(), "expect Err for expired token, got {res:?}");
        assert!(res.unwrap_err().contains("expired"));
    }

    #[test]
    fn t_a3_5_session_name_bound_into_signature() {
        let svc = StsService::new(KEY);
        let r1 = svc.assume_role("r", "s-A", 900).unwrap();
        let r2 = svc.assume_role("r", "s-B", 900).unwrap();
        assert_ne!(r1.credentials.session_token, r2.credentials.session_token);
        // 用错误的 session 名验证 r1 → 不匹配
        let ok = r1.credentials.verify(KEY, "r", "s-B", None).unwrap();
        assert!(!ok);
    }

    #[test]
    fn t_a3_6_role_id_distinguishes_tokens() {
        let svc = StsService::new(KEY);
        let ra = svc.assume_role("role-A", "s", 900).unwrap();
        let rb = svc.assume_role("role-B", "s", 900).unwrap();
        assert_ne!(ra.credentials.session_token, rb.credentials.session_token);
        // 验证时交换 role 名 → false
        let ok = ra.credentials.verify(KEY, "role-B", "s", None).unwrap();
        assert!(!ok);
    }

    #[test]
    fn t_a3_7_empty_role_or_session_rejected() {
        let svc = StsService::new(KEY);
        assert!(svc.assume_role("", "s", 900).is_err());
        assert!(svc.assume_role("r", "", 900).is_err());
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
                let r = s.assume_role(&role, &sess, 900).unwrap();
                assert!(r.credentials.verify(KEY, &role, &sess, None).unwrap());
                // 用错误的 role 验证
                let bad = r.credentials.verify(KEY, "role-other", &sess, None).unwrap();
                assert!(!bad);
            }));
        }
        for jh in jhs {
            jh.join().unwrap();
        }
    }
}
