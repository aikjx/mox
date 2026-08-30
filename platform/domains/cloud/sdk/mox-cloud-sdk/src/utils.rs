// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

/// CRC-64/ECMA-182 checksum (poly 0x42F0E1EBA9EA3693, init 0, no tail xor).
pub fn crc64_ecma(mut state: u64, bytes: &[u8]) -> u64 {
    const POLY: u64 = 0x42F0E1EBA9EA3693;
    for &b in bytes {
        state ^= (b as u64) << 56;
        for _ in 0..8 {
            if state & (1u64 << 63) != 0 {
                state = (state << 1) ^ POLY;
            } else {
                state <<= 1;
            }
        }
    }
    state
}

/// Simple deterministic 64-bit hash (FNV-1a 64 variant) to avoid pulling extra crates.
pub(crate) fn fxhash(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub(crate) fn rand_u64() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    // xorshift
    let mut x = t.wrapping_add(Box::into_raw(Box::new(0u8)) as u64);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}
