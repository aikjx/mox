// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! EC manifest: the per-object metadata that accompanies every shard set on
//! disk.  The manifest is round-trip (de)serializable via `serde_json`.
//!
//! Also provides a small reference CRC-64/ECMA (poly `0x42F0E1EBA9EA3693`)
//! implementation so we don't need another crate just for a checksum.

use serde::{Deserialize, Serialize};

/// Data / temperature tier for lifecycle hints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum StorageTier {
    /// Hot – low-latency path (default).
    #[default]
    Hot,
    /// Warm – medium-latency, high-capacity tier.
    Warm,
    /// Cold – high-latency, low-cost tier.
    Cold,
    /// Cold / archival – moved after `lifecycle_cold()`.
    Archive,
}


impl std::fmt::Display for StorageTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageTier::Hot => f.write_str("hot"),
            StorageTier::Warm => f.write_str("warm"),
            StorageTier::Cold => f.write_str("cold"),
            StorageTier::Archive => f.write_str("archive"),
        }
    }
}

/// Per-object EC manifest stored alongside shard slices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EcManifest {
    /// Opaque object id (uuid / hex / hash).
    pub oid: String,
    /// Bucket id / prefix that owns the object.
    pub bid: String,
    /// CRC-64/ECMA of the original user bytes (pre-padding).
    pub crc64: u64,
    /// Total shards currently stored (data + parity – equals
    /// `data_shards + parity_shards` after a full encode).
    pub shard_count: u16,
    /// Data shard count used at encode-time.
    pub data_shards: u16,
    /// Parity shard count used at encode-time.
    pub parity_shards: u16,
    /// Creation wall-clock (ms since UNIX epoch).  We use a plain u64 to
    /// avoid pulling `chrono` if the caller doesn't need it.
    pub created_at_ms: u64,
    /// Hot / archive storage tier.
    #[serde(default)]
    pub tier: StorageTier,
    /// Size in bytes of the original user payload before zero-padding the
    /// tail shard.  Used by RebuildJob to strip EC padding and re-verify
    /// `crc64` on the semantic bytes (matches the value written at
    /// encode-time).
    #[serde(default)]
    pub original_size: u64,
}

impl EcManifest {
    /// Returns a new manifest with `tier = Archive`, preserving all other
    /// fields (cheap-copy semantics for the lifecycle "cold down" op).
    pub fn lifecycle_cold(&self) -> Self {
        let mut next = self.clone();
        next.tier = StorageTier::Archive;
        next
    }
}

// ---------- CRC-64/ECMA ----------

/// CRC-64/ECMA-182: poly `0x42F0E1EBA9EA3693`, non-reflected, init `0`, no
/// final XOR.  Matches the well-known checksum vector CRC-64/ECMA("123456789")
/// = `0x6C40DF5F0B497347`.
static CRC64_TAB: std::sync::OnceLock<[u64; 256]> = std::sync::OnceLock::new();

fn crc64_table() -> &'static [u64; 256] {
    CRC64_TAB.get_or_init(|| {
        const POLY: u64 = 0x42F0_E1EB_A9EA_3693;
        let mut tab = [0u64; 256];
        for i in 0..256u64 {
            let mut c = i << 56;
            for _ in 0..8 {
                c = if c & (1u64 << 63) != 0 { (c << 1) ^ POLY } else { c << 1 };
            }
            tab[i as usize] = c;
        }
        tab
    })
}

/// Compute CRC-64/ECMA over a byte slice (streaming friendly).
pub fn crc64_ecma(data: &[u8]) -> u64 {
    crc64_ecma_update(0, data)
}

/// Feed more bytes into an existing CRC-64 state (init state should be `0`).
pub fn crc64_ecma_update(state: u64, data: &[u8]) -> u64 {
    let tab = crc64_table();
    let mut c = state;
    for &b in data {
        let idx = ((c >> 56) as u8 ^ b) as usize;
        c = (c << 8) ^ tab[idx];
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc64_known_vector() {
        // CRC-64/ECMA("123456789") == 0x6C40DF5F0B497347
        let digest = crc64_ecma(b"123456789");
        assert_eq!(digest, 0x6C40DF5F0B497347);
    }

    #[test]
    fn manifest_serde_roundtrip() {
        let m = EcManifest {
            oid: "abc123".into(),
            bid: "buck-1".into(),
            crc64: 0xdead_beef,
            shard_count: 6,
            data_shards: 4,
            parity_shards: 2,
            created_at_ms: 1_700_000_000_000,
            tier: StorageTier::Hot,
            original_size: 99,
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: EcManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert_eq!(back.lifecycle_cold().tier, StorageTier::Archive);
    }
}
