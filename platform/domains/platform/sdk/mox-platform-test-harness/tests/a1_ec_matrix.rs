// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! A1 – Reed-Solomon EC matrix.
//!
//! Test count = 5 profiles × 6 lengths × 5 patterns × 3 seeds = **450 tests**.
//!
//! Each test:
//! 1. Seeds `StdRng` with the test-specific seed, fills `payload_len` random bytes.
//! 2. Calls `ReedSolomonEngine::encode` for the (data,parity) profile.
//! 3. Drops shards matching `drop_pattern` (≤ parity shards always).
//! 4. Calls `decode_reconstruct` and asserts output == original bytes.
//! 5. Asserts CRC64/ECMA of the reconstructed payload matches the original.
//!
//! If a (profile, pattern) pair would drop more than `parity` shards, the test
//! is a no-op success (skip) as mandated.

use rand::{rngs::StdRng, RngCore, SeedableRng};
use mox_cloud_volume_svc::profile::EcProfile;
use mox_cloud_volume_svc::reed_solomon::ReedSolomonEngine;
use mox_data_plane_svc::multipart;

/// Inline CRC64/ECMA (compatible with mox_data_plane_svc multipart module).
/// We use the multipart manager approach: upload the data as a single part
/// and extract the resulting CRC64 field. This ensures parity with the
/// manifest-level checksums used in the rest of the platform.
fn crc64_ecma(bytes: &[u8]) -> u64 {
    if bytes.is_empty() {
        // Empty: the multipart manager refuses empty parts so we fall back to
        // completing with at least one non-empty then returning 0 for empty.
        return 0u64;
    }
    let mgr = multipart::MultipartManager::new();
    let id = mgr.create("_", "_", "_");
    let (crc, _etag) = mgr.upload_part(&id, 1, bytes.to_vec()).unwrap();
    crc
}

/// Returns a Vec of shard indices to drop for a given profile + pattern.
/// For patterns that would exceed parity, returns None (caller should skip).
fn pattern_drop_indices(data: u16, parity: u16, pattern: &str) -> Option<Vec<usize>> {
    let data_us = data as usize;
    let parity_us = parity as usize;
    let max_drop = parity_us;
    let drops: Vec<usize> = match pattern {
        "none" => vec![],
        "last_parity" => {
            // drop the last parity shard
            vec![data_us + parity_us - 1]
        }
        "first_data" => {
            // drop first data shard
            vec![0]
        }
        "all_parity" => {
            // drop all parity shards
            (data_us..data_us + parity_us).collect()
        }
        "mix_dp_dp" => {
            // mix: drop first data + second data + first parity + second parity
            // that is: 0, 1 (two data) + data, data+1 (two parity) -> 4 drops
            let mut v = vec![0usize, 1usize];
            v.push(data_us);
            if parity_us > 1 {
                v.push(data_us + 1);
            }
            v
        }
        other => panic!("unknown drop pattern: {other}"),
    };
    if drops.len() > max_drop {
        None
    } else {
        Some(drops)
    }
}

fn make_payload(len: usize, seed: u64) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut out = vec![0u8; len];
    rng.fill_bytes(&mut out);
    out
}

fn run_ec_case(
    data: u16,
    parity: u16,
    payload_len: usize,
    pattern: &str,
    seed: u64,
) {
    let profile = EcProfile::with_default_min_size(data, parity)
        .unwrap_or_else(|e| panic!("invalid profile {data}+{parity}: {e}"));
    let drops_opt = pattern_drop_indices(data, parity, pattern);
    let drops = match drops_opt {
        Some(d) => d,
        None => {
            // Pattern would exceed parity shards: skip as mandated.
            return;
        }
    };
    let engine = ReedSolomonEngine::new();
    let payload = make_payload(payload_len, seed);
    let crc_before = crc64_ecma(&payload);
    let shards = engine
        .encode(&profile, &payload)
        .unwrap_or_else(|e| panic!("encode failed for {data}+{parity} len={payload_len}: {e}"));
    assert_eq!(shards.len(), (data + parity) as usize);
    let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
    for idx in drops.iter().copied() {
        slots[idx] = None;
    }
    let restored = engine
        .decode_reconstruct(&profile, &slots, payload.len())
        .unwrap_or_else(|e| {
            panic!(
                "decode failed for {data}+{parity} len={payload_len} pat={pattern} seed={seed} drops={drops:?}: {e}"
            )
        });
    assert_eq!(restored, payload, "payload mismatch");
    let crc_after = crc64_ecma(&restored);
    assert_eq!(
        crc_before, crc_after,
        "crc64 mismatch: before={:#x} after={:#x}",
        crc_before, crc_after
    );
}

// -------- Macro expansion --------
//
// We expand the cross product:
//   profiles:    (2,1) (4,2) (8,4) (12,4) (16,4)
//   lengths:     10 100 1024 10240 102400 1048576
//   patterns:    none last_parity first_data all_parity mix_dp_dp
//   seeds:       0 1 2
//
// 5 × 6 × 5 × 3 = 450 individual #[test] functions.

macro_rules! gen_ec_tests {
    (
        profiles = [$( ($d:expr, $p:expr) ),* $(,)?];
        lengths  = [$( $len:expr ),* $(,)?];
        patterns = [$( $pat:ident => $pat_str:expr ),* $(,)?];
        seeds    = [$( $seed:expr ),* $(,)?];
    ) => {
        $(
            gen_ec_tests!(@for_profile ($d, $p)
                lengths  = [$( $len ),*];
                patterns = [$( $pat => $pat_str ),*];
                seeds    = [$( $seed ),*];
            );
        )*
    };

    (@for_profile ($d:expr, $p:expr)
        lengths  = [$( $len:expr ),*];
        patterns = [$( $pat:ident => $pat_str:expr ),*];
        seeds    = [$( $seed:expr ),*];
    ) => {
        $(
            gen_ec_tests!(@for_len ($d, $p, $len)
                patterns = [$( $pat => $pat_str ),*];
                seeds    = [$( $seed ),*];
            );
        )*
    };

    (@for_len ($d:expr, $p:expr, $len:expr)
        patterns = [$( $pat:ident => $pat_str:expr ),*];
        seeds    = [$( $seed:expr ),*];
    ) => {
        $(
            gen_ec_tests!(@for_pat ($d, $p, $len, $pat, $pat_str)
                seeds = [$( $seed ),*];
            );
        )*
    };

    (@for_pat ($d:expr, $p:expr, $len:expr, $pat:ident, $pat_str:expr)
        seeds = [$( $seed:expr ),*];
    ) => {
        $(
            paste::paste! {
                #[test]
                fn [< a1_d $d _p $p _len $len _ $pat _seed $seed >]() {
                    run_ec_case($d, $p, $len, $pat_str, $seed);
                }
            }
        )*
    };
}

// paste is not a dependency — we emulate it via concat_idents style by
// duplicating the actual test expansion with string literals. Since stable
// Rust does not have a stable concat_idents, we instead write each entry
// using a fully expanded explicit list of (ident-name, params) tuples.

macro_rules! emit_ec_case {
    ($name:ident, $d:expr, $p:expr, $len:expr, $pat:expr, $seed:expr) => {
        #[test]
        fn $name() {
            run_ec_case($d, $p, $len, $pat, $seed);
        }
    };
}

// The explicit 450 tests generated below. This approach avoids needing a
// `paste` dependency and ensures `cargo test` counts each one individually.

// Profile: (2,1)
emit_ec_case!(a1_d2_p1_len10_none_seed0,           2,1,10,"none",0);
emit_ec_case!(a1_d2_p1_len10_none_seed1,           2,1,10,"none",1);
emit_ec_case!(a1_d2_p1_len10_none_seed2,           2,1,10,"none",2);
emit_ec_case!(a1_d2_p1_len10_last_parity_seed0,    2,1,10,"last_parity",0);
emit_ec_case!(a1_d2_p1_len10_last_parity_seed1,    2,1,10,"last_parity",1);
emit_ec_case!(a1_d2_p1_len10_last_parity_seed2,    2,1,10,"last_parity",2);
emit_ec_case!(a1_d2_p1_len10_first_data_seed0,     2,1,10,"first_data",0);
emit_ec_case!(a1_d2_p1_len10_first_data_seed1,     2,1,10,"first_data",1);
emit_ec_case!(a1_d2_p1_len10_first_data_seed2,     2,1,10,"first_data",2);
emit_ec_case!(a1_d2_p1_len10_all_parity_seed0,     2,1,10,"all_parity",0);
emit_ec_case!(a1_d2_p1_len10_all_parity_seed1,     2,1,10,"all_parity",1);
emit_ec_case!(a1_d2_p1_len10_all_parity_seed2,     2,1,10,"all_parity",2);
emit_ec_case!(a1_d2_p1_len10_mix_dp_dp_seed0,      2,1,10,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len10_mix_dp_dp_seed1,      2,1,10,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len10_mix_dp_dp_seed2,      2,1,10,"mix_dp_dp",2);
emit_ec_case!(a1_d2_p1_len100_none_seed0,          2,1,100,"none",0);
emit_ec_case!(a1_d2_p1_len100_none_seed1,          2,1,100,"none",1);
emit_ec_case!(a1_d2_p1_len100_none_seed2,          2,1,100,"none",2);
emit_ec_case!(a1_d2_p1_len100_last_parity_seed0,   2,1,100,"last_parity",0);
emit_ec_case!(a1_d2_p1_len100_last_parity_seed1,   2,1,100,"last_parity",1);
emit_ec_case!(a1_d2_p1_len100_last_parity_seed2,   2,1,100,"last_parity",2);
emit_ec_case!(a1_d2_p1_len100_first_data_seed0,    2,1,100,"first_data",0);
emit_ec_case!(a1_d2_p1_len100_first_data_seed1,    2,1,100,"first_data",1);
emit_ec_case!(a1_d2_p1_len100_first_data_seed2,    2,1,100,"first_data",2);
emit_ec_case!(a1_d2_p1_len100_all_parity_seed0,    2,1,100,"all_parity",0);
emit_ec_case!(a1_d2_p1_len100_all_parity_seed1,    2,1,100,"all_parity",1);
emit_ec_case!(a1_d2_p1_len100_all_parity_seed2,    2,1,100,"all_parity",2);
emit_ec_case!(a1_d2_p1_len100_mix_dp_dp_seed0,     2,1,100,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len100_mix_dp_dp_seed1,     2,1,100,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len100_mix_dp_dp_seed2,     2,1,100,"mix_dp_dp",2);
emit_ec_case!(a1_d2_p1_len1024_none_seed0,         2,1,1024,"none",0);
emit_ec_case!(a1_d2_p1_len1024_none_seed1,         2,1,1024,"none",1);
emit_ec_case!(a1_d2_p1_len1024_none_seed2,         2,1,1024,"none",2);
emit_ec_case!(a1_d2_p1_len1024_last_parity_seed0,  2,1,1024,"last_parity",0);
emit_ec_case!(a1_d2_p1_len1024_last_parity_seed1,  2,1,1024,"last_parity",1);
emit_ec_case!(a1_d2_p1_len1024_last_parity_seed2,  2,1,1024,"last_parity",2);
emit_ec_case!(a1_d2_p1_len1024_first_data_seed0,   2,1,1024,"first_data",0);
emit_ec_case!(a1_d2_p1_len1024_first_data_seed1,   2,1,1024,"first_data",1);
emit_ec_case!(a1_d2_p1_len1024_first_data_seed2,   2,1,1024,"first_data",2);
emit_ec_case!(a1_d2_p1_len1024_all_parity_seed0,   2,1,1024,"all_parity",0);
emit_ec_case!(a1_d2_p1_len1024_all_parity_seed1,   2,1,1024,"all_parity",1);
emit_ec_case!(a1_d2_p1_len1024_all_parity_seed2,   2,1,1024,"all_parity",2);
emit_ec_case!(a1_d2_p1_len1024_mix_dp_dp_seed0,    2,1,1024,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len1024_mix_dp_dp_seed1,    2,1,1024,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len1024_mix_dp_dp_seed2,    2,1,1024,"mix_dp_dp",2);
emit_ec_case!(a1_d2_p1_len10240_none_seed0,        2,1,10240,"none",0);
emit_ec_case!(a1_d2_p1_len10240_none_seed1,        2,1,10240,"none",1);
emit_ec_case!(a1_d2_p1_len10240_none_seed2,        2,1,10240,"none",2);
emit_ec_case!(a1_d2_p1_len10240_last_parity_seed0, 2,1,10240,"last_parity",0);
emit_ec_case!(a1_d2_p1_len10240_last_parity_seed1, 2,1,10240,"last_parity",1);
emit_ec_case!(a1_d2_p1_len10240_last_parity_seed2, 2,1,10240,"last_parity",2);
emit_ec_case!(a1_d2_p1_len10240_first_data_seed0,  2,1,10240,"first_data",0);
emit_ec_case!(a1_d2_p1_len10240_first_data_seed1,  2,1,10240,"first_data",1);
emit_ec_case!(a1_d2_p1_len10240_first_data_seed2,  2,1,10240,"first_data",2);
emit_ec_case!(a1_d2_p1_len10240_all_parity_seed0,  2,1,10240,"all_parity",0);
emit_ec_case!(a1_d2_p1_len10240_all_parity_seed1,  2,1,10240,"all_parity",1);
emit_ec_case!(a1_d2_p1_len10240_all_parity_seed2,  2,1,10240,"all_parity",2);
emit_ec_case!(a1_d2_p1_len10240_mix_dp_dp_seed0,   2,1,10240,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len10240_mix_dp_dp_seed1,   2,1,10240,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len10240_mix_dp_dp_seed2,   2,1,10240,"mix_dp_dp",2);
emit_ec_case!(a1_d2_p1_len102400_none_seed0,       2,1,102400,"none",0);
emit_ec_case!(a1_d2_p1_len102400_none_seed1,       2,1,102400,"none",1);
emit_ec_case!(a1_d2_p1_len102400_none_seed2,       2,1,102400,"none",2);
emit_ec_case!(a1_d2_p1_len102400_last_parity_seed0,2,1,102400,"last_parity",0);
emit_ec_case!(a1_d2_p1_len102400_last_parity_seed1,2,1,102400,"last_parity",1);
emit_ec_case!(a1_d2_p1_len102400_last_parity_seed2,2,1,102400,"last_parity",2);
emit_ec_case!(a1_d2_p1_len102400_first_data_seed0, 2,1,102400,"first_data",0);
emit_ec_case!(a1_d2_p1_len102400_first_data_seed1, 2,1,102400,"first_data",1);
emit_ec_case!(a1_d2_p1_len102400_first_data_seed2, 2,1,102400,"first_data",2);
emit_ec_case!(a1_d2_p1_len102400_all_parity_seed0, 2,1,102400,"all_parity",0);
emit_ec_case!(a1_d2_p1_len102400_all_parity_seed1, 2,1,102400,"all_parity",1);
emit_ec_case!(a1_d2_p1_len102400_all_parity_seed2, 2,1,102400,"all_parity",2);
emit_ec_case!(a1_d2_p1_len102400_mix_dp_dp_seed0,  2,1,102400,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len102400_mix_dp_dp_seed1,  2,1,102400,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len102400_mix_dp_dp_seed2,  2,1,102400,"mix_dp_dp",2);
emit_ec_case!(a1_d2_p1_len1048576_none_seed0,      2,1,1048576,"none",0);
emit_ec_case!(a1_d2_p1_len1048576_none_seed1,      2,1,1048576,"none",1);
emit_ec_case!(a1_d2_p1_len1048576_none_seed2,      2,1,1048576,"none",2);
emit_ec_case!(a1_d2_p1_len1048576_last_parity_seed0,2,1,1048576,"last_parity",0);
emit_ec_case!(a1_d2_p1_len1048576_last_parity_seed1,2,1,1048576,"last_parity",1);
emit_ec_case!(a1_d2_p1_len1048576_last_parity_seed2,2,1,1048576,"last_parity",2);
emit_ec_case!(a1_d2_p1_len1048576_first_data_seed0, 2,1,1048576,"first_data",0);
emit_ec_case!(a1_d2_p1_len1048576_first_data_seed1, 2,1,1048576,"first_data",1);
emit_ec_case!(a1_d2_p1_len1048576_first_data_seed2, 2,1,1048576,"first_data",2);
emit_ec_case!(a1_d2_p1_len1048576_all_parity_seed0, 2,1,1048576,"all_parity",0);
emit_ec_case!(a1_d2_p1_len1048576_all_parity_seed1, 2,1,1048576,"all_parity",1);
emit_ec_case!(a1_d2_p1_len1048576_all_parity_seed2, 2,1,1048576,"all_parity",2);
emit_ec_case!(a1_d2_p1_len1048576_mix_dp_dp_seed0,  2,1,1048576,"mix_dp_dp",0);
emit_ec_case!(a1_d2_p1_len1048576_mix_dp_dp_seed1,  2,1,1048576,"mix_dp_dp",1);
emit_ec_case!(a1_d2_p1_len1048576_mix_dp_dp_seed2,  2,1,1048576,"mix_dp_dp",2);
// 90 total for (2,1) profile — on to (4,2) — 90 more.
emit_ec_case!(a1_d4_p2_len10_none_seed0,           4,2,10,"none",0);
emit_ec_case!(a1_d4_p2_len10_none_seed1,           4,2,10,"none",1);
emit_ec_case!(a1_d4_p2_len10_none_seed2,           4,2,10,"none",2);
emit_ec_case!(a1_d4_p2_len10_last_parity_seed0,    4,2,10,"last_parity",0);
emit_ec_case!(a1_d4_p2_len10_last_parity_seed1,    4,2,10,"last_parity",1);
emit_ec_case!(a1_d4_p2_len10_last_parity_seed2,    4,2,10,"last_parity",2);
emit_ec_case!(a1_d4_p2_len10_first_data_seed0,     4,2,10,"first_data",0);
emit_ec_case!(a1_d4_p2_len10_first_data_seed1,     4,2,10,"first_data",1);
emit_ec_case!(a1_d4_p2_len10_first_data_seed2,     4,2,10,"first_data",2);
emit_ec_case!(a1_d4_p2_len10_all_parity_seed0,     4,2,10,"all_parity",0);
emit_ec_case!(a1_d4_p2_len10_all_parity_seed1,     4,2,10,"all_parity",1);
emit_ec_case!(a1_d4_p2_len10_all_parity_seed2,     4,2,10,"all_parity",2);
emit_ec_case!(a1_d4_p2_len10_mix_dp_dp_seed0,      4,2,10,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len10_mix_dp_dp_seed1,      4,2,10,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len10_mix_dp_dp_seed2,      4,2,10,"mix_dp_dp",2);
emit_ec_case!(a1_d4_p2_len100_none_seed0,          4,2,100,"none",0);
emit_ec_case!(a1_d4_p2_len100_none_seed1,          4,2,100,"none",1);
emit_ec_case!(a1_d4_p2_len100_none_seed2,          4,2,100,"none",2);
emit_ec_case!(a1_d4_p2_len100_last_parity_seed0,   4,2,100,"last_parity",0);
emit_ec_case!(a1_d4_p2_len100_last_parity_seed1,   4,2,100,"last_parity",1);
emit_ec_case!(a1_d4_p2_len100_last_parity_seed2,   4,2,100,"last_parity",2);
emit_ec_case!(a1_d4_p2_len100_first_data_seed0,    4,2,100,"first_data",0);
emit_ec_case!(a1_d4_p2_len100_first_data_seed1,    4,2,100,"first_data",1);
emit_ec_case!(a1_d4_p2_len100_first_data_seed2,    4,2,100,"first_data",2);
emit_ec_case!(a1_d4_p2_len100_all_parity_seed0,    4,2,100,"all_parity",0);
emit_ec_case!(a1_d4_p2_len100_all_parity_seed1,    4,2,100,"all_parity",1);
emit_ec_case!(a1_d4_p2_len100_all_parity_seed2,    4,2,100,"all_parity",2);
emit_ec_case!(a1_d4_p2_len100_mix_dp_dp_seed0,     4,2,100,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len100_mix_dp_dp_seed1,     4,2,100,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len100_mix_dp_dp_seed2,     4,2,100,"mix_dp_dp",2);
emit_ec_case!(a1_d4_p2_len1024_none_seed0,         4,2,1024,"none",0);
emit_ec_case!(a1_d4_p2_len1024_none_seed1,         4,2,1024,"none",1);
emit_ec_case!(a1_d4_p2_len1024_none_seed2,         4,2,1024,"none",2);
emit_ec_case!(a1_d4_p2_len1024_last_parity_seed0,  4,2,1024,"last_parity",0);
emit_ec_case!(a1_d4_p2_len1024_last_parity_seed1,  4,2,1024,"last_parity",1);
emit_ec_case!(a1_d4_p2_len1024_last_parity_seed2,  4,2,1024,"last_parity",2);
emit_ec_case!(a1_d4_p2_len1024_first_data_seed0,   4,2,1024,"first_data",0);
emit_ec_case!(a1_d4_p2_len1024_first_data_seed1,   4,2,1024,"first_data",1);
emit_ec_case!(a1_d4_p2_len1024_first_data_seed2,   4,2,1024,"first_data",2);
emit_ec_case!(a1_d4_p2_len1024_all_parity_seed0,   4,2,1024,"all_parity",0);
emit_ec_case!(a1_d4_p2_len1024_all_parity_seed1,   4,2,1024,"all_parity",1);
emit_ec_case!(a1_d4_p2_len1024_all_parity_seed2,   4,2,1024,"all_parity",2);
emit_ec_case!(a1_d4_p2_len1024_mix_dp_dp_seed0,    4,2,1024,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len1024_mix_dp_dp_seed1,    4,2,1024,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len1024_mix_dp_dp_seed2,    4,2,1024,"mix_dp_dp",2);
emit_ec_case!(a1_d4_p2_len10240_none_seed0,        4,2,10240,"none",0);
emit_ec_case!(a1_d4_p2_len10240_none_seed1,        4,2,10240,"none",1);
emit_ec_case!(a1_d4_p2_len10240_none_seed2,        4,2,10240,"none",2);
emit_ec_case!(a1_d4_p2_len10240_last_parity_seed0, 4,2,10240,"last_parity",0);
emit_ec_case!(a1_d4_p2_len10240_last_parity_seed1, 4,2,10240,"last_parity",1);
emit_ec_case!(a1_d4_p2_len10240_last_parity_seed2, 4,2,10240,"last_parity",2);
emit_ec_case!(a1_d4_p2_len10240_first_data_seed0,  4,2,10240,"first_data",0);
emit_ec_case!(a1_d4_p2_len10240_first_data_seed1,  4,2,10240,"first_data",1);
emit_ec_case!(a1_d4_p2_len10240_first_data_seed2,  4,2,10240,"first_data",2);
emit_ec_case!(a1_d4_p2_len10240_all_parity_seed0,  4,2,10240,"all_parity",0);
emit_ec_case!(a1_d4_p2_len10240_all_parity_seed1,  4,2,10240,"all_parity",1);
emit_ec_case!(a1_d4_p2_len10240_all_parity_seed2,  4,2,10240,"all_parity",2);
emit_ec_case!(a1_d4_p2_len10240_mix_dp_dp_seed0,   4,2,10240,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len10240_mix_dp_dp_seed1,   4,2,10240,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len10240_mix_dp_dp_seed2,   4,2,10240,"mix_dp_dp",2);
emit_ec_case!(a1_d4_p2_len102400_none_seed0,       4,2,102400,"none",0);
emit_ec_case!(a1_d4_p2_len102400_none_seed1,       4,2,102400,"none",1);
emit_ec_case!(a1_d4_p2_len102400_none_seed2,       4,2,102400,"none",2);
emit_ec_case!(a1_d4_p2_len102400_last_parity_seed0,4,2,102400,"last_parity",0);
emit_ec_case!(a1_d4_p2_len102400_last_parity_seed1,4,2,102400,"last_parity",1);
emit_ec_case!(a1_d4_p2_len102400_last_parity_seed2,4,2,102400,"last_parity",2);
emit_ec_case!(a1_d4_p2_len102400_first_data_seed0, 4,2,102400,"first_data",0);
emit_ec_case!(a1_d4_p2_len102400_first_data_seed1, 4,2,102400,"first_data",1);
emit_ec_case!(a1_d4_p2_len102400_first_data_seed2, 4,2,102400,"first_data",2);
emit_ec_case!(a1_d4_p2_len102400_all_parity_seed0, 4,2,102400,"all_parity",0);
emit_ec_case!(a1_d4_p2_len102400_all_parity_seed1, 4,2,102400,"all_parity",1);
emit_ec_case!(a1_d4_p2_len102400_all_parity_seed2, 4,2,102400,"all_parity",2);
emit_ec_case!(a1_d4_p2_len102400_mix_dp_dp_seed0,  4,2,102400,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len102400_mix_dp_dp_seed1,  4,2,102400,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len102400_mix_dp_dp_seed2,  4,2,102400,"mix_dp_dp",2);
emit_ec_case!(a1_d4_p2_len1048576_none_seed0,      4,2,1048576,"none",0);
emit_ec_case!(a1_d4_p2_len1048576_none_seed1,      4,2,1048576,"none",1);
emit_ec_case!(a1_d4_p2_len1048576_none_seed2,      4,2,1048576,"none",2);
emit_ec_case!(a1_d4_p2_len1048576_last_parity_seed0,4,2,1048576,"last_parity",0);
emit_ec_case!(a1_d4_p2_len1048576_last_parity_seed1,4,2,1048576,"last_parity",1);
emit_ec_case!(a1_d4_p2_len1048576_last_parity_seed2,4,2,1048576,"last_parity",2);
emit_ec_case!(a1_d4_p2_len1048576_first_data_seed0, 4,2,1048576,"first_data",0);
emit_ec_case!(a1_d4_p2_len1048576_first_data_seed1, 4,2,1048576,"first_data",1);
emit_ec_case!(a1_d4_p2_len1048576_first_data_seed2, 4,2,1048576,"first_data",2);
emit_ec_case!(a1_d4_p2_len1048576_all_parity_seed0, 4,2,1048576,"all_parity",0);
emit_ec_case!(a1_d4_p2_len1048576_all_parity_seed1, 4,2,1048576,"all_parity",1);
emit_ec_case!(a1_d4_p2_len1048576_all_parity_seed2, 4,2,1048576,"all_parity",2);
emit_ec_case!(a1_d4_p2_len1048576_mix_dp_dp_seed0,  4,2,1048576,"mix_dp_dp",0);
emit_ec_case!(a1_d4_p2_len1048576_mix_dp_dp_seed1,  4,2,1048576,"mix_dp_dp",1);
emit_ec_case!(a1_d4_p2_len1048576_mix_dp_dp_seed2,  4,2,1048576,"mix_dp_dp",2);
// (8,4) profile — 90 more (270 so far)
emit_ec_case!(a1_d8_p4_len10_none_seed0,           8,4,10,"none",0);
emit_ec_case!(a1_d8_p4_len10_none_seed1,           8,4,10,"none",1);
emit_ec_case!(a1_d8_p4_len10_none_seed2,           8,4,10,"none",2);
emit_ec_case!(a1_d8_p4_len10_last_parity_seed0,    8,4,10,"last_parity",0);
emit_ec_case!(a1_d8_p4_len10_last_parity_seed1,    8,4,10,"last_parity",1);
emit_ec_case!(a1_d8_p4_len10_last_parity_seed2,    8,4,10,"last_parity",2);
emit_ec_case!(a1_d8_p4_len10_first_data_seed0,     8,4,10,"first_data",0);
emit_ec_case!(a1_d8_p4_len10_first_data_seed1,     8,4,10,"first_data",1);
emit_ec_case!(a1_d8_p4_len10_first_data_seed2,     8,4,10,"first_data",2);
emit_ec_case!(a1_d8_p4_len10_all_parity_seed0,     8,4,10,"all_parity",0);
emit_ec_case!(a1_d8_p4_len10_all_parity_seed1,     8,4,10,"all_parity",1);
emit_ec_case!(a1_d8_p4_len10_all_parity_seed2,     8,4,10,"all_parity",2);
emit_ec_case!(a1_d8_p4_len10_mix_dp_dp_seed0,      8,4,10,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len10_mix_dp_dp_seed1,      8,4,10,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len10_mix_dp_dp_seed2,      8,4,10,"mix_dp_dp",2);
emit_ec_case!(a1_d8_p4_len100_none_seed0,          8,4,100,"none",0);
emit_ec_case!(a1_d8_p4_len100_none_seed1,          8,4,100,"none",1);
emit_ec_case!(a1_d8_p4_len100_none_seed2,          8,4,100,"none",2);
emit_ec_case!(a1_d8_p4_len100_last_parity_seed0,   8,4,100,"last_parity",0);
emit_ec_case!(a1_d8_p4_len100_last_parity_seed1,   8,4,100,"last_parity",1);
emit_ec_case!(a1_d8_p4_len100_last_parity_seed2,   8,4,100,"last_parity",2);
emit_ec_case!(a1_d8_p4_len100_first_data_seed0,    8,4,100,"first_data",0);
emit_ec_case!(a1_d8_p4_len100_first_data_seed1,    8,4,100,"first_data",1);
emit_ec_case!(a1_d8_p4_len100_first_data_seed2,    8,4,100,"first_data",2);
emit_ec_case!(a1_d8_p4_len100_all_parity_seed0,    8,4,100,"all_parity",0);
emit_ec_case!(a1_d8_p4_len100_all_parity_seed1,    8,4,100,"all_parity",1);
emit_ec_case!(a1_d8_p4_len100_all_parity_seed2,    8,4,100,"all_parity",2);
emit_ec_case!(a1_d8_p4_len100_mix_dp_dp_seed0,     8,4,100,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len100_mix_dp_dp_seed1,     8,4,100,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len100_mix_dp_dp_seed2,     8,4,100,"mix_dp_dp",2);
emit_ec_case!(a1_d8_p4_len1024_none_seed0,         8,4,1024,"none",0);
emit_ec_case!(a1_d8_p4_len1024_none_seed1,         8,4,1024,"none",1);
emit_ec_case!(a1_d8_p4_len1024_none_seed2,         8,4,1024,"none",2);
emit_ec_case!(a1_d8_p4_len1024_last_parity_seed0,  8,4,1024,"last_parity",0);
emit_ec_case!(a1_d8_p4_len1024_last_parity_seed1,  8,4,1024,"last_parity",1);
emit_ec_case!(a1_d8_p4_len1024_last_parity_seed2,  8,4,1024,"last_parity",2);
emit_ec_case!(a1_d8_p4_len1024_first_data_seed0,   8,4,1024,"first_data",0);
emit_ec_case!(a1_d8_p4_len1024_first_data_seed1,   8,4,1024,"first_data",1);
emit_ec_case!(a1_d8_p4_len1024_first_data_seed2,   8,4,1024,"first_data",2);
emit_ec_case!(a1_d8_p4_len1024_all_parity_seed0,   8,4,1024,"all_parity",0);
emit_ec_case!(a1_d8_p4_len1024_all_parity_seed1,   8,4,1024,"all_parity",1);
emit_ec_case!(a1_d8_p4_len1024_all_parity_seed2,   8,4,1024,"all_parity",2);
emit_ec_case!(a1_d8_p4_len1024_mix_dp_dp_seed0,    8,4,1024,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len1024_mix_dp_dp_seed1,    8,4,1024,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len1024_mix_dp_dp_seed2,    8,4,1024,"mix_dp_dp",2);
emit_ec_case!(a1_d8_p4_len10240_none_seed0,        8,4,10240,"none",0);
emit_ec_case!(a1_d8_p4_len10240_none_seed1,        8,4,10240,"none",1);
emit_ec_case!(a1_d8_p4_len10240_none_seed2,        8,4,10240,"none",2);
emit_ec_case!(a1_d8_p4_len10240_last_parity_seed0, 8,4,10240,"last_parity",0);
emit_ec_case!(a1_d8_p4_len10240_last_parity_seed1, 8,4,10240,"last_parity",1);
emit_ec_case!(a1_d8_p4_len10240_last_parity_seed2, 8,4,10240,"last_parity",2);
emit_ec_case!(a1_d8_p4_len10240_first_data_seed0,  8,4,10240,"first_data",0);
emit_ec_case!(a1_d8_p4_len10240_first_data_seed1,  8,4,10240,"first_data",1);
emit_ec_case!(a1_d8_p4_len10240_first_data_seed2,  8,4,10240,"first_data",2);
emit_ec_case!(a1_d8_p4_len10240_all_parity_seed0,  8,4,10240,"all_parity",0);
emit_ec_case!(a1_d8_p4_len10240_all_parity_seed1,  8,4,10240,"all_parity",1);
emit_ec_case!(a1_d8_p4_len10240_all_parity_seed2,  8,4,10240,"all_parity",2);
emit_ec_case!(a1_d8_p4_len10240_mix_dp_dp_seed0,   8,4,10240,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len10240_mix_dp_dp_seed1,   8,4,10240,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len10240_mix_dp_dp_seed2,   8,4,10240,"mix_dp_dp",2);
emit_ec_case!(a1_d8_p4_len102400_none_seed0,       8,4,102400,"none",0);
emit_ec_case!(a1_d8_p4_len102400_none_seed1,       8,4,102400,"none",1);
emit_ec_case!(a1_d8_p4_len102400_none_seed2,       8,4,102400,"none",2);
emit_ec_case!(a1_d8_p4_len102400_last_parity_seed0,8,4,102400,"last_parity",0);
emit_ec_case!(a1_d8_p4_len102400_last_parity_seed1,8,4,102400,"last_parity",1);
emit_ec_case!(a1_d8_p4_len102400_last_parity_seed2,8,4,102400,"last_parity",2);
emit_ec_case!(a1_d8_p4_len102400_first_data_seed0, 8,4,102400,"first_data",0);
emit_ec_case!(a1_d8_p4_len102400_first_data_seed1, 8,4,102400,"first_data",1);
emit_ec_case!(a1_d8_p4_len102400_first_data_seed2, 8,4,102400,"first_data",2);
emit_ec_case!(a1_d8_p4_len102400_all_parity_seed0, 8,4,102400,"all_parity",0);
emit_ec_case!(a1_d8_p4_len102400_all_parity_seed1, 8,4,102400,"all_parity",1);
emit_ec_case!(a1_d8_p4_len102400_all_parity_seed2, 8,4,102400,"all_parity",2);
emit_ec_case!(a1_d8_p4_len102400_mix_dp_dp_seed0,  8,4,102400,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len102400_mix_dp_dp_seed1,  8,4,102400,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len102400_mix_dp_dp_seed2,  8,4,102400,"mix_dp_dp",2);
emit_ec_case!(a1_d8_p4_len1048576_none_seed0,      8,4,1048576,"none",0);
emit_ec_case!(a1_d8_p4_len1048576_none_seed1,      8,4,1048576,"none",1);
emit_ec_case!(a1_d8_p4_len1048576_none_seed2,      8,4,1048576,"none",2);
emit_ec_case!(a1_d8_p4_len1048576_last_parity_seed0,8,4,1048576,"last_parity",0);
emit_ec_case!(a1_d8_p4_len1048576_last_parity_seed1,8,4,1048576,"last_parity",1);
emit_ec_case!(a1_d8_p4_len1048576_last_parity_seed2,8,4,1048576,"last_parity",2);
emit_ec_case!(a1_d8_p4_len1048576_first_data_seed0, 8,4,1048576,"first_data",0);
emit_ec_case!(a1_d8_p4_len1048576_first_data_seed1, 8,4,1048576,"first_data",1);
emit_ec_case!(a1_d8_p4_len1048576_first_data_seed2, 8,4,1048576,"first_data",2);
emit_ec_case!(a1_d8_p4_len1048576_all_parity_seed0, 8,4,1048576,"all_parity",0);
emit_ec_case!(a1_d8_p4_len1048576_all_parity_seed1, 8,4,1048576,"all_parity",1);
emit_ec_case!(a1_d8_p4_len1048576_all_parity_seed2, 8,4,1048576,"all_parity",2);
emit_ec_case!(a1_d8_p4_len1048576_mix_dp_dp_seed0,  8,4,1048576,"mix_dp_dp",0);
emit_ec_case!(a1_d8_p4_len1048576_mix_dp_dp_seed1,  8,4,1048576,"mix_dp_dp",1);
emit_ec_case!(a1_d8_p4_len1048576_mix_dp_dp_seed2,  8,4,1048576,"mix_dp_dp",2);
// (12,4) profile — 90 more (360 so far)
emit_ec_case!(a1_d12_p4_len10_none_seed0,          12,4,10,"none",0);
emit_ec_case!(a1_d12_p4_len10_none_seed1,          12,4,10,"none",1);
emit_ec_case!(a1_d12_p4_len10_none_seed2,          12,4,10,"none",2);
emit_ec_case!(a1_d12_p4_len10_last_parity_seed0,   12,4,10,"last_parity",0);
emit_ec_case!(a1_d12_p4_len10_last_parity_seed1,   12,4,10,"last_parity",1);
emit_ec_case!(a1_d12_p4_len10_last_parity_seed2,   12,4,10,"last_parity",2);
emit_ec_case!(a1_d12_p4_len10_first_data_seed0,    12,4,10,"first_data",0);
emit_ec_case!(a1_d12_p4_len10_first_data_seed1,    12,4,10,"first_data",1);
emit_ec_case!(a1_d12_p4_len10_first_data_seed2,    12,4,10,"first_data",2);
emit_ec_case!(a1_d12_p4_len10_all_parity_seed0,    12,4,10,"all_parity",0);
emit_ec_case!(a1_d12_p4_len10_all_parity_seed1,    12,4,10,"all_parity",1);
emit_ec_case!(a1_d12_p4_len10_all_parity_seed2,    12,4,10,"all_parity",2);
emit_ec_case!(a1_d12_p4_len10_mix_dp_dp_seed0,     12,4,10,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len10_mix_dp_dp_seed1,     12,4,10,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len10_mix_dp_dp_seed2,     12,4,10,"mix_dp_dp",2);
emit_ec_case!(a1_d12_p4_len100_none_seed0,         12,4,100,"none",0);
emit_ec_case!(a1_d12_p4_len100_none_seed1,         12,4,100,"none",1);
emit_ec_case!(a1_d12_p4_len100_none_seed2,         12,4,100,"none",2);
emit_ec_case!(a1_d12_p4_len100_last_parity_seed0,  12,4,100,"last_parity",0);
emit_ec_case!(a1_d12_p4_len100_last_parity_seed1,  12,4,100,"last_parity",1);
emit_ec_case!(a1_d12_p4_len100_last_parity_seed2,  12,4,100,"last_parity",2);
emit_ec_case!(a1_d12_p4_len100_first_data_seed0,   12,4,100,"first_data",0);
emit_ec_case!(a1_d12_p4_len100_first_data_seed1,   12,4,100,"first_data",1);
emit_ec_case!(a1_d12_p4_len100_first_data_seed2,   12,4,100,"first_data",2);
emit_ec_case!(a1_d12_p4_len100_all_parity_seed0,   12,4,100,"all_parity",0);
emit_ec_case!(a1_d12_p4_len100_all_parity_seed1,   12,4,100,"all_parity",1);
emit_ec_case!(a1_d12_p4_len100_all_parity_seed2,   12,4,100,"all_parity",2);
emit_ec_case!(a1_d12_p4_len100_mix_dp_dp_seed0,    12,4,100,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len100_mix_dp_dp_seed1,    12,4,100,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len100_mix_dp_dp_seed2,    12,4,100,"mix_dp_dp",2);
emit_ec_case!(a1_d12_p4_len1024_none_seed0,        12,4,1024,"none",0);
emit_ec_case!(a1_d12_p4_len1024_none_seed1,        12,4,1024,"none",1);
emit_ec_case!(a1_d12_p4_len1024_none_seed2,        12,4,1024,"none",2);
emit_ec_case!(a1_d12_p4_len1024_last_parity_seed0, 12,4,1024,"last_parity",0);
emit_ec_case!(a1_d12_p4_len1024_last_parity_seed1, 12,4,1024,"last_parity",1);
emit_ec_case!(a1_d12_p4_len1024_last_parity_seed2, 12,4,1024,"last_parity",2);
emit_ec_case!(a1_d12_p4_len1024_first_data_seed0,  12,4,1024,"first_data",0);
emit_ec_case!(a1_d12_p4_len1024_first_data_seed1,  12,4,1024,"first_data",1);
emit_ec_case!(a1_d12_p4_len1024_first_data_seed2,  12,4,1024,"first_data",2);
emit_ec_case!(a1_d12_p4_len1024_all_parity_seed0,  12,4,1024,"all_parity",0);
emit_ec_case!(a1_d12_p4_len1024_all_parity_seed1,  12,4,1024,"all_parity",1);
emit_ec_case!(a1_d12_p4_len1024_all_parity_seed2,  12,4,1024,"all_parity",2);
emit_ec_case!(a1_d12_p4_len1024_mix_dp_dp_seed0,   12,4,1024,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len1024_mix_dp_dp_seed1,   12,4,1024,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len1024_mix_dp_dp_seed2,   12,4,1024,"mix_dp_dp",2);
emit_ec_case!(a1_d12_p4_len10240_none_seed0,       12,4,10240,"none",0);
emit_ec_case!(a1_d12_p4_len10240_none_seed1,       12,4,10240,"none",1);
emit_ec_case!(a1_d12_p4_len10240_none_seed2,       12,4,10240,"none",2);
emit_ec_case!(a1_d12_p4_len10240_last_parity_seed0,12,4,10240,"last_parity",0);
emit_ec_case!(a1_d12_p4_len10240_last_parity_seed1,12,4,10240,"last_parity",1);
emit_ec_case!(a1_d12_p4_len10240_last_parity_seed2,12,4,10240,"last_parity",2);
emit_ec_case!(a1_d12_p4_len10240_first_data_seed0, 12,4,10240,"first_data",0);
emit_ec_case!(a1_d12_p4_len10240_first_data_seed1, 12,4,10240,"first_data",1);
emit_ec_case!(a1_d12_p4_len10240_first_data_seed2, 12,4,10240,"first_data",2);
emit_ec_case!(a1_d12_p4_len10240_all_parity_seed0, 12,4,10240,"all_parity",0);
emit_ec_case!(a1_d12_p4_len10240_all_parity_seed1, 12,4,10240,"all_parity",1);
emit_ec_case!(a1_d12_p4_len10240_all_parity_seed2, 12,4,10240,"all_parity",2);
emit_ec_case!(a1_d12_p4_len10240_mix_dp_dp_seed0,  12,4,10240,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len10240_mix_dp_dp_seed1,  12,4,10240,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len10240_mix_dp_dp_seed2,  12,4,10240,"mix_dp_dp",2);
emit_ec_case!(a1_d12_p4_len102400_none_seed0,      12,4,102400,"none",0);
emit_ec_case!(a1_d12_p4_len102400_none_seed1,      12,4,102400,"none",1);
emit_ec_case!(a1_d12_p4_len102400_none_seed2,      12,4,102400,"none",2);
emit_ec_case!(a1_d12_p4_len102400_last_parity_seed0,12,4,102400,"last_parity",0);
emit_ec_case!(a1_d12_p4_len102400_last_parity_seed1,12,4,102400,"last_parity",1);
emit_ec_case!(a1_d12_p4_len102400_last_parity_seed2,12,4,102400,"last_parity",2);
emit_ec_case!(a1_d12_p4_len102400_first_data_seed0, 12,4,102400,"first_data",0);
emit_ec_case!(a1_d12_p4_len102400_first_data_seed1, 12,4,102400,"first_data",1);
emit_ec_case!(a1_d12_p4_len102400_first_data_seed2, 12,4,102400,"first_data",2);
emit_ec_case!(a1_d12_p4_len102400_all_parity_seed0, 12,4,102400,"all_parity",0);
emit_ec_case!(a1_d12_p4_len102400_all_parity_seed1, 12,4,102400,"all_parity",1);
emit_ec_case!(a1_d12_p4_len102400_all_parity_seed2, 12,4,102400,"all_parity",2);
emit_ec_case!(a1_d12_p4_len102400_mix_dp_dp_seed0,  12,4,102400,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len102400_mix_dp_dp_seed1,  12,4,102400,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len102400_mix_dp_dp_seed2,  12,4,102400,"mix_dp_dp",2);
emit_ec_case!(a1_d12_p4_len1048576_none_seed0,     12,4,1048576,"none",0);
emit_ec_case!(a1_d12_p4_len1048576_none_seed1,     12,4,1048576,"none",1);
emit_ec_case!(a1_d12_p4_len1048576_none_seed2,     12,4,1048576,"none",2);
emit_ec_case!(a1_d12_p4_len1048576_last_parity_seed0,12,4,1048576,"last_parity",0);
emit_ec_case!(a1_d12_p4_len1048576_last_parity_seed1,12,4,1048576,"last_parity",1);
emit_ec_case!(a1_d12_p4_len1048576_last_parity_seed2,12,4,1048576,"last_parity",2);
emit_ec_case!(a1_d12_p4_len1048576_first_data_seed0, 12,4,1048576,"first_data",0);
emit_ec_case!(a1_d12_p4_len1048576_first_data_seed1, 12,4,1048576,"first_data",1);
emit_ec_case!(a1_d12_p4_len1048576_first_data_seed2, 12,4,1048576,"first_data",2);
emit_ec_case!(a1_d12_p4_len1048576_all_parity_seed0, 12,4,1048576,"all_parity",0);
emit_ec_case!(a1_d12_p4_len1048576_all_parity_seed1, 12,4,1048576,"all_parity",1);
emit_ec_case!(a1_d12_p4_len1048576_all_parity_seed2, 12,4,1048576,"all_parity",2);
emit_ec_case!(a1_d12_p4_len1048576_mix_dp_dp_seed0, 12,4,1048576,"mix_dp_dp",0);
emit_ec_case!(a1_d12_p4_len1048576_mix_dp_dp_seed1, 12,4,1048576,"mix_dp_dp",1);
emit_ec_case!(a1_d12_p4_len1048576_mix_dp_dp_seed2, 12,4,1048576,"mix_dp_dp",2);
// (16,4) profile — final 90 (450 total)
emit_ec_case!(a1_d16_p4_len10_none_seed0,          16,4,10,"none",0);
emit_ec_case!(a1_d16_p4_len10_none_seed1,          16,4,10,"none",1);
emit_ec_case!(a1_d16_p4_len10_none_seed2,          16,4,10,"none",2);
emit_ec_case!(a1_d16_p4_len10_last_parity_seed0,   16,4,10,"last_parity",0);
emit_ec_case!(a1_d16_p4_len10_last_parity_seed1,   16,4,10,"last_parity",1);
emit_ec_case!(a1_d16_p4_len10_last_parity_seed2,   16,4,10,"last_parity",2);
emit_ec_case!(a1_d16_p4_len10_first_data_seed0,    16,4,10,"first_data",0);
emit_ec_case!(a1_d16_p4_len10_first_data_seed1,    16,4,10,"first_data",1);
emit_ec_case!(a1_d16_p4_len10_first_data_seed2,    16,4,10,"first_data",2);
emit_ec_case!(a1_d16_p4_len10_all_parity_seed0,    16,4,10,"all_parity",0);
emit_ec_case!(a1_d16_p4_len10_all_parity_seed1,    16,4,10,"all_parity",1);
emit_ec_case!(a1_d16_p4_len10_all_parity_seed2,    16,4,10,"all_parity",2);
emit_ec_case!(a1_d16_p4_len10_mix_dp_dp_seed0,     16,4,10,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len10_mix_dp_dp_seed1,     16,4,10,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len10_mix_dp_dp_seed2,     16,4,10,"mix_dp_dp",2);
emit_ec_case!(a1_d16_p4_len100_none_seed0,         16,4,100,"none",0);
emit_ec_case!(a1_d16_p4_len100_none_seed1,         16,4,100,"none",1);
emit_ec_case!(a1_d16_p4_len100_none_seed2,         16,4,100,"none",2);
emit_ec_case!(a1_d16_p4_len100_last_parity_seed0,  16,4,100,"last_parity",0);
emit_ec_case!(a1_d16_p4_len100_last_parity_seed1,  16,4,100,"last_parity",1);
emit_ec_case!(a1_d16_p4_len100_last_parity_seed2,  16,4,100,"last_parity",2);
emit_ec_case!(a1_d16_p4_len100_first_data_seed0,   16,4,100,"first_data",0);
emit_ec_case!(a1_d16_p4_len100_first_data_seed1,   16,4,100,"first_data",1);
emit_ec_case!(a1_d16_p4_len100_first_data_seed2,   16,4,100,"first_data",2);
emit_ec_case!(a1_d16_p4_len100_all_parity_seed0,   16,4,100,"all_parity",0);
emit_ec_case!(a1_d16_p4_len100_all_parity_seed1,   16,4,100,"all_parity",1);
emit_ec_case!(a1_d16_p4_len100_all_parity_seed2,   16,4,100,"all_parity",2);
emit_ec_case!(a1_d16_p4_len100_mix_dp_dp_seed0,    16,4,100,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len100_mix_dp_dp_seed1,    16,4,100,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len100_mix_dp_dp_seed2,    16,4,100,"mix_dp_dp",2);
emit_ec_case!(a1_d16_p4_len1024_none_seed0,        16,4,1024,"none",0);
emit_ec_case!(a1_d16_p4_len1024_none_seed1,        16,4,1024,"none",1);
emit_ec_case!(a1_d16_p4_len1024_none_seed2,        16,4,1024,"none",2);
emit_ec_case!(a1_d16_p4_len1024_last_parity_seed0, 16,4,1024,"last_parity",0);
emit_ec_case!(a1_d16_p4_len1024_last_parity_seed1, 16,4,1024,"last_parity",1);
emit_ec_case!(a1_d16_p4_len1024_last_parity_seed2, 16,4,1024,"last_parity",2);
emit_ec_case!(a1_d16_p4_len1024_first_data_seed0,  16,4,1024,"first_data",0);
emit_ec_case!(a1_d16_p4_len1024_first_data_seed1,  16,4,1024,"first_data",1);
emit_ec_case!(a1_d16_p4_len1024_first_data_seed2,  16,4,1024,"first_data",2);
emit_ec_case!(a1_d16_p4_len1024_all_parity_seed0,  16,4,1024,"all_parity",0);
emit_ec_case!(a1_d16_p4_len1024_all_parity_seed1,  16,4,1024,"all_parity",1);
emit_ec_case!(a1_d16_p4_len1024_all_parity_seed2,  16,4,1024,"all_parity",2);
emit_ec_case!(a1_d16_p4_len1024_mix_dp_dp_seed0,   16,4,1024,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len1024_mix_dp_dp_seed1,   16,4,1024,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len1024_mix_dp_dp_seed2,   16,4,1024,"mix_dp_dp",2);
emit_ec_case!(a1_d16_p4_len10240_none_seed0,       16,4,10240,"none",0);
emit_ec_case!(a1_d16_p4_len10240_none_seed1,       16,4,10240,"none",1);
emit_ec_case!(a1_d16_p4_len10240_none_seed2,       16,4,10240,"none",2);
emit_ec_case!(a1_d16_p4_len10240_last_parity_seed0,16,4,10240,"last_parity",0);
emit_ec_case!(a1_d16_p4_len10240_last_parity_seed1,16,4,10240,"last_parity",1);
emit_ec_case!(a1_d16_p4_len10240_last_parity_seed2,16,4,10240,"last_parity",2);
emit_ec_case!(a1_d16_p4_len10240_first_data_seed0, 16,4,10240,"first_data",0);
emit_ec_case!(a1_d16_p4_len10240_first_data_seed1, 16,4,10240,"first_data",1);
emit_ec_case!(a1_d16_p4_len10240_first_data_seed2, 16,4,10240,"first_data",2);
emit_ec_case!(a1_d16_p4_len10240_all_parity_seed0, 16,4,10240,"all_parity",0);
emit_ec_case!(a1_d16_p4_len10240_all_parity_seed1, 16,4,10240,"all_parity",1);
emit_ec_case!(a1_d16_p4_len10240_all_parity_seed2, 16,4,10240,"all_parity",2);
emit_ec_case!(a1_d16_p4_len10240_mix_dp_dp_seed0,  16,4,10240,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len10240_mix_dp_dp_seed1,  16,4,10240,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len10240_mix_dp_dp_seed2,  16,4,10240,"mix_dp_dp",2);
emit_ec_case!(a1_d16_p4_len102400_none_seed0,      16,4,102400,"none",0);
emit_ec_case!(a1_d16_p4_len102400_none_seed1,      16,4,102400,"none",1);
emit_ec_case!(a1_d16_p4_len102400_none_seed2,      16,4,102400,"none",2);
emit_ec_case!(a1_d16_p4_len102400_last_parity_seed0,16,4,102400,"last_parity",0);
emit_ec_case!(a1_d16_p4_len102400_last_parity_seed1,16,4,102400,"last_parity",1);
emit_ec_case!(a1_d16_p4_len102400_last_parity_seed2,16,4,102400,"last_parity",2);
emit_ec_case!(a1_d16_p4_len102400_first_data_seed0, 16,4,102400,"first_data",0);
emit_ec_case!(a1_d16_p4_len102400_first_data_seed1, 16,4,102400,"first_data",1);
emit_ec_case!(a1_d16_p4_len102400_first_data_seed2, 16,4,102400,"first_data",2);
emit_ec_case!(a1_d16_p4_len102400_all_parity_seed0, 16,4,102400,"all_parity",0);
emit_ec_case!(a1_d16_p4_len102400_all_parity_seed1, 16,4,102400,"all_parity",1);
emit_ec_case!(a1_d16_p4_len102400_all_parity_seed2, 16,4,102400,"all_parity",2);
emit_ec_case!(a1_d16_p4_len102400_mix_dp_dp_seed0,  16,4,102400,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len102400_mix_dp_dp_seed1,  16,4,102400,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len102400_mix_dp_dp_seed2,  16,4,102400,"mix_dp_dp",2);
emit_ec_case!(a1_d16_p4_len1048576_none_seed0,     16,4,1048576,"none",0);
emit_ec_case!(a1_d16_p4_len1048576_none_seed1,     16,4,1048576,"none",1);
emit_ec_case!(a1_d16_p4_len1048576_none_seed2,     16,4,1048576,"none",2);
emit_ec_case!(a1_d16_p4_len1048576_last_parity_seed0,16,4,1048576,"last_parity",0);
emit_ec_case!(a1_d16_p4_len1048576_last_parity_seed1,16,4,1048576,"last_parity",1);
emit_ec_case!(a1_d16_p4_len1048576_last_parity_seed2,16,4,1048576,"last_parity",2);
emit_ec_case!(a1_d16_p4_len1048576_first_data_seed0, 16,4,1048576,"first_data",0);
emit_ec_case!(a1_d16_p4_len1048576_first_data_seed1, 16,4,1048576,"first_data",1);
emit_ec_case!(a1_d16_p4_len1048576_first_data_seed2, 16,4,1048576,"first_data",2);
emit_ec_case!(a1_d16_p4_len1048576_all_parity_seed0, 16,4,1048576,"all_parity",0);
emit_ec_case!(a1_d16_p4_len1048576_all_parity_seed1, 16,4,1048576,"all_parity",1);
emit_ec_case!(a1_d16_p4_len1048576_all_parity_seed2, 16,4,1048576,"all_parity",2);
emit_ec_case!(a1_d16_p4_len1048576_mix_dp_dp_seed0, 16,4,1048576,"mix_dp_dp",0);
emit_ec_case!(a1_d16_p4_len1048576_mix_dp_dp_seed1, 16,4,1048576,"mix_dp_dp",1);
emit_ec_case!(a1_d16_p4_len1048576_mix_dp_dp_seed2, 16,4,1048576,"mix_dp_dp",2);
