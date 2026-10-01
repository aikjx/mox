// ====================================================================
// password_hash.rs — 口令哈希统一入口（P0-1 安全加固）
// ====================================================================
// 新口令统一使用 workspace 内 mox-auth-core 的 PBKDF2-HMAC-SHA256（随机 salt、
// 10000 次迭代、恒定时间比较），消除原 auth_session.rs 无盐 SHA-256 弱点。
//
// 兼容与透明升级：
// - 旧存量为 64 位十六进制（无盐 SHA-256）或历史明文：允许登录成功，
//   登录成功后由调用方触发 `needs_upgrade`，写回 PBKDF2 哈希，不锁死老用户。
// - 新口令/改密一律走 PBKDF2。

use mox_auth_core::PasswordManager;
use sha2::{Digest, Sha256};

fn manager() -> PasswordManager {
    PasswordManager::new()
}

/// 哈希结果分类。
pub enum VerifyOutcome {
    /// 校验通过，且已是现代 PBKDF2 哈希。
    Ok,
    /// 校验通过，但属于旧格式，需要透明升级。
    OkLegacy,
    /// 校验失败。
    Fail,
}

/// 为新口令生成 PBKDF2 存储串（`pbkdf2-sha256$iter$salt$hash`）。
pub fn hash_new(password: &str) -> String {
    manager()
        .hash_password(password)
        .map(|h| h.hash)
        .unwrap_or_default()
}

/// 判断存储串是否为旧格式（无盐 SHA-256 hex / 明文），需要升级。
pub fn is_legacy(stored: &str) -> bool {
    !stored.starts_with("pbkdf2-sha256$")
}

/// 校验口令（兼容现代 PBKDF2 与旧无盐 SHA-256 / 明文兜底）。
pub fn verify(password: &str, stored: &str) -> VerifyOutcome {
    if stored.is_empty() {
        return VerifyOutcome::Fail;
    }
    // 现代 PBKDF2
    if stored.starts_with("pbkdf2-sha256$") {
        return match manager().verify_password(password, stored) {
            Ok(true) => VerifyOutcome::Ok,
            _ => VerifyOutcome::Fail,
        };
    }
    // 旧无盐 SHA-256 hex
    if stored.len() == 64 && stored.chars().all(|c| c.is_ascii_hexdigit()) {
        let digest = hex_lower_sha256(password);
        if constant_eq(digest.as_bytes(), stored.as_bytes()) {
            return VerifyOutcome::OkLegacy;
        }
        return VerifyOutcome::Fail;
    }
    // 历史明文兜底（与原 verify_password 行为一致）
    if constant_eq(password.as_bytes(), stored.as_bytes()) {
        return VerifyOutcome::OkLegacy;
    }
    VerifyOutcome::Fail
}

fn hex_lower_sha256(p: &str) -> String {
    let mut h = Sha256::new();
    h.update(p.as_bytes());
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

fn constant_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pbkdf2_hash_verifies_and_is_salted() {
        let h1 = hash_new("CorrectHorse9!");
        let h2 = hash_new("CorrectHorse9!");
        assert!(h1.starts_with("pbkdf2-sha256$"));
        assert_ne!(h1, h2, "同口令两次哈希必须因随机 salt 而不同");
        assert!(matches!(verify("CorrectHorse9!", &h1), VerifyOutcome::Ok));
        assert!(matches!(verify("wrong", &h1), VerifyOutcome::Fail));
    }

    #[test]
    fn legacy_sha256_login_returns_ok_legacy() {
        // 模拟旧库中无盐 SHA-256 存储串
        let old = hex_lower_sha256("OldPassword1");
        assert!(matches!(verify("OldPassword1", &old), VerifyOutcome::OkLegacy));
        assert!(is_legacy(&old));
        assert!(matches!(verify("nope", &old), VerifyOutcome::Fail));
    }

    #[test]
    fn transparent_upgrade_target_is_modern() {
        let old = hex_lower_sha256("OldPassword1");
        let upgraded = hash_new("OldPassword1");
        assert!(!is_legacy(&upgraded));
        assert!(matches!(verify("OldPassword1", &upgraded), VerifyOutcome::Ok));
        assert!(matches!(verify("OldPassword1", &old), VerifyOutcome::OkLegacy));
    }
}
