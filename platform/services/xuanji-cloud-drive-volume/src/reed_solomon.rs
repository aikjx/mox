//! Full Reed-Solomon erasure codec over GF(2^8).  Keeps the legacy
//! `ReedSolomon2Plus1` XOR engine and exposes the new
//! `ReedSolomonEngine` (Vandermonde + Gauss-Jordan in GF(2^8)).

use bytes::Bytes;
use parking_lot::Mutex;
use std::fmt;
use std::time::Instant;

pub use crate::metrics::{observe_encode_us, SHARDS_LOST_TOTAL};
pub use crate::profile::EcProfile;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RSError {
    TooManyShardsMissing(String),
    ShardSizeMismatch(String),
    InvalidInput(String),
    MatrixSingular(String),
}

impl fmt::Display for RSError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RSError::TooManyShardsMissing(m) => write!(f, "Too many shards missing: {}", m),
            RSError::ShardSizeMismatch(m) => write!(f, "Shard size mismatch: {}", m),
            RSError::InvalidInput(m) => write!(f, "Invalid input: {}", m),
            RSError::MatrixSingular(m) => write!(f, "Matrix singular: {}", m),
        }
    }
}

impl std::error::Error for RSError {}
pub type RSResult<T> = Result<T, RSError>;

// ---------------------------------------------------------------------------
// GF(2^8) – poly 0x11d, primitive element α = 2.
// ---------------------------------------------------------------------------

struct GfTables {
    exp: [u8; 512],
    log: [u8; 256],
}

static GF_TABLES: std::sync::OnceLock<GfTables> = std::sync::OnceLock::new();

fn gf() -> &'static GfTables {
    GF_TABLES.get_or_init(|| {
        let mut exp = [0u8; 512];
        let mut log = [0u8; 256];
        let mut x: u16 = 1;
        for i in 0..255 {
            exp[i] = x as u8;
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
pub(crate) fn gf_inv(a: u8) -> u8 {
    debug_assert_ne!(a, 0);
    let t = gf();
    let la = t.log[a as usize] as usize;
    t.exp[255 - la]
}

pub(crate) type Matrix = Vec<Vec<u8>>;

pub(crate) fn build_encoding_matrix(data: usize, total: usize) -> RSResult<Matrix> {
    if total > 255 {
        return Err(RSError::InvalidInput(format!(
            "total shards must be <= 255 for GF(2^8), got {total}"
        )));
    }
    let parity = total - data;
    let mut m: Matrix = vec![vec![0u8; data]; total];
    for i in 0..data {
        m[i][i] = 1;
    }
    let t = gf();
    for r in 0..parity {
        for c in 0..data {
            let exp = (r * c) % 255;
            m[data + r][c] = t.exp[exp];
        }
    }
    Ok(m)
}

pub(crate) fn invert_square(src: &[Vec<u8>]) -> RSResult<Matrix> {
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

struct CachedMatrix {
    data: u16,
    parity: u16,
    matrix: Matrix,
}
static MATRIX_CACHE: Mutex<Vec<CachedMatrix>> = Mutex::new(Vec::new());

pub(crate) fn matrix_for(data: u16, parity: u16) -> RSResult<Matrix> {
    {
        let c = MATRIX_CACHE.lock();
        for e in c.iter() {
            if e.data == data && e.parity == parity {
                return Ok(e.matrix.clone());
            }
        }
    }
    let m = build_encoding_matrix(data as usize, (data + parity) as usize)?;
    let mut c = MATRIX_CACHE.lock();
    c.push(CachedMatrix {
        data,
        parity,
        matrix: m.clone(),
    });
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
    (data_len + data_shards - 1) / data_shards
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
        let data_shard: Vec<&[u8]> = (0..data)
            .map(|c| &padded[c * shard_size..(c + 1) * shard_size])
            .collect();
        for row in 0..total {
            if row < data {
                output[row].copy_from_slice(data_shard[row]);
                continue;
            }
            let enc = &encoder[row];
            let dst = &mut output[row];
            for (c, &coef) in enc.iter().enumerate() {
                if coef == 0 {
                    continue;
                }
                if coef == 1 {
                    for (d, &s) in dst.iter_mut().zip(data_shard[c].iter()) {
                        *d ^= s;
                    }
                } else {
                    for (d, &s) in dst.iter_mut().zip(data_shard[c].iter()) {
                        if s == 0 {
                            continue;
                        }
                        *d ^= gf_mul(coef, s);
                    }
                }
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
        let data = profile.data_shards as usize;
        let parity = profile.parity_shards as usize;
        let total = data + parity;
        if shards.len() != total {
            return Err(RSError::InvalidInput(format!(
                "expected {total} slots, got {}",
                shards.len()
            )));
        }
        let missing: Vec<usize> = shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_none())
            .map(|(i, _)| i)
            .collect();
        let lost = missing.len();
        if lost > parity {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
            return Err(RSError::TooManyShardsMissing(format!(
                "{lost} missing > parity={parity}"
            )));
        }
        if lost > 0 {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
        }
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;
        let mut present_rows: Vec<usize> = shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_some())
            .map(|(i, _)| i)
            .collect();
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
                if coef == 0 {
                    continue;
                }
                let src = &present[y];
                let dst = &mut recovered[x];
                if coef == 1 {
                    for (d, &s) in dst.iter_mut().zip(src.iter()) {
                        *d ^= s;
                    }
                } else {
                    for (d, &s) in dst.iter_mut().zip(src.iter()) {
                        if s == 0 {
                            continue;
                        }
                        *d ^= gf_mul(coef, s);
                    }
                }
            }
        }
        let mut flat = Vec::with_capacity(data * shard_size);
        for d in &recovered {
            flat.extend_from_slice(d);
        }
        flat.truncate(original_len);
        Ok(flat)
    }

    pub fn reconstruct_shards(
        &self,
        profile: &EcProfile,
        shards: &[Option<Vec<u8>>],
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
        let missing: Vec<usize> = shards
            .iter()
            .enumerate()
            .filter(|(_, s)| s.is_none())
            .map(|(i, _)| i)
            .collect();
        let lost = missing.len();
        if lost > parity {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
            return Err(RSError::TooManyShardsMissing(format!(
                "{lost} missing > parity={parity}"
            )));
        }
        if lost > 0 {
            SHARDS_LOST_TOTAL.fetch_add(lost as u64, std::sync::atomic::Ordering::Relaxed);
        }
        let shard_size = shards
            .iter()
            .find_map(|s| s.as_ref().map(|v| v.len()))
            .ok_or_else(|| RSError::InvalidInput("no shard present".into()))?;
        let synthetic = data * shard_size;
        let recovered_data_bytes = self.decode_reconstruct(profile, shards, synthetic)?;
        let encoder = matrix_for(profile.data_shards, profile.parity_shards)?;
        let mut out: Vec<Vec<u8>> = vec![vec![0u8; shard_size]; total];
        for i in 0..data {
            out[i].copy_from_slice(&recovered_data_bytes[i * shard_size..(i + 1) * shard_size]);
        }
        for p in 0..parity {
            let row = &encoder[data + p];
            // split_at_mut so data shard `src` borrows don't conflict with
            // the parity shard dst borrow (data and parity ranges are disjoint).
            let (data_shards, parity_shards) = out.split_at_mut(data);
            let dst = &mut parity_shards[p];
            for (c, &coef) in row.iter().enumerate() {
                if coef == 0 {
                    continue;
                }
                let src = &data_shards[c];
                if coef == 1 {
                    for (d, &s) in dst.iter_mut().zip(src.iter()) {
                        *d ^= s;
                    }
                } else {
                    for (d, &s) in dst.iter_mut().zip(src.iter()) {
                        if s == 0 {
                            continue;
                        }
                        *d ^= gf_mul(coef, s);
                    }
                }
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
            }
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
        let got = engine
            .decode_reconstruct(&profile, &slots, bytes.len())
            .unwrap();
        assert_eq!(got, bytes);
    }
}
