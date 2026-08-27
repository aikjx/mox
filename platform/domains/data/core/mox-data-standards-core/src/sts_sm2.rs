// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! T24-4 STS SM2 双签名封装（`gm-sm` feature 下可用）。
//!
//! 在不修改 `mox-domain-abstractions` 的前提下（避免 crate 循环依赖），
//! 本模块提供 `StsServiceSm2`，它包装 `mox_cloud_foundation::sts_ttl900::StsService`
//! 作为 HMAC 引擎，并附加 `Sm2RoleKeystore`：
//!
//! - 当 role 已在 keystore 注册时，`assume_role` 会先调用基础 HMAC 签发，
//!   再使用该 role 的 SM2 私钥对扩展 payload 签名，返回带 `sm2_signature_hex` 的结果。
//! - 当 role 未注册时，行为退化为基础 HMAC 单签名（`sm2_signature_hex=None`），
//!   与遗留实现 100% 兼容。
//! - `verify_credentials`：当 feature=gm-sm 启用且 sig_hex=Some 时强制走 SM2 验证；
//!   若验证失败返回 Err；若 sig_hex=None（遗留）则返回 Ok，保持向后兼容。
//!
//! 载荷构造与任务描述严格一致：
//! ```text
//! payload_for_sm2 = session_token_bytes
//!                 || expiration_ms.to_le_bytes()
//!                 || b"sts"
//!                 || role_id.as_bytes()
//!                 || user_id.as_bytes()
//! ```
//! 使用 `id = b"mox-sts"`（ZA 标识），签名 OS RNG。
//!
//! SM2 hex 签名字段长度固定为 128 字符（r||s 各 32 bytes = 64 hex chars × 2）。

use std::collections::HashMap;

use mox_cloud_foundation::sts_ttl900::{
    now_ms, sign_session_token, StsAssumeRoleResult, StsCredentials, StsService,
    StsVerifyExt,
};

use crate::sm2_sign::{Sm2Pk, Sm2Sk};

// ---------------------------------------------------------------------------
// Sm2RoleKeystore  — 保持与任务描述完全一致的 API 形状
// ---------------------------------------------------------------------------

pub struct Sm2RoleKeystore {
    keys: HashMap<String, (Sm2Pk, Sm2Sk)>,
}

impl Sm2RoleKeystore {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    /// Register (role_id → pk_bytes_uncompressed_65, sk_32bytes_pair).
    pub fn register(
        &mut self,
        role_id: &str,
        pk_65: [u8; 65],
        sk_32: [u8; 32],
    ) -> Result<(), String> {
        let pk = Sm2Pk::from_uncompressed_bytes(&pk_65)
            .ok_or_else(|| "Invalid uncompressed SM2 pk bytes (expect 0x04 prefix + 64 bytes xy)".to_string())?;
        let sk = Sm2Sk(sk_32);
        let derived = sk.public_key();
        if derived != pk {
            return Err("SM2 register failed: supplied pk does not match public key derived from sk".to_string());
        }
        self.keys.insert(role_id.to_string(), (pk, sk));
        Ok(())
    }

    pub fn get_pk(&self, role_id: &str) -> Option<Sm2Pk> {
        self.keys.get(role_id).map(|(pk, _)| pk.clone())
    }

    fn get_pair(&self, role_id: &str) -> Option<&(Sm2Pk, Sm2Sk)> {
        self.keys.get(role_id)
    }
}

impl Default for Sm2RoleKeystore {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// StsServiceSm2  — 基础 HMAC StsService + SM2 keystore 的组合
// ---------------------------------------------------------------------------

/// STS TTL=900 双签名签发器（HMAC + SM2）。
pub struct StsServiceSm2 {
    inner: StsService,
    keystore: Sm2RoleKeystore,
}

impl StsServiceSm2 {
    pub fn new(root_secret: impl AsRef<[u8]>) -> Self {
        Self {
            inner: StsService::new(root_secret),
            keystore: Sm2RoleKeystore::new(),
        }
    }

    pub fn with_keystore(
        root_secret: impl AsRef<[u8]>,
        keystore: Sm2RoleKeystore,
    ) -> Self {
        Self {
            inner: StsService::new(root_secret),
            keystore,
        }
    }

    pub fn register_sm2_key(
        &mut self,
        role_id: &str,
        pk_65: [u8; 65],
        sk_32: [u8; 32],
    ) -> Result<(), String> {
        self.keystore.register(role_id, pk_65, sk_32)
    }

    pub fn get_sm2_pk(&self, role_id: &str) -> Option<Sm2Pk> {
        self.keystore.get_pk(role_id)
    }

    /// 签发带可选 SM2 双签名的 STS TTL=900 临时凭据。
    pub fn assume_role(
        &self,
        role_id: &str,
        session_name: &str,
        user_id: &str,
        duration_secs: u64,
    ) -> Result<StsAssumeRoleResult, String> {
        let mut r = self.inner.assume_role(role_id, session_name, user_id, duration_secs)?;

        // 若 keystore 包含对应 role → 双签名
        if let Some((_pk, sk)) = self.keystore.get_pair(role_id) {
            let mut payload: Vec<u8> = Vec::with_capacity(
                r.credentials.session_token.len() + 8 + 3 + role_id.len() + user_id.len(),
            );
            payload.extend_from_slice(r.credentials.session_token.as_bytes());
            payload.extend_from_slice(&r.credentials.expiration.to_le_bytes());
            payload.extend_from_slice(b"sts");
            payload.extend_from_slice(role_id.as_bytes());
            payload.extend_from_slice(user_id.as_bytes());

            let mut rng = rand::rngs::OsRng;
            let sig_hex = crate::sm2_sign::sm2_sign_hex(
                &payload,
                b"mox-sts",
                sk,
                &mut rng,
            );
            r.sm2_signature_hex = Some(sig_hex);
        }

        Ok(r)
    }

    /// 双签名验证：
    /// - 先走 HMAC + TTL 检查（基础 StsService）；
    /// - 若 `sm2_signature_hex = Some(s)` 则验证 SM2；
    ///   失败时返回 `Err("STS SM2 signature verification failed")`；
    ///   若没有对应 role 的公钥，也返回 Err（因为有签名要求却无法校验）。
    /// - 若 `sm2_signature_hex = None` 则跳过 SM2 阶段（遗留令牌向后兼容），
    ///   只要 HMAC 通过且未过期即返回 Ok(true)。
    pub fn verify_credentials(
        &self,
        cred: &StsCredentials,
        role_id: &str,
        session_name: &str,
        user_id: &str,
        now_ms_override: Option<u64>,
        sm2_signature_hex: Option<&str>,
    ) -> Result<bool, String> {
        // Stage 1: HMAC + expiry via inner
        // NOTE: inner.verify_credentials ignores SM2 part by design.
        let hmac_ok = self.inner.verify_credentials(
            cred,
            role_id,
            session_name,
            user_id,
            now_ms_override,
            sm2_signature_hex,
        )?;
        if !hmac_ok {
            return Ok(false);
        }

        // Stage 2: SM2 if provided
        if let Some(sig_hex) = sm2_signature_hex {
            let pk = self
                .keystore
                .get_pk(role_id)
                .ok_or_else(|| format!("STS SM2 verify: role {role_id:?} not in keystore (cannot verify SM2 sig)"))?;
            let pk_bytes = pk.as_uncompressed_bytes();
            let mut payload: Vec<u8> = Vec::with_capacity(
                cred.session_token.len() + 8 + 3 + role_id.len() + user_id.len(),
            );
            payload.extend_from_slice(cred.session_token.as_bytes());
            payload.extend_from_slice(&cred.expiration.to_le_bytes());
            payload.extend_from_slice(b"sts");
            payload.extend_from_slice(role_id.as_bytes());
            payload.extend_from_slice(user_id.as_bytes());
            let ok = crate::sm2_sign::sm2_verify_hex(
                &payload,
                b"mox-sts",
                &pk_bytes,
                sig_hex,
            );
            if !ok {
                return Err("STS SM2 signature verification failed".into());
            }
        }
        Ok(true)
    }

    /// Accessor for the inner HMAC service (exposed for advanced composition).
    pub fn inner(&self) -> &StsService {
        &self.inner
    }

    /// Accessor for the keystore (read-only).
    pub fn keystore(&self) -> &Sm2RoleKeystore {
        &self.keystore
    }
}

// ---------------------------------------------------------------------------
// 便捷扩展：对 StsAssumeRoleResult 直接 verify（使用 StsServiceSm2）。
// ---------------------------------------------------------------------------

pub trait StsAssumeRoleVerifyExt {
    fn verify_sm2(&self, svc: &StsServiceSm2) -> Result<bool, String>;
}

impl StsAssumeRoleVerifyExt for StsAssumeRoleResult {
    fn verify_sm2(&self, svc: &StsServiceSm2) -> Result<bool, String> {
        svc.verify_credentials(
            &self.credentials,
            &self.role_id,
            &self.session_name,
            &self.user_id,
            None,
            self.sm2_signature_hex.as_deref(),
        )
    }
}

// Re-export helpers so callers of sts_sm2 don't have to import domain-abstractions
// to build the base session_token manually (rarely needed; kept for tooling tests).
#[allow(dead_code)]
pub(crate) fn _sign_session_token_shim(
    secret: &[u8],
    role_id: &str,
    session_name: &str,
    expiration_ms: u64,
) -> String {
    sign_session_token(secret, role_id, session_name, expiration_ms)
}
#[allow(dead_code)]
pub(crate) fn _now_ms_shim() -> u64 {
    now_ms()
}
// Suppress unused StsVerifyExt re-import warning.
#[allow(dead_code)]
fn _touch_verify_ext(_c: &StsCredentials) {
    use hex::ToHex as _;
    let _ = <StsCredentials as StsVerifyExt>::verify;
    let _ = [0u8; 0].encode_hex::<String>();
}

// ===========================================================================
// A1 – A4 测试
// ===========================================================================

#[cfg(test)]
mod t24_sts_sm2_tests {
    use super::*;
    use crate::sm2_sign::{Sm2Sk};
    use rand::RngCore;

    const KEY: &[u8] = b"sts-test-key-000000000000000000000000";

    fn gen_keypair() -> ([u8; 65], [u8; 32]) {
        let mut rng = rand::thread_rng();
        let sk = Sm2Sk::generate(&mut rng);
        let pk = sk.public_key();
        (pk.as_uncompressed_bytes(), sk.0)
    }

    fn build_svc_with_role(role_id: &str) -> (StsServiceSm2, [u8; 65], [u8; 32]) {
        let (pk, sk) = gen_keypair();
        let mut ks = Sm2RoleKeystore::new();
        ks.register(role_id, pk, sk).unwrap();
        (StsServiceSm2::with_keystore(KEY, ks), pk, sk)
    }

    // ------------------------------------------------------------------
    // A1. 100 × random user + payload → assume_role + verify 都通过
    // ------------------------------------------------------------------
    #[test]
    fn t24_sts_sm2_dual_sign_100() {
        let mut rng = rand::thread_rng();
        for i in 0..100 {
            let role_id = format!("role-signed-{i}");
            let (svc, _, _) = build_svc_with_role(&role_id);
            let mut ub = [0u8; 16];
            rng.fill_bytes(&mut ub);
            let user_id = format!("user-{}", hex::encode(&ub));
            let mut sb = [0u8; 12];
            rng.fill_bytes(&mut sb);
            let session_name = format!("sess-{}", hex::encode(&sb));

            let r = svc.assume_role(&role_id, &session_name, &user_id, 900).unwrap();
            let sig = r.sm2_signature_hex.clone().expect("A1: sig must exist for registered role");
            assert_eq!(sig.len(), 128, "A1 i={i}: sig hex len should be 128 (r||s raw)");

            // Path (a) — via result convenience wrapper
            let ok_wrap = r.verify_sm2(&svc).expect("A1 i={i}: verify_sm2 should not error");
            assert!(ok_wrap, "A1 i={i}: verify_sm2 returned Ok(false) unexpectedly");

            // Path (b) — explicit service call
            let ok_svc = svc
                .verify_credentials(
                    &r.credentials,
                    &role_id,
                    &session_name,
                    &user_id,
                    None,
                    r.sm2_signature_hex.as_deref(),
                )
                .expect("A1 i={i}: verify_credentials must not error");
            assert!(ok_svc, "A1 i={i}: service verify Ok(false)");

            // Path (c) — Legacy mode (drop SM2 sig, still Ok — backward compat for HMAC base)
            // We use inner directly (no SM2 check):
            let ok_legacy = svc
                .inner()
                .verify_credentials(
                    &r.credentials,
                    &role_id,
                    &session_name,
                    &user_id,
                    None,
                    None,
                )
                .expect("A1 inner legacy verify should not error");
            assert!(ok_legacy, "A1 i={i}: legacy HMAC verify should be true");
        }
    }

    // ------------------------------------------------------------------
    // A2. 100 × 用角色 A 的 sk 签名 → 用角色 B 的 pk 验证 → 失败
    // ------------------------------------------------------------------
    #[test]
    fn t24_sts_sm2_100_pk_mismatch_fail() {
        for i in 0..100 {
            let role_a = format!("role-A-{i}");
            let role_b = format!("role-B-{i}");
            let (_pk_a_bytes, sk_a_bytes) = gen_keypair();
            let (pk_b_bytes, sk_b_bytes) = gen_keypair();

            // Derive pk_a_bytes from sk_a for register
            let pk_a_bytes = {
                let s = Sm2Sk(sk_a_bytes);
                s.public_key().as_uncompressed_bytes()
            };

            // Service with CORRECT role_b pk-bytes/sk-b pair
            let mut ks_correct = Sm2RoleKeystore::new();
            ks_correct.register(&role_b, pk_b_bytes, sk_b_bytes).unwrap();
            let svc_correct = StsServiceSm2::with_keystore(KEY, ks_correct);

            // Malicious issuer — registers role_b under pk=pk_a_bytes, sk=sk_a_bytes
            // so when it signs for role_b, the SM2 sig comes from sk_a (mismatches pk_b).
            let mut ks_bad = Sm2RoleKeystore::new();
            ks_bad.register(&role_b, pk_a_bytes, sk_a_bytes).unwrap();
            let svc_bad = StsServiceSm2::with_keystore(KEY, ks_bad);

            let user_id = format!("user-{i}");
            let session_name = format!("sess-{i}");
            let r_bad = svc_bad
                .assume_role(&role_b, &session_name, &user_id, 900)
                .unwrap();
            let bad_sig = r_bad.sm2_signature_hex.as_deref();

            // session_token HMAC is identical (same KEY + role/session), so HMAC stage passes.
            // SM2 stage must FAIL.
            let res = svc_correct.verify_credentials(
                &r_bad.credentials,
                &role_b,
                &session_name,
                &user_id,
                None,
                bad_sig,
            );
            assert!(
                res.is_err(),
                "A2 i={i}: expect Err when pk mismatches, got {res:?}"
            );
            let msg = res.unwrap_err();
            assert!(
                msg.contains("SM2") && msg.contains("verification"),
                "A2 i={i}: error message should mention SM2 verification, got {msg}"
            );
        }
    }

    // ------------------------------------------------------------------
    // A3. expired_ms = now - 1 → verify 返回 Err
    // ------------------------------------------------------------------
    #[test]
    fn t24_sts_sm2_expired_fail() {
        let (svc, _, _) = build_svc_with_role("role-exp");
        let r = svc.assume_role("role-exp", "sess", "user", 900).unwrap();
        let fake_now = r.credentials.expiration + 1; // 过期 1ms
        let res = svc.verify_credentials(
            &r.credentials,
            "role-exp",
            "sess",
            "user",
            Some(fake_now),
            r.sm2_signature_hex.as_deref(),
        );
        assert!(res.is_err(), "A3 expect Err when token expired, got {res:?}");
        assert!(
            res.unwrap_err().contains("expired"),
            "A3 error message should mention 'expired'"
        );
    }

    // ------------------------------------------------------------------
    // A4. sig_hex 长度 = 128 (r||s 64+64 hex)；总 hex ≥ 160
    // ------------------------------------------------------------------
    #[test]
    fn t24_sts_sm2_len_160_200() {
        let (svc, _, _) = build_svc_with_role("role-len");
        let r = svc
            .assume_role("role-len", "sess-test-len", "user-test-len", 900)
            .unwrap();
        let sig = r.sm2_signature_hex.clone().expect("sig Some");
        assert_eq!(
            sig.len(),
            128,
            "A4: r||s = 32+32 bytes → 64 bytes raw → 128 hex chars, got {}",
            sig.len()
        );
        let total = r.credentials.session_token.len() + sig.len();
        assert!(
            total >= 160,
            "A4: total hex length of session_token ({}) + sig ({}) = {} should be >= 160",
            r.credentials.session_token.len(),
            sig.len(),
            total
        );
        // Also sanity: session_token alone should be base64 of 32 bytes (SHA256) = ~44 chars.
        // 44 + 128 = 172 ≥ 160.
    }
}
