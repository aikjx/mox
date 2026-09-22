// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 一键开关配置：`MOX_API_CRYPTO` / `MOX_API_CRYPTO_KEY`

use std::sync::OnceLock;

/// 加密模式开关（全服务统一约定）
pub const ENV_MODE: &str = "MOX_API_CRYPTO";
/// 32 位 hex（128-bit SM4 密钥）；未设置时使用内置开发密钥
pub const ENV_KEY: &str = "MOX_API_CRYPTO_KEY";

/// 传输加密配置（进程级一次性解析）
#[derive(Clone)]
pub struct CryptoConfig {
    /// 是否启用压缩+加密
    pub enabled: bool,
    /// SM4 128-bit 密钥
    pub key: [u8; 16],
}

impl CryptoConfig {
    /// 纯函数解析（供测试与自定义注入）：mode ∈ {"", "off", 其他=启用}
    pub fn resolve(mode: &str, key_hex: Option<String>) -> Self {
        let enabled = !mode.is_empty() && mode != "off";
        let key = match key_hex.as_deref().and_then(parse_key_hex) {
            Some(k) => k,
            None => {
                if enabled {
                    tracing::warn!(
                        "{ENV_KEY} 未配置或非法，使用内置开发密钥（仅限本地/演示，生产必须显式注入）"
                    );
                }
                DEV_KEY
            }
        };
        Self { enabled, key }
    }

    /// 从环境变量解析（进程内缓存一次）
    pub fn from_env() -> Self {
        static CFG: OnceLock<CryptoConfig> = OnceLock::new();
        CFG.get_or_init(|| {
            Self::resolve(
                &std::env::var(ENV_MODE).unwrap_or_default(),
                std::env::var(ENV_KEY).ok(),
            )
        })
        .clone()
    }
}

/// 内置开发密钥（仅当开关打开但未配置密钥时使用；启动日志会 WARN）
const DEV_KEY: [u8; 16] = *b"mox-dev-sm4-key!";

fn parse_key_hex(hex_str: &str) -> Option<[u8; 16]> {
    let s = hex_str.trim();
    if s.len() != 32 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut key = [0u8; 16];
    for i in 0..16 {
        key[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_and_empty_disable() {
        assert!(!CryptoConfig::resolve("", None).enabled);
        assert!(!CryptoConfig::resolve("off", None).enabled);
        assert!(CryptoConfig::resolve("sm4", None).enabled);
    }

    #[test]
    fn hex_key_parsed_and_bad_falls_back() {
        let cfg = CryptoConfig::resolve("sm4", Some("00112233445566778899aabbccddeeff".into()));
        assert_eq!(cfg.key, [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff
        ]);
        // 非法 hex（长度错/非 hex）回退开发密钥
        assert_eq!(CryptoConfig::resolve("sm4", Some("zz".into())).key, DEV_KEY);
        assert_eq!(CryptoConfig::resolve("sm4", None).key, DEV_KEY);
    }
}
