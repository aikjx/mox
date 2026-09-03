// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Full Reed-Solomon erasure codec over GF(2^8).  Keeps the legacy
//! `ReedSolomon2Plus1` XOR engine and exposes the new
//! `ReedSolomonEngine` (Vandermonde + Gauss-Jordan in GF(2^8)).

use bytes::Bytes;
use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, OnceLock, RwLock},
    time::Instant,
};

pub use crate::{
    metrics::{observe_encode_us, SHARDS_LOST_TOTAL},
    profile::EcProfile,
};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RSError {
    TooManyShardsMissing(String),
    ShardSizeMismatch(String),
    InvalidInput(String),
    MatrixSingular(String),
    ReconstructionVerificationFailed(String),
}

impl fmt::Display for RSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RSError::TooManyShardsMissing(m) => write!(f, "Too many shards missing: {}", m),
            RSError::ShardSizeMismatch(m) => write!(f, "Shard size mismatch: {}", m),
            RSError::InvalidInput(m) => write!(f, "Invalid input: {}", m),
            RSError::MatrixSingular(m) => write!(f, "Matrix singular: {}", m),
            RSError::ReconstructionVerificationFailed(m) => {
                write!(f, "Reconstruction verification failed: {}", m)
            },
        }
    }
}

impl std::error::Error for RSError {}
pub type RSResult<T> = Result<T, RSError>;

// ---------------------------------------------------------------------------
// GF(2^8) – poly 0x11d, primitive element α = 2.
// ---------------------------------------------------------------------------

pub struct GfTables {
    pub exp: [u8; 512],
    pub log: [u8; 256],
}

static GF_TABLES: std::sync::OnceLock<GfTables> = std::sync::OnceLock::new();

pub fn gf() -> &'static GfTables {
    GF_TABLES.get_or_init(|| {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        for (i, exp_item) in exp.iter_mut().enumerate().take(255) {
            *exp_item = x as u8;
            log[x as usize] = i as u8;
            x <<= 1;
            if x & 0x100 != 0 {
                x ^= 0x011d;
            }
        }
        exp[255] = exp[0];
        for i in 256..512 {
            exp[i] = exp[i - 255];
        }
        GfTables { exp, log }
    })
}

#[inline]
pub(crate) fn gf_mul(a: u8, b: u8) -> u8 {
    if a == 0 || b == 0 {
        return 0;
    }
    let t = gf();
    let idx = (t.log[a as usize] as usize) + (t.log[b as usize] as usize);
    t.exp[idx]
}

#[inline]
pub fn gf_inv(a: u8) -> u8 {
    debug_assert_ne!(a, 0);
    let t = gf();
    let la = t.log[a as usize] as usize;
    t.exp[255 - la]
}

// ---------------------------------------------------------------------------
// SIMD path choice & accelerated vector×GF multiply (T22-3)
// ---------------------------------------------------------------------------

/// Runtime path choice for Reed-Solomon matrix operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathChoice {
    /// Runtime-detect AVX2/NEON and prefer SIMD.
    Auto,
    /// Force SIMD kernels (caller guarantees host has support).
    Simd,
    /// Force scalar byte-by-byte path.  Used for tests and for very short
    /// work where the SIMD prologue / tail bookkeeping would dominate.
    Scalar,
}

/// If coef is 0 the result is zero; if 1 it's pure copy/xor; otherwise SIMD
/// is exploited when available.  `dst` is XOR'd with `coef × src`.
#[inline]
pub(crate) fn xor_gf_mul_vec(coef: u8, src: &[u8], dst: &mut [u8], path: PathChoice) {
    debug_assert_eq!(src.len(), dst.len());
    match coef {
        0 => {},
        1 => {
            for (d, &s) in dst.iter_mut().zip(src.iter()) {
                *d ^= s;
            }
        },
        _ => {
            let use_simd = match path {
                PathChoice::Scalar => false,
                PathChoice::Simd => true,
                #[cfg(feature = "simd")]
                PathChoice::Auto => auto_prefers_simd(src.len()),
                #[cfg(not(feature = "simd"))]
                PathChoice::Auto => false,
            };
            if use_simd {
                #[cfg(feature = "simd")]
                crate::gf256_simd::gf_vec_mul_xor_auto(coef, src, dst);
                #[cfg(not(feature = "simd"))]
                {
                    let t = gf();
                    let log_coef = t.log[coef as usize] as usize;
                    for i in 0..src.len() {
                        let s = src[i];
                        if s == 0 {
                            continue;
                        }
                        let idx = log_coef + (t.log[s as usize] as usize);
                        dst[i] ^= t.exp[idx];
                    }
                }
            } else {
                let t = gf();
                let log_coef = t.log[coef as usize] as usize;
                for i in 0..src.len() {
                    let s = src[i];
                    if s == 0 {
                        continue;
                    }
                    let idx = log_coef + (t.log[s as usize] as usize);
                    dst[i] ^= t.exp[idx];
                }
            }
        },
    }
}

/// Decide whether `PathChoice::Auto` should use the SIMD fused mul-xor path vs
/// the scalar fallback on the current host.  A `std::sync::OnceLock` caches
/// the decision across calls so we only pay the microbench cost once.
///
/// Rationale: Our AVX2 implementation uses a 16-deep 256-entry per-lane LUT
/// cascade which is memory bound.  On modern x86_64 CPUs the tuned scalar
/// log/exp table (2 L1 lookups per byte) can outrun the naive vector LUT.  So
/// `Auto` must micro-benchmark on first use to be correct-by-performance.
#[cfg(feature = "simd")]
fn auto_prefers_simd(shard_len_hint: usize) -> bool {
    use std::sync::OnceLock;
    static DECISION: OnceLock<bool> = OnceLock::new();
    *DECISION.get_or_init(|| decide_auto_simd(shard_len_hint))
}

#[cfg(feature = "simd")]
fn decide_auto_simd(hint: usize) -> bool {
    // NEON hosts: we assume the ARM scalar pipeline is relatively weaker so
    // the vector LUT almost always wins; skip bench.
    if crate::gf256_simd::is_neon_supported() && !crate::gf256_simd::is_avx2_supported() {
        return true;
    }
    if !crate::gf256_simd::is_avx2_supported() {
        // No vector ISA available at mox_platform_orchestrator_svc, use scalar.
        return false;
    }
    // AVX2 available: run a 64 KiB microbench comparing SIMD fused vs scalar
    // using coef=17 (a representative non-0/1 coefficient).  If SIMD is not
    // strictly faster, we pick scalar.  Repeat = 7 iterations each; compare
    // median time.
    use std::time::Instant;
    let bench = if hint >= 65536 { 65536 } else { hint.max(4096) };
    let mut src = vec![3u8; bench];
    // Fill with pseudo-random mix so SIMD can't win trivially on zeros.
    let mut acc: u32 = 0x9E37_79B9;
    for b in src.iter_mut() {
        acc = acc.wrapping_mul(2654435761).wrapping_add(acc >> 13);
        *b = acc as u8;
    }
    let mut dst_a = vec![0u8; bench];
    let mut dst_b = vec![0u8; bench];
    const COEF: u8 = 17;
    const ITERS: usize = 7;

    let mut scalar_us = Vec::<u128>::with_capacity(ITERS);
    for _ in 0..ITERS {
        for d in dst_a.iter_mut() {
            *d = 0;
        }
        let t = Instant::now();
        let tables = gf();
        let log_coef = tables.log[COEF as usize] as usize;
        for i in 0..bench {
            let s = src[i];
            if s == 0 {
                continue;
            }
            let idx = log_coef + tables.log[s as usize] as usize;
            dst_a[i] ^= tables.exp[idx];
        }
        scalar_us.push(t.elapsed().as_micros());
    }

    let mut simd_us = Vec::<u128>::with_capacity(ITERS);
    for _ in 0..ITERS {
        for d in dst_b.iter_mut() {
            *d = 0;
        }
        let t = Instant::now();
        crate::gf256_simd::gf_vec_mul_xor_auto(COEF, &src, &mut dst_b);
        simd_us.push(t.elapsed().as_micros());
    }

    scalar_us.sort_unstable();
    simd_us.sort_unstable();
    let s_med = scalar_us[ITERS / 2];
    let a_med = simd_us[ITERS / 2];
    // SIMD wins iff strictly faster; otherwise stick to the battle-tested scalar.
    a_med < s_med && a_med > 0
}

pub type Matrix = Vec<Vec<u8>>;

pub(crate) fn build_encoding_matrix(data: usize, total: usize) -> RSResult<Matrix> {
    if total > 255 {
        return Err(RSError::InvalidInput(format!(
            "total shards must be <= 255 for GF(2^8), got {total}"
        )));
    }
    let parity = total - data;
    let mut m: Matrix = vec![vec![0u8; data]; total];
    for (i, row) in m.iter_mut().enumerate().take(data) {
        row[i] = 1;
    }
    let t = gf();
    for r in 0..parity {
        for (c, cell) in m[data + r].iter_mut().enumerate().take(data) {
            let exp = (r * c) % 255;
            *cell = t.exp[exp];
        }
    }
    Ok(m)
}

pub fn invert_square(src: &[Vec<u8>]) -> RSResult<Matrix> {
    let n = src.len();
    let mut aug = vec![vec![0u8; 2 * n]; n];
    for r in 0..n {
        aug[r][..n].copy_from_slice(&src[r][..n]);
        aug[r][n + r] = 1;
    }
    for col in 0..n {
        let pivot = (col..n)
            .find(|&r| aug[r][col] != 0)
            .ok_or_else(|| RSError::MatrixSingular(format!("zero pivot at col {col}")))?;
        if pivot != col {
            aug.swap(col, pivot);
        }
        let inv = gf_inv(aug[col][col]);
        for j in col..aug[col].len() {
            aug[col][j] = gf_mul(aug[col][j], inv);
        }
        for r in 0..n {
            if r == col || aug[r][col] == 0 {
                continue;
            }
            let factor = aug[r][col];
            for j in col..aug[r].len() {
                aug[r][j] ^= gf_mul(factor, aug[col][j]);
            }
        }
    }
    let mut inv = vec![vec![0u8; n]; n];
    for r in 0..n {
        inv[r].copy_from_slice(&aug[r][n..2 * n]);
    }
    Ok(inv)
}

// ---------------------------------------------------------------------------
// Engine cache
// ---------------------------------------------------------------------------

/// Global cache for Vandermonde encoding matrices, keyed by `(data_shards, parity_shards)`.
///
/// Design reference: RustFS ecstore matrix cache pattern (Apache 2.0).
/// Uses `OnceLock<RwLock<HashMap>>` for O(1) lookup with concurrent read access,
/// and `Arc<Matrix>` to avoid cloning large `Vec<Vec<u8>>` matrices on the hot path.
type MatrixCacheMap = RwLock<HashMap<(u16, u16), Arc<Matrix>>>;
static MATRIX_CACHE: OnceLock<MatrixCacheMap> = OnceLock::new();

#[inline]
fn matrix_cache() -> &'static MatrixCacheMap {
    MATRIX_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Return the encoding matrix for `(data, parity)`, building and caching it on first use.
///
/// Read path takes an `RwLock` read guard and does an O(1) `HashMap` lookup;
/// only the first miss for a given key pays the build cost and the write guard.
/// Double-checked locking under the write guard prevents duplicate builds from
/// concurrent first-time callers.
pub(crate) fn matrix_for(data: u16, parity: u16) -> RSResult<Matrix> {
    let key = (data, parity);
    // Fast path: concurrent read lock + O(1) lookup.
    if let Some(arc) =
        matrix_cache().read().unwrap_or_else(|poisoned| poisoned.into_inner()).get(&key)
    {
        return Ok((**arc).clone());
    }
    // Slow path: build outside the lock, then insert under write lock.
    let m = build_encoding_matrix(data as usize, (data + parity) as usize)?;
    let arc = Arc::new(m.clone());
    let mut guard = matrix_cache().write().unwrap_or_else(|poisoned| poisoned.into_inner());
    // Double-check: another thread may have inserted while we were building.
    if let Some(existing) = guard.get(&key) {
        return Ok((**existing).clone());
    }
    guard.insert(key, arc);
    Ok(m)
}

// ---------------------------------------------------------------------------
// ReedSolomonEngine (public)
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ReedSolomonEngine {
    _priv: (),
}

pub fn shard_size_for(data_shards: usize, data_len: usize) -> usize {
    if data_shards == 0 {
        return 0;
    }
    data_len.div_ceil(data_shards)
}

fn pad_to(input: &[u8], len: usize) -> Vec<u8> {
    if input.len() >= len {
        return input.to_vec();
    }
    let mut v = Vec::with_capacity(len);
    v.extend_from_slice(input);
    v.resize(len, 0);
    v
}

impl ReedSolomonEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode(&self, profile: &EcProfile, data_bytes: &[u8]) -> RSResult<Vec<Vec<u8>>> {
        self.encode_with_path(profile, data_bytes, PathChoice::Auto)
    }

    /// Encode with explicit SIMD/scalar path choice.  Useful for benchmarks
    /// and platforms where mox_platform_orchestrator_svc feature-detection heuristic is wrong.
    pub fn encode_with_path(
        &self,
        profile: &EcProfile,
        data_bytes: &[u8],
        path: PathChoice,
    ) -> RSResult<Vec<Vec<u8>>> {
        let started = Instant::now();
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        if data < 2 || parity < 1 {
            return Err(RSError::InvalidInput(format!(
                "profile data={data} parity={parity} out of range"
            )));
        }
        let total = data + parity;
        let shard_size = shard_size_for(data, data_bytes.len());
        let padded = pad_to(data_bytes, shard_size * data);
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;
        let mut output: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; total];
        let data_shard: Vec<&[u8]> =
            (0..data).map(|c| &padded[c * shard_size..(c + 1) * shard_size]).collect();
        for row in 0..data {
            output[row].copy_from_slice(data_shard[row]);
        }
        for row in data..total {
            let enc = &encoder[row];
            let dst = &mut output[row];
            for (c, &coef) in enc.iter().enumerate() {
                xor_gf_mul_vec(coef, data_shard[c], dst, path);
            }
        }
        let us = started.elapsed().as_micros() as u64;
        observe_encode_us(us);
        Ok(output)
    }

    pub fn decode_reconstruct(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
        original_len: usize,
    ) -> RSResult<Vec<u8>> {
        self.decode_with_path(profile, shards, original_len, PathChoice::Auto)
    }

    pub fn decode_with_path(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
        original_len: usize,
        path: PathChoice,
    ) -> RSResult<Vec<u8>> {
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;
        if shards.len() != total {
            return Err(RSError::InvalidInput(format!(
                "expected {total} slots, got {}",
                shards.len()
            )));
        }
        let missing: Vec<usize> =
            shards.iter().enumerate().filter(|(_, s)| s.is_none()).map(|(i, _)| i).collect();
        let lost = missing.len();
        if lost > parity {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
            return Err(RSError::TooManyShardsMissing(format!("{lost} missing > parity={parity}")));
        }
        if lost > 0 {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
        }
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;
        let mut present_rows: Vec<usize> =
            shards.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i).collect();
        present_rows.truncate(data);
        if present_rows.len() < data {
            return Err(RSError::TooManyShardsMissing(format!(
                "{} present < data_shards={data}",
                present_rows.len()
            )));
        }
        let mut sub: Matrix = vec![vec![0u8; data]; data];
        let mut present: Vec<Vec<u8>> = vec![vec![]; data];
        for (out, &idx) in present_rows.iter().enumerate() {
            sub[out].copy_from_slice(&encoder[idx][..data]);
            present[out] = shards[idx].clone().unwrap();
            if present[out].len() != shard_size {
                return Err(RSError::ShardSizeMismatch(format!(
                    "shard {idx} len {} != {}",
                    present[out].len(),
                    shard_size
                )));
            }
        }
        let inv = invert_square(&sub)?;
        let mut recovered: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; data];
        for x in 0..data {
            for y in 0..data {
                let coef = inv[x][y];
                let src = &present[y];
                let dst = &mut recovered[x];
                xor_gf_mul_vec(coef, src, dst, path);
            }
        }
        let mut flat = Vec::with_capacity(data * shard_size);
        for d in &recovered {
            flat.extend_from_slice(d);
        }
        flat.truncate(original_len);
        Ok(flat)
    }

    /// Reconstruct missing data shards and verify integrity against surplus parity.
    ///
    /// When more than `data_shards` shards are available, the surplus parity shards
    /// provide an independent integrity check: after reconstructing the data from the
    /// first `data_shards` present shards, the parity is recomputed from the recovered
    /// data and compared byte-by-byte with the actual surplus parity shards. Any
    /// mismatch triggers a **fail-closed** error — potentially corrupted data is never
    /// returned.
    ///
    /// When exactly `data_shards` shards are present there is no redundancy, so
    /// verification is skipped and behavior is identical to [`Self::decode_reconstruct`].
    ///
    /// Algorithm reference: RustFS ecstore `decode_data_with_reconstruction_verification`
    /// (Apache 2.0); reimplemented for the self-hosted GF(2^8) engine.
    pub fn decode_with_verification(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
        original_len: usize,
    ) -> RSResult<Vec<u8>> {
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;

        if shards.len() != total {
            return Err(RSError::InvalidInput(format!(
                "expected {total} slots, got {}",
                shards.len()
            )));
        }

        let present_count = shards.iter().filter(|s| s.is_some()).count();

        if present_count < data {
            return Err(RSError::TooManyShardsMissing(format!(
                "{present_count} present < data_shards={data}"
            )));
        }

        // No redundancy available: cannot verify, fall back to plain reconstruct.
        if present_count == data {
            return self.decode_reconstruct(profile, shards, original_len);
        }

        // --- Redundancy available: reconstruct then verify ---

        // Step 1: Reconstruct the original data (uses first `data` present shards).
        let reconstructed = self.decode_reconstruct(profile, shards, original_len)?;

        // Step 2: Determine shard size from the first present shard.
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;

        // Step 3: Pad reconstructed data to full data-region size and split into shards.
        let padded = pad_to(&reconstructed, shard_size * data);
        let data_shard_slices: Vec<&[u8]> =
            (0..data).map(|c| &padded[c * shard_size..(c + 1) * shard_size]).collect();

        // Step 4: Get the encoding matrix for parity recomputation.
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;

        // Step 5: Identify surplus present shards (beyond the first `data` used in
        // reconstruction) and verify each against the re-encoded value.
        let present_indices: Vec<usize> =
            shards.iter().enumerate().filter(|(_, s)| s.is_some()).map(|(i, _)| i).collect();

        for &idx in &present_indices[data..] {
            let expected: Vec<u8> = if idx < data {
                // Surplus data shard: compare directly with recovered data slice.
                padded[idx * shard_size..(idx + 1) * shard_size].to_vec()
            } else {
                // Surplus parity shard: recompute from recovered data via encoding matrix.
                let row = &encoder[idx];
                let mut parity_shard = vec![0u8; shard_size];
                for (c, &coef) in row.iter().enumerate() {
                    xor_gf_mul_vec(coef, data_shard_slices[c], &mut parity_shard, PathChoice::Auto);
                }
                parity_shard
            };

            let actual = shards[idx].as_ref().unwrap();
            if actual.len() != expected.len() || actual != &expected {
                return Err(RSError::ReconstructionVerificationFailed(format!(
                    "shard {idx} mismatch: reconstructed data inconsistent with available parity"
                )));
            }
        }

        Ok(reconstructed)
    }

    pub fn reconstruct_shards(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
    ) -> RSResult<Vec<Vec<u8>>> {
        self.reconstruct_shards_with_path(profile, shards, PathChoice::Auto)
    }

    /// Convenience helper: [`ReedSolomonEngine::reconstruct_shards`] with
    /// explicit [`PathChoice`].  Mirrors the public API but exposes SIMD
    /// override for benchmarks and testing.
    pub fn reconstruct_shards_with_path(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
        path: PathChoice,
    ) -> RSResult<Vec<Vec<u8>>> {
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;
        if shards.len() != total {
            return Err(RSError::InvalidInput(format!(
                "expected {total} slots, got {}",
                shards.len()
            )));
        }
        let missing: Vec<usize> =
            shards.iter().enumerate().filter(|(_, s)| s.is_none()).map(|(i, _)| i).collect();
        let lost = missing.len();
        if lost > parity {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
            return Err(RSError::TooManyShardsMissing(format!("{lost} missing > parity={parity}")));
        }
        if lost > 0 {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
        }
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;
        let synthetic = data * shard_size;
        let recovered_data_bytes = self.decode_with_path(profile, shards, synthetic, path)?;
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;
        let mut out: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; total];
        for i in 0..data {
            out[i].copy_from_slice(&recovered_data_bytes[i * shard_size..(i + 1) * shard_size]);
        }
        for p in 0..parity {
            let row = &encoder[data + p];
            let (data_shards, parity_shards) = out.split_at_mut(data);
            let dst = &mut parity_shards[p];
            for (c, &coef) in row.iter().enumerate() {
                let src = &data_shards[c];
                xor_gf_mul_vec(coef, src, dst, path);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Legacy 2+1 XOR (preserved verbatim).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct ReedSolomon2Plus1;

impl ReedSolomon2Plus1 {
    pub(crate) fn xor_bytes(&self, a: &[u8], b: &[u8]) -> Vec<u8> {
        assert_eq!(a.len(), b.len(), "xor_bytes requires equal length inputs");
        let mut out = Vec::with_capacity(a.len());
        for i in 0..a.len() {
            out.push(a[i] ^ b[i]);
        }
        out
    }

    pub fn encode_2_1(&self, data: &[Bytes; 2]) -> RSResult<[Bytes; 3]> {
        if data[0].len() != data[1].len() {
            return Err(RSError::ShardSizeMismatch(format!(
                "data shards length mismatch: {} vs {}",
                data[0].len(),
                data[1].len()
            )));
        }
        let parity = self.xor_bytes(&data[0], &data[1]);
        Ok([data[0].clone(), data[1].clone(), Bytes::from(parity)])
    }

    pub fn decode_2_1(&self, three_shards: [Option<Bytes>; 3]) -> RSResult<[Bytes; 2]> {
        let missing_count = three_shards.iter().filter(|s| s.is_none()).count();
        if missing_count >= 2 {
            return Err(RSError::TooManyShardsMissing(format!(
                "need at most 1 missing shard, got {missing_count}"
            )));
        }
        let len = three_shards
            .iter()
            .find_map(|s| s.as_ref().map(|b| b.len()))
            .ok_or_else(|| RSError::InvalidInput("all shards None".into()))?;
        let mut owned: [Vec<u8>; 3] = [vec![], vec![], vec![]];
        for i in 0..3 {
            match &three_shards[i] {
                Some(b) => owned[i] = b.to_vec(),
                None => owned[i] = vec![0u8; len],
            }
        }
        match missing_count {
            0 => Ok([Bytes::from(owned[0].clone()), Bytes::from(owned[1].clone())]),
            1 => {
                if three_shards[0].is_none() {
                    let d0 = self.xor_bytes(&owned[1], &owned[2]);
                    Ok([Bytes::from(d0), Bytes::from(owned[1].clone())])
                } else if three_shards[1].is_none() {
                    let d1 = self.xor_bytes(&owned[0], &owned[2]);
                    Ok([Bytes::from(owned[0].clone()), Bytes::from(d1)])
                } else {
                    Ok([Bytes::from(owned[0].clone()), Bytes::from(owned[1].clone())])
                }
            },
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn gf_roundtrip() {
        let t = gf();
        assert_eq!(gf_mul(t.exp[3], t.exp[5]), t.exp[8]);
        for x in 1..=255u8 {
            assert_eq!(gf_mul(x, gf_inv(x)), 1);
        }
    }

    #[test]
    fn encode_and_restore_4plus2() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let bytes = (0..=255u8).cycle().take(1000).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &bytes).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        slots[4] = None;
        let got = engine.decode_reconstruct(&profile, &slots, bytes.len()).unwrap();
        assert_eq!(got, bytes);
    }

    #[test]
    fn test_matrix_cache_optimization() {
        // Same (data, parity) returns identical matrix content.
        let m1 = matrix_for(4, 2).unwrap();
        let m2 = matrix_for(4, 2).unwrap();
        assert_eq!(m1, m2, "cached matrix for (4,2) must be identical");

        // Matrix dimensions match (data+parity) rows x data cols.
        assert_eq!(m1.len(), 6);
        assert_eq!(m1[0].len(), 4);

        // Different (data, parity) returns a differently-shaped matrix.
        let m3 = matrix_for(6, 3).unwrap();
        assert_eq!(m3.len(), 9);
        assert_eq!(m3[0].len(), 6);
        assert_ne!(m1.len(), m3.len());

        // Repeated call returns same content (cache hit, no rebuild divergence).
        let m4 = matrix_for(4, 2).unwrap();
        assert_eq!(m1, m4, "second cache hit must return identical matrix");

        // Vandermonde structure: top-left is identity for data rows.
        for (i, row) in m1.iter().enumerate().take(4) {
            for (j, &cell) in row.iter().enumerate().take(4) {
                assert_eq!(cell, if i == j { 1 } else { 0 });
            }
        }
    }

    #[test]
    fn test_decode_verification_consistent() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let bytes = (0..=255u8).cycle().take(1000).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &bytes).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        // Drop 1 data shard — 5 present > 4 data, so verification runs.
        slots[1] = None;
        let got = engine
            .decode_with_verification(&profile, &slots, bytes.len())
            .expect("consistent shards must pass verification");
        assert_eq!(got, bytes);
    }

    #[test]
    fn test_decode_verification_fail_closed() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let bytes = (0..=255u8).cycle().take(1000).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &bytes).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        // Drop data shard 0 so present = [1,2,3,4,5]; first 4 used = [1,2,3,4],
        // surplus = [5]. Tamper with surplus parity shard 5 to trigger mismatch.
        slots[0] = None;
        if let Some(ref mut p) = slots[5] {
            p[0] ^= 0xFF;
        }
        let result = engine.decode_with_verification(&profile, &slots, bytes.len());
        assert!(result.is_err(), "corrupted surplus parity must fail-closed, not return data");
        assert!(
            matches!(result.unwrap_err(), RSError::ReconstructionVerificationFailed(_)),
            "error must be ReconstructionVerificationFailed"
        );
    }

    #[test]
    fn test_decode_verification_no_redundancy() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let bytes = (0..=255u8).cycle().take(1000).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &bytes).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        // Drop exactly parity=2 shards — 4 present == 4 data, no redundancy.
        slots[1] = None;
        slots[4] = None;
        let got = engine
            .decode_with_verification(&profile, &slots, bytes.len())
            .expect("no-redundancy path must still reconstruct");
        assert_eq!(got, bytes);

        // Must match decode_reconstruct output exactly.
        let got2 = engine.decode_reconstruct(&profile, &slots, bytes.len()).unwrap();
        assert_eq!(got, got2);
    }

    // ── RSError Display ──

    #[test]
    fn test_rs_error_display_all_variants() {
        let e1 = RSError::TooManyShardsMissing("3 > 2".into());
        assert!(format!("{e1}").contains("Too many shards missing"));
        let e2 = RSError::ShardSizeMismatch("100 != 200".into());
        assert!(format!("{e2}").contains("Shard size mismatch"));
        let e3 = RSError::InvalidInput("bad".into());
        assert!(format!("{e3}").contains("Invalid input"));
        let e4 = RSError::MatrixSingular("zero pivot".into());
        assert!(format!("{e4}").contains("Matrix singular"));
        let e5 = RSError::ReconstructionVerificationFailed("mismatch".into());
        assert!(format!("{e5}").contains("Reconstruction verification failed"));
    }

    // ── gf256 arithmetic edge cases ──

    #[test]
    fn test_gf_mul_zero() {
        assert_eq!(gf_mul(0, 255), 0);
        assert_eq!(gf_mul(255, 0), 0);
        assert_eq!(gf_mul(0, 0), 0);
    }

    #[test]
    fn test_gf_mul_identity() {
        for x in 0..=255u8 {
            assert_eq!(gf_mul(x, 1), x);
            assert_eq!(gf_mul(1, x), x);
        }
    }

    #[test]
    fn test_gf_inv_boundary() {
        assert_eq!(gf_inv(1), 1);
        assert_eq!(gf_mul(255, gf_inv(255)), 1);
        for x in 1..=255u8 {
            assert_eq!(gf_mul(x, gf_inv(x)), 1, "inv({x}) invalid");
        }
    }

    #[test]
    fn test_gf_tables_exp_log_consistency() {
        let t = gf();
        assert_eq!(t.exp[0], 1);
        assert_eq!(t.log[1], 0);
        assert_eq!(t.exp[255], 1);
        assert_eq!(t.exp[511], t.exp[256]);
    }

    // ── shard_size_for ──

    #[test]
    fn test_shard_size_for_zero_data_shards() {
        assert_eq!(shard_size_for(0, 1000), 0);
    }

    #[test]
    fn test_shard_size_for_exact_division() {
        assert_eq!(shard_size_for(4, 1000), 250);
    }

    #[test]
    fn test_shard_size_for_ceil_division() {
        assert_eq!(shard_size_for(4, 1001), 251);
    }

    #[test]
    fn test_shard_size_for_zero_data() {
        assert_eq!(shard_size_for(4, 0), 0);
    }

    // ── build_encoding_matrix ──

    #[test]
    fn test_build_encoding_matrix_too_many_shards() {
        let result = build_encoding_matrix(200, 260);
        assert!(matches!(result, Err(RSError::InvalidInput(_))));
    }

    #[test]
    fn test_build_encoding_matrix_identity_top() {
        let m = build_encoding_matrix(4, 6).unwrap();
        assert_eq!(m.len(), 6);
        assert_eq!(m[0].len(), 4);
        for (i, row) in m.iter().enumerate().take(4) {
            for (j, &cell) in row.iter().enumerate().take(4) {
                assert_eq!(cell, if i == j { 1 } else { 0 });
            }
        }
    }

    #[test]
    fn test_build_encoding_matrix_max_valid() {
        let m = build_encoding_matrix(200, 255).unwrap();
        assert_eq!(m.len(), 255);
    }

    // ── invert_square ──

    #[test]
    fn test_invert_square_identity() {
        let identity = vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
        let inv = invert_square(&identity).unwrap();
        assert_eq!(inv, identity);
    }

    #[test]
    fn test_invert_square_singular() {
        let singular = vec![vec![1, 2], vec![1, 2]];
        let result = invert_square(&singular);
        assert!(matches!(result, Err(RSError::MatrixSingular(_))));
    }

    #[test]
    fn test_invert_square_roundtrip() {
        let m = vec![vec![2, 3, 5], vec![7, 11, 13], vec![17, 19, 23]];
        let inv = invert_square(&m).unwrap();
        let inv2 = invert_square(&inv).unwrap();
        assert_eq!(inv2, m);
    }

    #[test]
    fn test_invert_square_1x1() {
        let m = vec![vec![5]];
        let inv = invert_square(&m).unwrap();
        assert_eq!(inv[0][0], gf_inv(5));
    }

    // ── encode edge cases ──

    #[test]
    fn test_encode_empty_data() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let shards = engine.encode(&profile, &[]).unwrap();
        assert_eq!(shards.len(), 6);
        for s in &shards {
            assert_eq!(s.len(), 0);
        }
    }

    #[test]
    fn test_encode_single_byte() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = vec![0xAB];
        let shards = engine.encode(&profile, &data).unwrap();
        assert_eq!(shards.len(), 6);
        let slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        let recovered = engine.decode_reconstruct(&profile, &slots, 1).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_encode_2plus1_minimal() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(2, 1).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        assert_eq!(shards.len(), 3);
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        let recovered = engine.decode_reconstruct(&profile, &slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_encode_max_data_shards_32() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(32, 4).unwrap();
        let data = (0..=255u8).cycle().take(1024).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        assert_eq!(shards.len(), 36);
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        for slot in slots.iter_mut().take(36).skip(32) {
            *slot = None;
        }
        let recovered = engine.decode_reconstruct(&profile, &slots, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_encode_with_scalar_path() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(500).collect::<Vec<_>>();
        let shards_scalar = engine.encode_with_path(&profile, &data, PathChoice::Scalar).unwrap();
        let shards_auto = engine.encode(&profile, &data).unwrap();
        assert_eq!(shards_scalar, shards_auto);
    }

    #[test]
    fn test_encode_with_simd_path() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(500).collect::<Vec<_>>();
        let shards_simd = engine.encode_with_path(&profile, &data, PathChoice::Simd).unwrap();
        let shards_scalar = engine.encode_with_path(&profile, &data, PathChoice::Scalar).unwrap();
        assert_eq!(shards_simd, shards_scalar);
    }

    // ── decode error paths ──

    #[test]
    fn test_decode_shard_count_mismatch() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let slots = vec![Some(vec![0u8; 10]); 3];
        let result = engine.decode_reconstruct(&profile, &slots, 100);
        assert!(matches!(result, Err(RSError::InvalidInput(_))));
    }

    #[test]
    fn test_decode_too_many_missing() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[1] = None;
        slots[2] = None;
        let result = engine.decode_reconstruct(&profile, &slots, data.len());
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_decode_no_shard_present() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let slots = vec![None; 6];
        let result = engine.decode_reconstruct(&profile, &slots, 100);
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_decode_present_less_than_data() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[1] = None;
        slots[2] = None;
        let result = engine.decode_reconstruct(&profile, &slots, data.len());
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_decode_shard_size_mismatch() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        if let Some(ref mut s) = slots[1] {
            s.push(0xFF);
        }
        let result = engine.decode_reconstruct(&profile, &slots, data.len());
        assert!(matches!(result, Err(RSError::ShardSizeMismatch(_))));
    }

    #[test]
    fn test_decode_with_path_scalar() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(500).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[2] = None;
        let recovered = engine
            .decode_with_path(&profile, &slots, data.len(), PathChoice::Scalar)
            .unwrap();
        assert_eq!(recovered, data);
    }

    // ── decode_with_verification error paths ──

    #[test]
    fn test_verify_shard_count_mismatch() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let slots = vec![Some(vec![0u8; 10]); 3];
        let result = engine.decode_with_verification(&profile, &slots, 100);
        assert!(matches!(result, Err(RSError::InvalidInput(_))));
    }

    #[test]
    fn test_verify_present_less_than_data() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[1] = None;
        slots[2] = None;
        let result = engine.decode_with_verification(&profile, &slots, data.len());
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_verify_surplus_parity_mismatch() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(1000).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[5] = None;
        if let Some(ref mut p) = slots[4] {
            p[0] ^= 0xFF;
        }
        let result = engine.decode_with_verification(&profile, &slots, data.len());
        assert!(matches!(result, Err(RSError::ReconstructionVerificationFailed(_))));
    }

    // ── reconstruct_shards ──

    #[test]
    fn test_reconstruct_shards_basic() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(500).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        slots[4] = None;
        let reconstructed = engine.reconstruct_shards(&profile, &slots).unwrap();
        assert_eq!(reconstructed.len(), 6);
        let original = engine.encode(&profile, &data).unwrap();
        for i in 0..4 {
            assert_eq!(reconstructed[i], original[i]);
        }
    }

    #[test]
    fn test_reconstruct_shards_count_mismatch() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let slots = vec![Some(vec![0u8; 10]); 3];
        let result = engine.reconstruct_shards(&profile, &slots);
        assert!(matches!(result, Err(RSError::InvalidInput(_))));
    }

    #[test]
    fn test_reconstruct_shards_too_many_missing() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..100u8).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[0] = None;
        slots[1] = None;
        slots[2] = None;
        let result = engine.reconstruct_shards(&profile, &slots);
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_reconstruct_shards_with_path() {
        let engine = ReedSolomonEngine::new();
        let profile = EcProfile::with_default_min_size(4, 2).unwrap();
        let data = (0..=255u8).cycle().take(500).collect::<Vec<_>>();
        let shards = engine.encode(&profile, &data).unwrap();
        let mut slots: Vec<Option<Vec<u8>>> = shards.into_iter().map(Some).collect();
        slots[1] = None;
        let r_scalar = engine
            .reconstruct_shards_with_path(&profile, &slots, PathChoice::Scalar)
            .unwrap();
        let r_auto = engine.reconstruct_shards(&profile, &slots).unwrap();
        assert_eq!(r_scalar, r_auto);
    }

    // ── ReedSolomon2Plus1 legacy ──

    #[test]
    fn test_rs2plus1_encode_decode() {
        let rs = ReedSolomon2Plus1;
        let d0 = Bytes::from(vec![1u8, 2, 3, 4]);
        let d1 = Bytes::from(vec![5u8, 6, 7, 8]);
        let encoded = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
        assert_eq!(encoded[0], d0);
        assert_eq!(encoded[1], d1);
        assert_eq!(encoded[2], Bytes::from(vec![4u8, 4, 4, 12]));
        let all: [Option<Bytes>; 3] =
            [Some(encoded[0].clone()), Some(encoded[1].clone()), Some(encoded[2].clone())];
        let decoded = rs.decode_2_1(all).unwrap();
        assert_eq!(decoded[0], d0);
        assert_eq!(decoded[1], d1);
    }

    #[test]
    fn test_rs2plus1_encode_size_mismatch() {
        let rs = ReedSolomon2Plus1;
        let d0 = Bytes::from(vec![1u8, 2, 3]);
        let d1 = Bytes::from(vec![5u8, 6]);
        let result = rs.encode_2_1(&[d0, d1]);
        assert!(matches!(result, Err(RSError::ShardSizeMismatch(_))));
    }

    #[test]
    fn test_rs2plus1_decode_missing_d0() {
        let rs = ReedSolomon2Plus1;
        let d0 = Bytes::from(vec![10u8, 20, 30]);
        let d1 = Bytes::from(vec![40u8, 50, 60]);
        let encoded = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
        let slots: [Option<Bytes>; 3] = [None, Some(encoded[1].clone()), Some(encoded[2].clone())];
        let decoded = rs.decode_2_1(slots).unwrap();
        assert_eq!(decoded[0], d0);
        assert_eq!(decoded[1], d1);
    }

    #[test]
    fn test_rs2plus1_decode_missing_d1() {
        let rs = ReedSolomon2Plus1;
        let d0 = Bytes::from(vec![10u8, 20, 30]);
        let d1 = Bytes::from(vec![40u8, 50, 60]);
        let encoded = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
        let slots: [Option<Bytes>; 3] = [Some(encoded[0].clone()), None, Some(encoded[2].clone())];
        let decoded = rs.decode_2_1(slots).unwrap();
        assert_eq!(decoded[0], d0);
        assert_eq!(decoded[1], d1);
    }

    #[test]
    fn test_rs2plus1_decode_missing_parity() {
        let rs = ReedSolomon2Plus1;
        let d0 = Bytes::from(vec![10u8, 20, 30]);
        let d1 = Bytes::from(vec![40u8, 50, 60]);
        let encoded = rs.encode_2_1(&[d0.clone(), d1.clone()]).unwrap();
        let slots: [Option<Bytes>; 3] = [Some(encoded[0].clone()), Some(encoded[1].clone()), None];
        let decoded = rs.decode_2_1(slots).unwrap();
        assert_eq!(decoded[0], d0);
        assert_eq!(decoded[1], d1);
    }

    #[test]
    fn test_rs2plus1_decode_too_many_missing() {
        let rs = ReedSolomon2Plus1;
        let slots: [Option<Bytes>; 3] = [None, None, Some(Bytes::from(vec![1u8, 2]))];
        let result = rs.decode_2_1(slots);
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_rs2plus1_decode_all_none() {
        let rs = ReedSolomon2Plus1;
        let slots: [Option<Bytes>; 3] = [None, None, None];
        let result = rs.decode_2_1(slots);
        assert!(matches!(result, Err(RSError::TooManyShardsMissing(_))));
    }

    #[test]
    fn test_rs2plus1_xor_bytes() {
        let rs = ReedSolomon2Plus1;
        let a = vec![0xFFu8, 0x00, 0xAA];
        let b = vec![0x00u8, 0xFF, 0x55];
        let result = rs.xor_bytes(&a, &b);
        assert_eq!(result, vec![0xFFu8, 0xFF, 0xFF]);
    }

    // ── PathChoice ──

    #[test]
    fn test_path_choice_variants() {
        assert_ne!(PathChoice::Auto, PathChoice::Simd);
        assert_ne!(PathChoice::Simd, PathChoice::Scalar);
        assert_ne!(PathChoice::Auto, PathChoice::Scalar);
    }

    // ── matrix cache concurrent access ──

    #[test]
    fn test_matrix_cache_concurrent() {
        use std::thread;
        let mut handles = Vec::new();
        for _ in 0..8 {
            handles.push(thread::spawn(|| {
                for _ in 0..100 {
                    let m = matrix_for(6, 3).unwrap();
                    assert_eq!(m.len(), 9);
                    assert_eq!(m[0].len(), 6);
                }
            }));
        }
        for h in handles {
            h.join().unwrap();
        }
    }

    // ── xor_gf_mul_vec direct tests ──

    #[test]
    fn test_xor_gf_mul_vec_coef_zero() {
        let src = vec![0xABu8; 64];
        let mut dst = vec![0xFFu8; 64];
        xor_gf_mul_vec(0, &src, &mut dst, PathChoice::Scalar);
        assert!(dst.iter().all(|&b| b == 0xFF));
    }

    #[test]
    fn test_xor_gf_mul_vec_coef_one() {
        let src = vec![0xABu8; 64];
        let mut dst = vec![0x00u8; 64];
        xor_gf_mul_vec(1, &src, &mut dst, PathChoice::Scalar);
        assert!(dst.iter().all(|&b| b == 0xAB));
    }

    #[test]
    fn test_xor_gf_mul_vec_general() {
        let src: Vec<u8> = (0..64u8).collect();
        let mut dst = vec![0u8; 64];
        xor_gf_mul_vec(3, &src, &mut dst, PathChoice::Scalar);
        for i in 0..64 {
            assert_eq!(dst[i], gf_mul(3, src[i]));
        }
    }

    #[test]
    fn test_xor_gf_mul_vec_empty() {
        let src: Vec<u8> = vec![];
        let mut dst: Vec<u8> = vec![];
        xor_gf_mul_vec(5, &src, &mut dst, PathChoice::Scalar);
        assert!(dst.is_empty());
    }

    // ── EcProfile ──

    #[test]
    fn test_ec_profile_total_shards() {
        let p = EcProfile::with_default_min_size(4, 2).unwrap();
        assert_eq!(p.total_shards(), 6);
    }

    #[test]
    fn test_ec_profile_is_replica() {
        let p = EcProfile::with_default_min_size(4, 2).unwrap();
        assert!(p.is_replica(crate::DEFAULT_MIN_OBJ_SIZE - 1));
        assert!(!p.is_replica(crate::DEFAULT_MIN_OBJ_SIZE));
    }

    #[test]
    fn test_ec_profile_default() {
        let p = EcProfile::default();
        assert_eq!(p.data_shards, 4);
        assert_eq!(p.parity_shards, 2);
    }

    #[test]
    fn test_ec_profile_new_valid() {
        let p = EcProfile::new(2, 1, 1000).unwrap();
        assert_eq!(p.data_shards, 2);
        assert_eq!(p.parity_shards, 1);
        assert_eq!(p.min_obj_size, 1000);
    }

    #[test]
    fn test_ec_profile_new_invalid_data() {
        assert!(EcProfile::new(0, 1, 100).is_err());
        assert!(EcProfile::new(1, 1, 100).is_err());
    }

    #[test]
    fn test_ec_profile_new_invalid_parity() {
        assert!(EcProfile::new(4, 0, 100).is_err());
    }
}

// ---------------------------------------------------------------------------
// T22-3 acceptance tests: encode/decode SIMD vs scalar bit-identical.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod t22_rs_simd_tests {
    use super::*;
    use rand::RngCore;

    fn profiles() -> [EcProfile; 3] {
        [
            EcProfile::with_default_min_size(2, 1).unwrap(),
            EcProfile::with_default_min_size(4, 2).unwrap(),
            EcProfile::with_default_min_size(12, 4).unwrap(),
        ]
    }
    fn payloads() -> [usize; 4] {
        [64, 4096, 1_048_576, 16_777_216]
    }

    /// Compare parity shards produced by Scalar path vs Auto (SIMD on supported
    /// hosts).  All data shards + parity shards must be byte-identical.
    #[test]
    fn t22_encode_12plus4_identical_16mb() {
        let mut rng = rand::thread_rng();
        let profile = EcProfile::with_default_min_size(12, 4).unwrap();
        let mut payload = vec![0u8; 16_777_216];
        rng.fill_bytes(&mut payload);
        let eng = ReedSolomonEngine::new();
        let scalar = eng.encode_with_path(&profile, &payload, PathChoice::Scalar).unwrap();
        let auto = eng.encode_with_path(&profile, &payload, PathChoice::Auto).unwrap();
        for i in 0..scalar.len() {
            assert_eq!(scalar[i], auto[i], "SIMD parity mismatch for shard i={} (16MB 12+4)", i);
        }
    }

    /// For 3 profiles × 4 payload sizes × 2 loss patterns: Encode(SIMD-auto)
    /// then drop shards, decode(Scalar) reconstruct identical bytes.
    #[test]
    fn t22_encode_bit_identical_3x4x2_grid() {
        let mut rng = rand::thread_rng();
        let eng = ReedSolomonEngine::new();
        let mut grid_count = 0usize;
        for profile in profiles().iter() {
            for &size in payloads().iter() {
                if profile.data_shards as usize * 64 > size {
                    continue; // skip too-small combos
                }
                let mut payload = vec![0u8; size];
                rng.fill_bytes(&mut payload);
                let shards = eng.encode_with_path(profile, &payload, PathChoice::Auto).unwrap();
                let parity = profile.parity_shards as usize;
                for first_k in [true, false] {
                    let mut slots: Vec<Option<Vec<u8>>> =
                        shards.iter().cloned().map(Some).collect();
                    if first_k {
                        for slot in slots.iter_mut().take(parity) {
                            *slot = None;
                        }
                    } else {
                        // random parity indices plus potentially one data shard inside parity_count.
                        let total = slots.len();
                        let seed = (size ^ (profile.data_shards as usize)) % total;
                        for k in 0..parity {
                            slots[(seed + k) % total] = None;
                        }
                    }
                    let recovered = eng
                        .decode_with_path(profile, &slots, payload.len(), PathChoice::Scalar)
                        .unwrap();
                    assert_eq!(
                        recovered, payload,
                        "profile={}+{} size={} first_k={first_k}",
                        profile.data_shards, profile.parity_shards, size
                    );
                    grid_count += 1;
                }
            }
        }
        // 3 profiles × 4 payloads (min 3 valid sizes) × 2 patterns ≥ 18 combos.
        assert!(grid_count >= 18, "grid_count={grid_count}");
    }

    /// 1000 iterations of dropping random 1..=4 shards from 12+4 and
    /// reconstruct_scalar vs original payload byte equality.
    #[test]
    fn t22_decode_lost_4_reconstruct_identical_1000() {
        let mut rng = rand::thread_rng();
        let profile = EcProfile::with_default_min_size(12, 4).unwrap();
        let eng = ReedSolomonEngine::new();
        let mut payload = vec![0u8; 4 * 1024];
        rng.fill_bytes(&mut payload);
        let shards = eng.encode(&profile, &payload).unwrap();
        let total = shards.len();
        for _ in 0..1000 {
            use rand::seq::SliceRandom;
            let loss = (rng.next_u32() as usize % 4) + 1;
            let mut indices: Vec<usize> = (0..total).collect();
            indices.shuffle(&mut rng);
            let mut slots: Vec<Option<Vec<u8>>> = shards.iter().cloned().map(Some).collect();
            for &idx in indices.iter().take(loss) {
                slots[idx] = None;
            }
            let got = eng.decode_reconstruct(&profile, &slots, payload.len()).unwrap();
            assert_eq!(got, payload, "1000-round mismatch at loss={loss}");
        }
    }
}
