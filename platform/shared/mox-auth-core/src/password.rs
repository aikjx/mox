// =============================================================================
// 密码哈希模块（HMAC-SHA256 + Salt + 迭代）
// =============================================================================

use crate::{AuthError, AuthResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// 密码哈希迭代次数
const PBKDF2_ITERATIONS: u32 = 10_000;

/// 密码哈希结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashedPassword {
    /// 格式: algorithm$iterations$salt_b64$hash_b64
    pub hash: String,
    /// 算法名称
    pub algorithm: String,
}

/// 密码管理器
#[derive(Debug, Clone)]
pub struct PasswordManager {
    iterations: u32,
}

impl Default for PasswordManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PasswordManager {
    pub fn new() -> Self {
        Self {
            iterations: PBKDF2_ITERATIONS,
        }
    }

    pub fn with_iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }

    /// 哈希密码
    pub fn hash_password(&self, password: &str) -> AuthResult<HashedPassword> {
        if password.is_empty() {
            return Err(AuthError::AuthenticationFailed("密码不能为空".to_string()));
        }
        if password.len() < 8 {
            return Err(AuthError::AuthenticationFailed(
                "密码长度至少8位".to_string(),
            ));
        }

        // 生成随机 salt (16 bytes)
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);

        // PBKDF2-HMAC-SHA256
        let hash = self.pbkdf2(password.as_bytes(), &salt);

        // 编码为字符串: algorithm$iterations$salt_b64$hash_b64
        let salt_b64 = STANDARD.encode(salt);
        let hash_b64 = STANDARD.encode(hash);
        let encoded = format!(
            "pbkdf2-sha256${}${}${}",
            self.iterations, salt_b64, hash_b64
        );

        Ok(HashedPassword {
            hash: encoded,
            algorithm: "pbkdf2-sha256".to_string(),
        })
    }

    /// 验证密码
    pub fn verify_password(&self, password: &str, hash: &str) -> AuthResult<bool> {
        // 解析哈希字符串
        let parts: Vec<&str> = hash.split('$').collect();
        if parts.len() != 4 {
            return Err(AuthError::InternalError("哈希格式错误".to_string()));
        }

        let _algorithm = parts[0];
        let iterations: u32 = parts[1]
            .parse()
            .map_err(|e| AuthError::InternalError(format!("迭代次数解析失败: {}", e)))?;
        let salt = STANDARD
            .decode(parts[2])
            .map_err(|e| AuthError::InternalError(format!("Salt 解码失败: {}", e)))?;
        let expected_hash = STANDARD
            .decode(parts[3])
            .map_err(|e| AuthError::InternalError(format!("哈希解码失败: {}", e)))?;

        // 使用相同参数计算哈希
        let actual_hash = self.pbkdf2_with_iterations(password.as_bytes(), &salt, iterations);

        // 常量时间比较
        Ok(constant_time_eq(&expected_hash, &actual_hash))
    }

    /// 验证密码并在失败时返回错误
    pub fn verify_or_error(&self, password: &str, hash: &str) -> AuthResult<()> {
        if self.verify_password(password, hash)? {
            Ok(())
        } else {
            Err(AuthError::WrongPassword)
        }
    }

    /// 检查密码强度
    pub fn check_strength(password: &str) -> PasswordStrength {
        let mut score = 0;

        if password.len() >= 8 {
            score += 1;
        }
        if password.len() >= 12 {
            score += 1;
        }
        if password.chars().any(|c| c.is_uppercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_lowercase()) {
            score += 1;
        }
        if password.chars().any(|c| c.is_ascii_digit()) {
            score += 1;
        }
        if password.chars().any(|c| !c.is_alphanumeric()) {
            score += 1;
        }

        match score {
            0..=1 => PasswordStrength::Weak,
            2..=3 => PasswordStrength::Medium,
            4..=5 => PasswordStrength::Strong,
            _ => PasswordStrength::VeryStrong,
        }
    }

    // ── 内部方法 ──────────────────────────────────────────────────────────

    fn pbkdf2(&self, password: &[u8], salt: &[u8]) -> Vec<u8> {
        self.pbkdf2_with_iterations(password, salt, self.iterations)
    }

    fn pbkdf2_with_iterations(&self, password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
        // 简化的 PBKDF2 实现（单块，32字节输出）
        let mut result = vec![0u8; 32];

        // U1 = HMAC(password, salt || block_index)
        let mut block = salt.to_vec();
        block.extend_from_slice(&1u32.to_be_bytes());

        let mut mac = HmacSha256::new_from_slice(password).expect("HMAC 密钥错误");
        mac.update(&block);
        let mut u = mac.finalize().into_bytes();
        result.copy_from_slice(&u);

        // 后续迭代
        for _ in 1..iterations {
            let mut mac = HmacSha256::new_from_slice(password).expect("HMAC 密钥错误");
            mac.update(&u);
            u = mac.finalize().into_bytes();
            for i in 0..32 {
                result[i] ^= u[i];
            }
        }

        result
    }
}

/// 常量时间比较（防止时序攻击）
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 密码强度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PasswordStrength {
    Weak,
    Medium,
    Strong,
    VeryStrong,
}

impl PasswordStrength {
    pub fn as_str(&self) -> &'static str {
        match self {
            PasswordStrength::Weak => "weak",
            PasswordStrength::Medium => "medium",
            PasswordStrength::Strong => "strong",
            PasswordStrength::VeryStrong => "very_strong",
        }
    }

    pub fn score(&self) -> u8 {
        match self {
            PasswordStrength::Weak => 1,
            PasswordStrength::Medium => 2,
            PasswordStrength::Strong => 3,
            PasswordStrength::VeryStrong => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let manager = PasswordManager::new().with_iterations(100); // 测试用较少迭代
        let password = "MySecurePassword123!";

        let hash = manager.hash_password(password).unwrap();
        assert!(!hash.hash.is_empty());
        assert_eq!(hash.algorithm, "pbkdf2-sha256");
        assert!(hash.hash.starts_with("pbkdf2-sha256$"));

        assert!(manager.verify_password(password, &hash.hash).unwrap());
        assert!(!manager.verify_password("wrongpassword", &hash.hash).unwrap());
    }

    #[test]
    fn test_hash_is_salted() {
        let manager = PasswordManager::new().with_iterations(100);
        let password = "TestPassword123!";

        let hash1 = manager.hash_password(password).unwrap();
        let hash2 = manager.hash_password(password).unwrap();

        // 相同密码的哈希应该不同（因为有随机盐）
        assert_ne!(hash1.hash, hash2.hash);

        // 但两个哈希都能验证原密码
        assert!(manager.verify_password(password, &hash1.hash).unwrap());
        assert!(manager.verify_password(password, &hash2.hash).unwrap());
    }

    #[test]
    fn test_short_password_rejected() {
        let manager = PasswordManager::new();
        assert!(manager.hash_password("short").is_err());
        assert!(manager.hash_password("").is_err());
    }

    #[test]
    fn test_verify_or_error() {
        let manager = PasswordManager::new().with_iterations(100);
        let hash = manager.hash_password("CorrectPassword123!").unwrap();

        assert!(manager.verify_or_error("CorrectPassword123!", &hash.hash).is_ok());
        assert!(matches!(
            manager.verify_or_error("WrongPassword", &hash.hash),
            Err(AuthError::WrongPassword)
        ));
    }

    #[test]
    fn test_password_strength() {
        assert_eq!(
            PasswordManager::check_strength("weak").score(),
            PasswordStrength::Weak.score()
        );
        assert_eq!(
            PasswordManager::check_strength("Medium123").score(),
            PasswordStrength::Strong.score()
        );
        assert_eq!(
            PasswordManager::check_strength("StrongPass123").score(),
            PasswordStrength::Strong.score()
        );
        assert_eq!(
            PasswordManager::check_strength("VeryStrong!Pass123").score(),
            PasswordStrength::VeryStrong.score()
        );
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
