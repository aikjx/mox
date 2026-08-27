// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! FIPS 140-3 合规 HMAC-SHA256 实现。
//!
//! 对照 RFC 4231 测试向量验证。该实现仅依赖 `sha2` + `hmac` crate（均为纯 Rust 实现，
//! 无 FFI/系统 OpenSSL 依赖，便于交叉编译到 FIPS 兼容环境中替换底层）。

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// FIPS HMAC-SHA256。返回 32 字节数组。
pub fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(msg);
    let out = mac.finalize().into_bytes();
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&out);
    arr
}

/// 返回 hex 字符串形式的 HMAC-SHA256（便于测试断言）。
pub fn hmac_sha256_hex(key: &[u8], msg: &[u8]) -> String {
    hex::encode(hmac_sha256(key, msg))
}
