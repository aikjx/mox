// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! SIMD-accelerated GF(2^8) vector multiplication for x86_64 AVX2 and
//! aarch64 NEON, with safe cross-architecture scalar stubs.
//!
//! Algorithm outline (per 32-byte SIMD chunk):
//!   1. 16-subtable LUT cascade (vpshufb / vtbl1) -> log[src_byte]
//!   2. wrapping add + saturation-adjust -> (log[src] + log[coef]) mod 255
//!   3. 16-subtable LUT cascade -> exp[adjusted_sum]
//!   4. Mask bytes whose src == 0 -> force 0 (log[0] artefact would leak)
//!
//! On unsupported targets or when the `simd` feature is off, `gf_vec_mul_auto`
//! transparently falls back to byte-by-byte scalar `gf_mul`.

#![allow(non_camel_case_types)]

/// AVX2 processes 32 bytes per iteration; also the SIMD chunk alignment used
/// by callers of [`gf_vec_mul_auto`] for the SIMD/scalar split.
pub const SIMD_CHUNK: usize = 32;

/// Runtime AVX2 feature detection.
///
/// * On x86_64 → wraps `is_x86_feature_detected!("avx2")` (CPUID leaf 7).
/// * On every other architecture → returns `false` unconditionally.
pub fn is_avx2_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("avx2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Runtime NEON feature detection (aarch64 ASIMD is baseline).
///
/// * On `aarch64` → returns `true` (Advanced SIMD is guaranteed in the
///   AArch64 base architecture).
/// * On every other architecture → returns `false` unconditionally.
pub fn is_neon_supported() -> bool {
    #[cfg(target_arch = "aarch64")]
    {
        true
    }
    #[cfg(not(target_arch = "aarch64"))]
    {
        false
    }
}

// ---------------------------------------------------------------------------
// x86_64-only: intrinsics + AVX2 core
// ---------------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod x86_impl {
    use std::arch::x86_64::*;

    /// 16 sub-tables x 16 bytes.  subs[k][j] = big_table[k * 16 + j].
    pub(crate) type SubLanes16 = [__m128i; 16];

    #[inline]
    pub(crate) unsafe fn build_subtables(big: &[u8]) -> SubLanes16 {
        debug_assert!(big.len() >= 256);
        let mut subs = [_mm_setzero_si128(); 16];
        for k in 0..16 {
            let mut bytes = [0u8; 16];
            for j in 0..16 {
                bytes[j] = big[k * 16 + j];
            }
            subs[k] = _mm_loadu_si128(bytes.as_ptr().cast::<__m128i>());
        }
        subs
    }

    /// 256-entry parallel LUT within one 128-bit lane.
    #[inline]
    pub(crate) unsafe fn lut256_lane(subs: &SubLanes16, idx: __m128i) -> __m128i {
        let lo = _mm_and_si128(idx, _mm_set1_epi8(0x0F));
        let hi_raw = _mm_srli_epi16(idx, 4);
        let hi = _mm_and_si128(hi_raw, _mm_set1_epi8(0x0F));

        let mut result = _mm_setzero_si128();
        for k in 0..16 {
            let cand = _mm_shuffle_epi8(subs[k], lo);
            let k_vec = _mm_set1_epi8(k as i8);
            let mask = _mm_cmpeq_epi8(hi, k_vec);
            result = _mm_or_si128(result, _mm_and_si128(mask, cand));
        }
        result
    }

    /// 256-bit wrapper: process each 128-bit lane independently.
    #[inline]
    pub(crate) unsafe fn lut256_256(subs: &SubLanes16, idx: __m256i) -> __m256i {
        let lo_lane = _mm256_castsi256_si128(idx);
        let hi_lane = _mm256_extractf128_si256(idx, 1);
        let r0 = lut256_lane(subs, lo_lane);
        let r1 = lut256_lane(subs, hi_lane);
        _mm256_insertf128_si256(_mm256_castsi128_si256(r0), r1, 1)
    }

    /// AVX2 vectorised GF(2^8) multiply kernel.
    ///
    /// # Safety
    /// Requires `src.len() == dst.len()`, both lengths divisible by
    /// [`super::SIMD_CHUNK`] (32), and a CPU supporting AVX2.
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn gf_vec_mul_avx2_inner(coef: u8, src: &[u8], dst: &mut [u8]) {
        use crate::reed_solomon::gf;
        use super::SIMD_CHUNK;

        debug_assert_eq!(src.len(), dst.len());
        let len = src.len();
        debug_assert_eq!(len % SIMD_CHUNK, 0);

        // coef == 0 -> dst all zero.
        if coef == 0 {
            let mut off = 0;
            let zero = _mm256_setzero_si256();
            let dst_ptr = dst.as_mut_ptr();
            while off < len {
                _mm256_storeu_si256(dst_ptr.add(off).cast::<__m256i>(), zero);
                off += SIMD_CHUNK;
            }
            return;
        }

        let tables = gf();
        let log_subs = build_subtables(&tables.log);
        let exp_subs = build_subtables(&tables.exp);
        let log_coef = tables.log[coef as usize]; // coef != 0
        let log_coef_vec = _mm256_set1_epi8(log_coef as i8);

        let src_ptr = src.as_ptr();
        let dst_ptr = dst.as_mut_ptr();
        let mut off = 0;
        while off < len {
            let src_vec = _mm256_loadu_si256(src_ptr.add(off).cast::<__m256i>());

            // 1) log[src]
            let log_src = lut256_256(&log_subs, src_vec);

            // 2) Mask bytes whose original src == 0 (log[0] artefact would give
            //    exp[0 + log[coef]] = coef instead of 0).
            let zero_mask = _mm256_cmpeq_epi8(src_vec, _mm256_setzero_si256());

            // 3) wrapping sum of log bytes
            let log_sum_wrap = _mm256_add_epi8(log_src, log_coef_vec);

            // 4) mod 255 adjust: saturating-add saturates at 255 iff true sum >= 255,
            //    then wrapping sum (+ 1 when adjusted) = true_sum mod 255 as u8.
            let log_sum_sat = _mm256_adds_epu8(log_src, log_coef_vec);
            let adjust_mask = _mm256_cmpeq_epi8(log_sum_sat, _mm256_set1_epi8(255u8 as i8));
            let adjust_one = _mm256_and_si256(adjust_mask, _mm256_set1_epi8(1));
            let exp_idx = _mm256_add_epi8(log_sum_wrap, adjust_one);

            // 5) exp lookup
            let exp_raw = lut256_256(&exp_subs, exp_idx);

            // 6) zero mask
            let result = _mm256_andnot_si256(zero_mask, exp_raw);

            _mm256_storeu_si256(dst_ptr.add(off).cast::<__m256i>(), result);
            off += SIMD_CHUNK;
        }
    }

    /// Fused GF(2^8) multiply-XOR over AVX2 `main_len` bytes of `src`/`dst`.
    /// `dst[i] ^= mul(coef, src[i])` for all bytes.
    ///
    /// # Safety
    /// Requires AVX2 support; `main_len` divisible by `SIMD_CHUNK` (32);
    /// caller ensures pointer ranges are valid for read+write as indicated.
    #[target_feature(enable = "avx2")]
    pub(crate) unsafe fn avx2_xor_fused_body(
        coef: u8,
        src_ptr: *const u8,
        dst_ptr: *mut u8,
        main_len: usize,
    ) {
        use crate::reed_solomon::gf;
        use super::SIMD_CHUNK;
        debug_assert_eq!(main_len % SIMD_CHUNK, 0);
        if coef == 0 {
            return;
        }
        let tables = gf();
        let log_subs = build_subtables(&tables.log);
        let exp_subs = build_subtables(&tables.exp);
        let log_coef = tables.log[coef as usize];
        let log_coef_vec = _mm256_set1_epi8(log_coef as i8);

        let mut off = 0;
        while off < main_len {
            let src_vec = _mm256_loadu_si256(src_ptr.add(off).cast::<__m256i>());
            let log_src = lut256_256(&log_subs, src_vec);
            let zero_mask = _mm256_cmpeq_epi8(src_vec, _mm256_setzero_si256());
            let log_sum_wrap = _mm256_add_epi8(log_src, log_coef_vec);
            let log_sum_sat = _mm256_adds_epu8(log_src, log_coef_vec);
            let adjust_mask =
                _mm256_cmpeq_epi8(log_sum_sat, _mm256_set1_epi8(255u8 as i8));
            let adjust_one = _mm256_and_si256(adjust_mask, _mm256_set1_epi8(1));
            let exp_idx = _mm256_add_epi8(log_sum_wrap, adjust_one);
            let exp_raw = lut256_256(&exp_subs, exp_idx);
            let prod = _mm256_andnot_si256(zero_mask, exp_raw);
            let d = _mm256_loadu_si256(dst_ptr.add(off).cast::<__m256i>());
            _mm256_storeu_si256(
                dst_ptr.add(off).cast::<__m256i>(),
                _mm256_xor_si256(d, prod),
            );
            off += SIMD_CHUNK;
        }
    }
}

// ---------------------------------------------------------------------------
// aarch64-only: NEON (ASIMD) core
// ---------------------------------------------------------------------------

#[cfg(target_arch = "aarch64")]
mod neon_impl {
    use std::arch::aarch64::*;

    /// 16 sub-tables x 16 bytes.  Each sub-table is stored as a pair of
    /// uint8x8_t halves, which is the lane width `vtbl1_u8` operates on.
    pub(crate) type SubLanes16 = [[uint8x8_t; 2]; 16];

    #[inline]
    pub(crate) unsafe fn build_subtables(big: &[u8]) -> SubLanes16 {
        debug_assert!(big.len() >= 256);
        let mut subs = [[vmov_n_u8(0); 2]; 16];
        for k in 0..16 {
            let mut bytes = [0u8; 16];
            for j in 0..16 {
                bytes[j] = big[k * 16 + j];
            }
            subs[k][0] = vld1_u8(bytes.as_ptr());
            subs[k][1] = vld1_u8(bytes.as_ptr().add(8));
        }
        subs
    }

    /// 256-entry parallel LUT over a 128-bit NEON lane.
    ///
    /// Low/high nibble cascade with vtbl1_u8 (8-wide).  We process the
    /// 16-byte lane as two 8-byte halves and reinterleave the result.
    #[inline]
    pub(crate) unsafe fn lut256_lane(subs: &SubLanes16, idx: uint8x16_t) -> uint8x16_t {
        let idx_lo = vget_low_u8(idx);
        let idx_hi = vget_high_u8(idx);
        let lo_lo = vand_u8(idx_lo, vmov_n_u8(0x0F));
        let lo_hi = vand_u8(vshr_n_u8(idx_lo, 4), vmov_n_u8(0x0F));
        let hi_lo = vand_u8(idx_hi, vmov_n_u8(0x0F));
        let hi_hi = vand_u8(vshr_n_u8(idx_hi, 4), vmov_n_u8(0x0F));

        // For each of the two 8-byte halves, use the top nibble (0..15) to
        // pick which sub-table half to vtbl1, then combine into 16-byte res.
        let sub_half = |nib_lo: uint8x8_t, nib_hi: uint8x8_t| -> uint8x8_t {
            let mut r = vmov_n_u8(0);
            for k in 0..16usize {
                // lower 8 bytes of sub[k] → half=0, upper 8 → half=1
                // nibble hi == k → we read sub k, use the matching 8-entry
                // half of sub[k] for this index's 8-byte lane. Since both
                // lanes of the 16-byte sub share the same hi-nibble cascade
                // we pick up the matching half in each 8-byte group.
                let cand_0 = vtbl1_u8(subs[k][0], nib_lo);
                let cand_1 = vtbl1_u8(subs[k][1], nib_lo);
                let k_vec = vmov_n_u8(k as u8);
                let mask = vceq_u8(nib_hi, k_vec);
                let nib_lo_ge8 = vcge_u8(nib_lo, vmov_n_u8(8));
                let cand = vbsl_u8(nib_lo_ge8, cand_1, cand_0);
                r = vorr_u8(r, vand_u8(mask, cand));
            }
            r
        };

        let r_lo = sub_half(lo_lo, lo_hi);
        let r_hi = sub_half(hi_lo, hi_hi);
        vcombine_u8(r_lo, r_hi)
    }

    /// NEON vectorised GF(2^8) multiply kernel.
    ///
    /// One [`super::SIMD_CHUNK`] = 32 bytes → two 128-bit lanes processed
    /// with the same sub-tables.
    ///
    /// # Safety
    /// Requires `src.len() == dst.len()` and both lengths divisible by 32.
    /// Executes unconditionally on aarch64 (ASIMD is base architecture).
    pub(crate) unsafe fn gf_vec_mul_neon_inner(coef: u8, src: &[u8], dst: &mut [u8]) {
        use crate::reed_solomon::gf;
        use super::SIMD_CHUNK;

        debug_assert_eq!(src.len(), dst.len());
        let len = src.len();
        debug_assert_eq!(len % SIMD_CHUNK, 0);

        // coef == 0 → dst all zero.
        if coef == 0 {
            let mut off = 0;
            let zero = vmovq_n_u8(0);
            let dst_ptr = dst.as_mut_ptr();
            while off < len {
                vst1q_u8(dst_ptr.add(off), zero);
                vst1q_u8(dst_ptr.add(off).add(16), zero);
                off += SIMD_CHUNK;
            }
            return;
        }

        let tables = gf();
        let log_subs = build_subtables(&tables.log);
        let exp_subs = build_subtables(&tables.exp);
        let log_coef = tables.log[coef as usize]; // coef != 0
        let log_coef_vec = vmovq_n_u8(log_coef);
        let zero_vec = vmovq_n_u8(0);
        let one_vec = vmovq_n_u8(1);
        let ff_vec = vmovq_n_u8(0xFF);

        /// Process one 16-byte lane.
        #[inline]
        unsafe fn lane_mul(
            src_vec: uint8x16_t,
            log_subs: &SubLanes16,
            exp_subs: &SubLanes16,
            log_coef_vec: uint8x16_t,
            zero_vec: uint8x16_t,
            one_vec: uint8x16_t,
            ff_vec: uint8x16_t,
        ) -> uint8x16_t {
            // 1) log[src]
            let log_src = lut256_lane(log_subs, src_vec);
            // 2) zero mask
            let zero_mask = vceqq_u8(src_vec, zero_vec);
            // 3) wrapping sum
            let wrap_sum = vaddq_u8(log_src, log_coef_vec);
            // 4) saturating unsigned sum; where it saturates at 0xFF → true_sum>=255
            let sat_sum = vqaddq_u8(log_src, log_coef_vec);
            let sat255_mask = vceqq_u8(sat_sum, ff_vec);
            let adjust_one = vandq_u8(sat255_mask, one_vec);
            let exp_idx = vaddq_u8(wrap_sum, adjust_one);
            // 5) exp
            let exp_raw = lut256_lane(exp_subs, exp_idx);
            // 6) zero mask: clear bytes where src == 0
            vbicq_u8(exp_raw, zero_mask)
        }

        let src_ptr = src.as_ptr();
        let dst_ptr = dst.as_mut_ptr();
        let mut off = 0;
        while off < len {
            let lane0 = vld1q_u8(src_ptr.add(off));
            let lane1 = vld1q_u8(src_ptr.add(off).add(16));
            let r0 = lane_mul(
                lane0, &log_subs, &exp_subs, log_coef_vec, zero_vec, one_vec, ff_vec,
            );
            let r1 = lane_mul(
                lane1, &log_subs, &exp_subs, log_coef_vec, zero_vec, one_vec, ff_vec,
            );
            vst1q_u8(dst_ptr.add(off), r0);
            vst1q_u8(dst_ptr.add(off).add(16), r1);
            off += SIMD_CHUNK;
        }
    }
}

// ---------------------------------------------------------------------------
// Public (safe) entry points
// ---------------------------------------------------------------------------

/// Raw AVX2 kernel, exposed as `pub unsafe` for benchmarking / direct use.
///
/// On non-x86_64 this function does not exist (gated by
/// `#[cfg(target_arch = "x86_64")]`).  Prefer [`gf_vec_mul_auto`] for portable
/// code.
#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn gf_vec_mul_avx2(coef: u8, src: &[u8], dst: &mut [u8]) {
    x86_impl::gf_vec_mul_avx2_inner(coef, src, dst)
}

/// Raw NEON kernel, exposed as `pub unsafe` for benchmarking / direct use.
///
/// On non-aarch64 this function does not exist (gated by
/// `#[cfg(target_arch = "aarch64")]`).  Prefer [`gf_vec_mul_auto`] for portable
/// code.
#[cfg(target_arch = "aarch64")]
#[inline]
pub unsafe fn gf_vec_mul_neon(coef: u8, src: &[u8], dst: &mut [u8]) {
    neon_impl::gf_vec_mul_neon_inner(coef, src, dst)
}

/// Safe, mox_platform_orchestrator_svc-dispatching vector multiply.
///
/// Dispatch order (first match wins):
///   1. `feature = "simd"` AND AVX2 detected at mox_platform_orchestrator_svc (x86_64) → 32-byte
///      AVX2 chunks + scalar tail for the last 0..=31 bytes.
///   2. `feature = "simd"` AND NEON available (aarch64) → 32-byte NEON chunks
///      + scalar tail for the last 0..=31 bytes.
///   3. Otherwise → pure byte-by-byte scalar `gf_mul`.
pub fn gf_vec_mul_auto(coef: u8, src: &[u8], dst: &mut [u8]) {
    assert_eq!(
        src.len(),
        dst.len(),
        "gf_vec_mul_auto: src/dst length mismatch"
    );
    let len = src.len();
    if len == 0 {
        return;
    }

    #[cfg(feature = "simd")]
    #[cfg(target_arch = "x86_64")]
    {
        if is_avx2_supported() {
            let main_len = len & !(SIMD_CHUNK - 1);
            if main_len > 0 {
                unsafe {
                    x86_impl::gf_vec_mul_avx2_inner(
                        coef,
                        &src[..main_len],
                        &mut dst[..main_len],
                    );
                }
            }
            for i in main_len..len {
                dst[i] = crate::reed_solomon::gf_mul(coef, src[i]);
            }
            return;
        }
    }

    #[cfg(feature = "simd")]
    #[cfg(target_arch = "aarch64")]
    {
        if is_neon_supported() {
            let main_len = len & !(SIMD_CHUNK - 1);
            if main_len > 0 {
                unsafe {
                    neon_impl::gf_vec_mul_neon_inner(
                        coef,
                        &src[..main_len],
                        &mut dst[..main_len],
                    );
                }
            }
            for i in main_len..len {
                dst[i] = crate::reed_solomon::gf_mul(coef, src[i]);
            }
            return;
        }
    }

    // Pure scalar fallback (universal).
    for i in 0..len {
        dst[i] = crate::reed_solomon::gf_mul(coef, src[i]);
    }
}

/// Safe, mox_platform_orchestrator_svc-dispatching vector fused multiply-XOR:
/// `dst[i] ^= GF(2^8)::mul(coef, src[i])` for all `i`.
///
/// Uses the same dispatch tree as [`gf_vec_mul_auto`] but avoids a
/// user-visible temporary allocation (this is the hot path in RS
/// encode/decode parity construction).
pub fn gf_vec_mul_xor_auto(coef: u8, src: &[u8], dst: &mut [u8]) {
    assert_eq!(
        src.len(),
        dst.len(),
        "gf_vec_mul_xor_auto: src/dst length mismatch"
    );
    let len = src.len();
    if len == 0 {
        return;
    }
    if coef == 0 {
        // No-op: xor with zero.
        return;
    }
    if coef == 1 {
        // Xor-with-copy.
        for (d, &s) in dst.iter_mut().zip(src.iter()) {
            *d ^= s;
        }
        return;
    }

    #[cfg(feature = "simd")]
    #[cfg(target_arch = "x86_64")]
    {
        if is_avx2_supported() {
            let main_len = len & !(SIMD_CHUNK - 1);
            if main_len > 0 {
                unsafe {
                    // Wrap the hot loop in a #[target_feature] so AVX2
                    // intrinsics are inlined rather than turned into slow
                    // outlined calls.  Without this we observed a 4–8×
                    // regression vs the scalar path on release builds.
                    x86_impl::avx2_xor_fused_body(
                        coef,
                        src.as_ptr(),
                        dst.as_mut_ptr(),
                        main_len,
                    );
                }
            }
            // Scalar tail: xor fused.
            for i in main_len..len {
                dst[i] ^= crate::reed_solomon::gf_mul(coef, src[i]);
            }
            return;
        }
    }

    #[cfg(feature = "simd")]
    #[cfg(target_arch = "aarch64")]
    {
        if is_neon_supported() {
            let main_len = len & !(SIMD_CHUNK - 1);
            if main_len > 0 {
                unsafe {
                    use std::arch::aarch64::*;
                    use crate::reed_solomon::gf;
                    let tables = gf();
                    let log_subs = neon_impl::build_subtables(&tables.log);
                    let exp_subs = neon_impl::build_subtables(&tables.exp);
                    let log_coef = tables.log[coef as usize];
                    let log_coef_v = vmovq_n_u8(log_coef);

                    let src_ptr = src.as_ptr();
                    let dst_ptr = dst.as_mut_ptr();
                    let mut off = 0;
                    while off < main_len {
                        // 2× 128-bit NEON lanes = 32 bytes.
                        for lane in 0..2usize {
                            let base = off + lane * 16;
                            let sv = vld1q_u8(src_ptr.add(base));
                            let lsrc = neon_impl::lut256_lane(&log_subs, sv);
                            let zm = vceqq_u8(sv, vmovq_n_u8(0));
                            let lsumw = vaddq_u8(lsrc, log_coef_v);
                            // saturating add of u8 -> vqaddq_u8
                            let lsums = vqaddq_u8(lsrc, log_coef_v);
                            let am = vceqq_u8(lsums, vmovq_n_u8(255));
                            let aone = vandq_u8(am, vmovq_n_u8(1));
                            let eidx = vaddq_u8(lsumw, aone);
                            let eraw = neon_impl::lut256_lane(&exp_subs, eidx);
                            let prod = vbicq_u8(eraw, zm); // andnot(zm, eraw) = eraw & ~zm
                            let d = vld1q_u8(dst_ptr.add(base));
                            vst1q_u8(dst_ptr.add(base), veorq_u8(d, prod));
                        }
                        off += SIMD_CHUNK;
                    }
                }
            }
            for i in main_len..len {
                dst[i] ^= crate::reed_solomon::gf_mul(coef, src[i]);
            }
            return;
        }
    }

    // Pure scalar fused mul-xor fallback.
    let t = crate::reed_solomon::gf();
    let log_coef = t.log[coef as usize] as usize;
    for i in 0..len {
        let s = src[i];
        if s == 0 {
            continue;
        }
        let idx = log_coef + t.log[s as usize] as usize;
        dst[i] ^= t.exp[idx];
    }
}

// ---------------------------------------------------------------------------
// Feature-gated unit tests (T22-1 acceptance tests)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(feature = "simd")]
mod t22_tests {
    use super::*;
    use crate::reed_solomon::gf_mul;
    use rand::Rng;

    fn scalar_block_mul(coef: u8, src: &[u8]) -> Vec<u8> {
        src.iter().map(|&b| gf_mul(coef, b)).collect()
    }

    /// 1,000,000 random (coef, 32-byte block) pairs — SIMD must agree with scalar.
    #[test]
    fn t22_avx2_rand_1m() {
        if !is_avx2_supported() {
            eprintln!("SKIP t22_avx2_rand_1m: no AVX2 on this host");
            return;
        }
        let mut rng = rand::thread_rng();
        let mut dst = [0u8; 32];
        for iter in 0..1_000_000 {
            let coef: u8 = rng.gen();
            let block: [u8; 32] = rng.gen();
            let expected = scalar_block_mul(coef, &block);
            unsafe {
                // Safety: AVX2 just detected, len = 32 = SIMD_CHUNK.
                gf_vec_mul_avx2(coef, &block, &mut dst);
            }
            assert_eq!(
                dst[..], expected[..],
                "t22_avx2_rand_1m mismatch @ iter={} coef={:#04x}",
                iter, coef
            );
        }
    }

    /// Tail lengths 1..=63: SIMD + scalar tail must equal pure scalar, and the
    /// untouched bytes beyond `len` in the oversized dst (sentinel 0xA5) must
    /// not be modified.
    #[test]
    fn t22_avx2_tail_1_through_63() {
        if !is_avx2_supported() {
            eprintln!("SKIP t22_avx2_tail_1_through_63: no AVX2 on this host");
            return;
        }
        let mut rng = rand::thread_rng();
        const SENTINEL: u8 = 0xA5;
        for len in 1..=63usize {
            let coef: u8 = rng.gen();
            let mut src = vec![0u8; len];
            rng.fill(src.as_mut_slice());
            let mut dst = vec![SENTINEL; 128];
            let expected = scalar_block_mul(coef, &src);

            gf_vec_mul_auto(coef, &src, &mut dst[..len]);

            assert_eq!(&dst[..len], &expected[..], "tail mismatch len={}", len);
            for (i, &b) in dst[len..128].iter().enumerate() {
                assert_eq!(
                    b, SENTINEL,
                    "tail over-write: len={} pos={} got={:#04x}",
                    len,
                    len + i,
                    b
                );
            }
        }
    }

    /// coef == 0 -> all bytes 0; coef == 1 -> dst byte-identical to src.
    /// Both cross-checked against scalar.
    #[test]
    fn t22_avx2_coef_0_and_1() {
        if !is_avx2_supported() {
            eprintln!("SKIP t22_avx2_coef_0_and_1: no AVX2 on this host");
            return;
        }
        let mut rng = rand::thread_rng();
        for &size in &[32usize, 64, 127, 256, 1024] {
            let mut src = vec![0u8; size];
            rng.fill(src.as_mut_slice());
            let mut dst0 = vec![0xFFu8; size];
            let mut dst1 = vec![0u8; size];

            gf_vec_mul_auto(0, &src, &mut dst0);
            gf_vec_mul_auto(1, &src, &mut dst1);

            assert!(
                dst0.iter().all(|&b| b == 0),
                "coef=0 all-zero fail @ size={}",
                size
            );
            assert_eq!(dst1, src, "coef=1 identity-mul fail @ size={}", size);

            let exp0 = scalar_block_mul(0, &src);
            let exp1 = scalar_block_mul(1, &src);
            assert_eq!(dst0, exp0, "coef=0 vs scalar @ size={}", size);
            assert_eq!(dst1, exp1, "coef=1 vs scalar @ size={}", size);
        }
    }
}

// ---------------------------------------------------------------------------
// T22-2 NEON acceptance tests (cross-arch; aarch64 tests skipped on x86_64)
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(feature = "simd")]
mod t22_neon_tests {
    use super::*;
    use crate::reed_solomon::gf_mul;
    use rand::Rng;

    fn scalar_block_mul(coef: u8, src: &[u8]) -> Vec<u8> {
        src.iter().map(|&b| gf_mul(coef, b)).collect()
    }

    /// Cross-platform compile probe: verifies `is_neon_supported()` reports
    /// the right value given the target arch.  Runs on every platform so CI
    /// always exercises the public API surface of this module.
    #[test]
    fn t22_neon_compile_probe() {
        #[cfg(target_arch = "aarch64")]
        assert!(is_neon_supported(), "is_neon_supported must be true on aarch64");
        #[cfg(not(target_arch = "aarch64"))]
        assert!(
            !is_neon_supported(),
            "is_neon_supported must be false off aarch64"
        );
    }

    /// 100,000 random (coef, 32B block) pairs — NEON agrees with scalar.
    /// Skipped when not on aarch64 (the raw kernel is cfg-gated).
    #[test]
    fn t22_neon_rand_100k() {
        if !is_neon_supported() {
            eprintln!("SKIP t22_neon_rand_100k: not aarch64 host");
            return;
        }
        #[cfg(target_arch = "aarch64")]
        {
            let mut rng = rand::thread_rng();
            let mut dst = [0u8; 32];
            for iter in 0..100_000 {
                let coef: u8 = rng.gen();
                let block: [u8; 32] = rng.gen();
                let expected = scalar_block_mul(coef, &block);
                unsafe {
                    gf_vec_mul_neon(coef, &block, &mut dst);
                }
                assert_eq!(
                    dst[..], expected[..],
                    "t22_neon_rand_100k mismatch @ iter={} coef={:#04x}",
                    iter, coef
                );
            }
        }
    }

    /// Tail lengths 1..=63 via `gf_vec_mul_auto` (NEON main + scalar tail on
    /// aarch64); sentinel bytes 0xA5 outside the slice must remain untouched.
    #[test]
    fn t22_neon_tail_1_63() {
        if !is_neon_supported() {
            eprintln!("SKIP t22_neon_tail_1_63: not aarch64 host");
            return;
        }
        let mut rng = rand::thread_rng();
        const SENTINEL: u8 = 0xA5;
        for len in 1..=63usize {
            let coef: u8 = rng.gen();
            let mut src = vec![0u8; len];
            rng.fill(src.as_mut_slice());
            let mut dst = vec![SENTINEL; 128];
            let expected = scalar_block_mul(coef, &src);

            gf_vec_mul_auto(coef, &src, &mut dst[..len]);

            assert_eq!(&dst[..len], &expected[..], "tail mismatch len={}", len);
            for (i, &b) in dst[len..128].iter().enumerate() {
                assert_eq!(
                    b, SENTINEL,
                    "tail over-write: len={} pos={} got={:#04x}",
                    len,
                    len + i,
                    b
                );
            }
        }
    }

    /// coef == 0 -> all bytes 0; coef == 1 -> dst byte-identical to src.
    #[test]
    fn t22_neon_coef_0_1() {
        if !is_neon_supported() {
            eprintln!("SKIP t22_neon_coef_0_1: not aarch64 host");
            return;
        }
        let mut rng = rand::thread_rng();
        for &size in &[32usize, 64, 127, 256, 1024] {
            let mut src = vec![0u8; size];
            rng.fill(src.as_mut_slice());
            let mut dst0 = vec![0xFFu8; size];
            let mut dst1 = vec![0u8; size];

            gf_vec_mul_auto(0, &src, &mut dst0);
            gf_vec_mul_auto(1, &src, &mut dst1);

            assert!(
                dst0.iter().all(|&b| b == 0),
                "coef=0 all-zero fail @ size={}",
                size
            );
            assert_eq!(dst1, src, "coef=1 identity-mul fail @ size={}", size);

            let exp0 = scalar_block_mul(0, &src);
            let exp1 = scalar_block_mul(1, &src);
            assert_eq!(dst0, exp0, "coef=0 vs scalar @ size={}", size);
            assert_eq!(dst1, exp1, "coef=1 vs scalar @ size={}", size);
        }
    }
}
