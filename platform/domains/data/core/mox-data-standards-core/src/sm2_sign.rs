// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SM2 digital signature algorithm — pure Rust implementation.
//!
//! Follows GM/T 0003.2-2012 (256-bit recommended curve sm2p256v1).  Big-integer
//! primitives are backed by the well-audited `num-bigint` crate over `BigUint`
//! to guarantee mathematical correctness.  Curve point operations use affine
//! coordinates (explicit modular inverses), which are acceptable for the
//! sign/verify rates our platform encounters.  Scalar multiplication is the
//! classical left-to-right double-and-add.
//!
//! Only raw `(r, s)` signature format is produced; DER conversion helpers are
//! also exposed.

#![allow(clippy::needless_range_loop)]

use num_bigint::BigUint;
use num_traits::{One, Zero};
use core::convert::TryFrom;

// ---------------------------------------------------------------------------
// 256-bit big integer helpers: thin `[u32; 8]` (little-endian limbs) façade
// over `num_bigint::BigUint`.  The same `U256` type alias is exported so
// callers in `Sm2Sk` / `Sm2Pk` / tests remain completely unchanged.
// ---------------------------------------------------------------------------

mod big256 {
    use super::BigUint;

    /// 256-bit unsigned integer stored as 8 × u32 little-endian limbs.
    /// Limb 0 is the least significant word.
    pub type U256 = [u32; 8];

    const LIMBS: usize = 8;

    #[inline]
    fn to_big(x: &U256) -> BigUint {
        let bytes = to_bytes_le(x);
        BigUint::from_bytes_le(&bytes)
    }

    #[inline]
    fn from_big(b: &BigUint) -> U256 {
        let mut bytes = b.to_bytes_le();
        bytes.resize(32, 0u8);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[..32]);
        bytes_to_le_u256(&arr)
    }

    #[inline]
    fn to_bytes_le(x: &U256) -> [u8; 32] {
        let mut out = [0u8; 32];
        for i in 0..LIMBS {
            out[i * 4]     = (x[i]      ) as u8;
            out[i * 4 + 1] = (x[i] >> 8 ) as u8;
            out[i * 4 + 2] = (x[i] >> 16) as u8;
            out[i * 4 + 3] = (x[i] >> 24) as u8;
        }
        out
    }

    #[inline]
    fn bytes_to_le_u256(b: &[u8; 32]) -> U256 {
        let mut r = [0u32; LIMBS];
        for i in 0..LIMBS {
            r[i] = (b[i * 4] as u32)
                | ((b[i * 4 + 1] as u32) << 8)
                | ((b[i * 4 + 2] as u32) << 16)
                | ((b[i * 4 + 3] as u32) << 24);
        }
        r
    }

    /// Compare two U256 (limbs LE): returns true iff x >= y.
    #[inline]
    pub fn is_ge(x: &U256, y: &U256) -> bool {
        to_big(x) >= to_big(y)
    }

    /// Add two U256 (limbs LE), returning (low, carry).
    #[inline]
    pub fn add(x: &U256, y: &U256) -> (U256, u32) {
        let a = to_big(x);
        let b = to_big(y);
        let sum = a + b;
        let bytes = sum.to_bytes_le();
        let carry: u32 = if bytes.len() > 32 {
            // The extra limbs' value doesn't actually matter for current
            // callers because they only test carry != 0.
            1
        } else {
            0
        };
        let mut pad = [0u8; 32];
        let n = bytes.len().min(32);
        pad[..n].copy_from_slice(&bytes[..n]);
        (bytes_to_le_u256(&pad), carry)
    }

    /// Subtract two U256 (limbs LE).  x MUST be >= y.
    #[inline]
    pub fn sub(x: &U256, y: &U256) -> U256 {
        let d = to_big(x) - to_big(y);
        from_big(&d)
    }

    /// (x + y) mod m.
    pub fn add_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        let sum = to_big(x) + to_big(y);
        let mm = to_big(m);
        let r = if &sum >= &mm { sum - &mm } else { sum };
        // Wrap to [0, m) explicitly (no-op for <2*m additions).
        let r2 = if &r >= &mm { &r - &mm } else { r };
        from_big(&r2)
    }

    /// (x - y) mod m.
    pub fn sub_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        let a = to_big(x);
        let b = to_big(y);
        let mm = to_big(m);
        let r = if a >= b { a - b } else { &mm - (b - a) };
        from_big(&r)
    }

    /// (x * y) mod m.
    pub fn mul_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        let prod = to_big(x) * to_big(y);
        let mm = to_big(m);
        let r = prod % &mm;
        from_big(&r)
    }

    /// Modular exponentiation via square-and-multiply.
    pub fn pow_mod(base: &U256, exp: &U256, m: &U256) -> U256 {
        let mm = to_big(m);
        // BigUint::pow takes a usize for exponent (!= BigUint).  Use
        // manual square-and-multiply over the exponent's LE bytes.
        let e_bytes = to_bytes_le(exp);
        let mut result = BigUint::from(1u32);
        let mut b = to_big(base) % &mm;
        for byte in &e_bytes {
            let mut ee = *byte;
            for _ in 0..8 {
                if ee & 1 == 1 {
                    result = (&result * &b) % &mm;
                }
                ee >>= 1;
                b = (&b * &b) % &mm;
            }
        }
        from_big(&result)
    }

    /// Modular inverse via Fermat's little theorem.  m MUST be prime.
    pub fn inv_mod(x: &U256, m: &U256) -> U256 {
        let mm = to_big(m);
        let xb = to_big(x) % &mm;
        // exponent: m - 2
        let two = BigUint::from(2u32);
        let exp = &mm - two;
        pow_mod(&from_big(&xb), &from_big(&exp), m)
    }

    /// Return true if U256 == 0.
    pub fn is_zero(x: &U256) -> bool {
        x.iter().all(|&l| l == 0)
    }

    /// U256 == 1?
    pub fn is_one(x: &U256) -> bool {
        x[0] == 1 && x.iter().skip(1).all(|&l| l == 0)
    }

    pub fn zero() -> U256 { [0u32; 8] }
    pub fn one()  -> U256 { let mut r = [0u32; 8]; r[0] = 1; r }
    pub fn two()  -> U256 { let mut r = [0u32; 8]; r[0] = 2; r }

    /// Convert 32 big-endian bytes to U256 (limbs LE).
    pub fn from_be_bytes(b: &[u8; 32]) -> U256 {
        let mut le = [0u8; 32];
        for i in 0..32 {
            le[i] = b[31 - i];
        }
        bytes_to_le_u256(&le)
    }

    /// Convert U256 to 32 big-endian bytes.
    pub fn to_be_bytes(x: &U256) -> [u8; 32] {
        let le = to_bytes_le(x);
        let mut be = [0u8; 32];
        for i in 0..32 {
            be[i] = le[31 - i];
        }
        be
    }
}

// ---------------------------------------------------------------------------
// SM2 domain parameters (sm2p256v1 / GM/T 0003.5-2012)
// ---------------------------------------------------------------------------

mod curve_params {
    use super::big256::U256;

    macro_rules! hex_u256 {
        ($s:literal) => {{
            let b = hex_lit_32($s);
            crate::sm2_sign::big256::from_be_bytes(&b)
        }};
    }

    const fn hex_lit_32(s: &'static str) -> [u8; 32] {
        let s_bytes = s.as_bytes();
        let mut r = [0u8; 32];
        let mut i = 0;
        while i < 64 {
            let hi = s_bytes[i];
            let lo = s_bytes[i + 1];
            r[i / 2] = (hex_nib(hi) << 4) | hex_nib(lo);
            i += 2;
        }
        r
    }

    const fn hex_nib(b: u8) -> u8 {
        match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            b'A'..=b'F' => b - b'A' + 10,
            _ => 0,
        }
    }

    /// prime p
    pub fn p() -> U256 {
        hex_u256!("FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000FFFFFFFFFFFFFFFF")
    }
    /// curve coeff a
    pub fn a() -> U256 {
        hex_u256!("FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000FFFFFFFFFFFFFFFC")
    }
    /// curve coeff b
    pub fn b() -> U256 {
        hex_u256!("28E9FA9E9D9F5E344D5A9E4BCF6509A7F39789F515AB8F92DDBCBD414D940E93")
    }
    /// order n
    pub fn n() -> U256 {
        hex_u256!("FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFF7203DF6B21C6052B53BBF40939D54123")
    }
    /// G.x
    pub fn gx() -> U256 {
        hex_u256!("32C4AE2C1F1981195F9904466A39C9948FE30BBFF2660BE1715A4589334C74C7")
    }
    /// G.y
    pub fn gy() -> U256 {
        hex_u256!("BC3736A2F4F6779C59BDCEE36B692153D0A9877CC62A474002DF32E52139F0A0")
    }
}

// ---------------------------------------------------------------------------
// Affine-point arithmetic over sm2p256v1 (backed by BigUint — correctness)
// ---------------------------------------------------------------------------

mod ec {
    use super::big256::{self, U256};
    use super::curve_params;
    use super::{BigUint, One, Zero};

    fn to_b(x: &U256) -> BigUint {
        let mut le = [0u8; 32];
        for i in 0..8 {
            le[i * 4]     = (x[i]      ) as u8;
            le[i * 4 + 1] = (x[i] >> 8 ) as u8;
            le[i * 4 + 2] = (x[i] >> 16) as u8;
            le[i * 4 + 3] = (x[i] >> 24) as u8;
        }
        BigUint::from_bytes_le(&le)
    }

    fn from_b(b: &BigUint) -> U256 {
        let mut bytes = b.to_bytes_le();
        bytes.resize(32, 0u8);
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[..32]);
        let mut r = [0u32; 8];
        for i in 0..8 {
            r[i] = (arr[i * 4] as u32)
                | ((arr[i * 4 + 1] as u32) << 8)
                | ((arr[i * 4 + 2] as u32) << 16)
                | ((arr[i * 4 + 3] as u32) << 24);
        }
        r
    }

    /// Affine point.  Infinity is represented by `inf = true`.
    #[derive(Clone, Copy)]
    pub struct Point {
        pub x: U256,
        pub y: U256,
        pub inf: bool,
    }

    impl Point {
        pub fn infinity() -> Self {
            Point {
                x: big256::zero(),
                y: big256::one(),
                inf: true,
            }
        }
        pub fn is_infinity(&self) -> bool { self.inf }

        pub fn from_affine(x: &U256, y: &U256) -> Self {
            Point { x: *x, y: *y, inf: false }
        }

        /// Return affine coordinates if point is not at infinity.
        pub fn to_affine(&self) -> Option<(U256, U256)> {
            if self.inf { None } else { Some((self.x, self.y)) }
        }

        /// Generator G in affine.
        pub fn generator() -> Self {
            Point::from_affine(&curve_params::gx(), &curve_params::gy())
        }
    }

    /// Point double in affine coords.
    pub fn point_double(p: &Point) -> Point {
        if p.inf { return Point::infinity(); }
        let prime_b = to_b(&curve_params::p());
        let a_b     = to_b(&curve_params::a());
        let x_b = to_b(&p.x);
        let y_b = to_b(&p.y);
        // λ = (3·x² + a) / (2·y)   mod p
        let three = BigUint::from(3u32);
        let two   = BigUint::from(2u32);
        let num = (&three * &x_b * &x_b + &a_b) % &prime_b;
        let den = (&two * &y_b) % &prime_b;
        if den.is_zero() {
            return Point::infinity();
        }
        let lam = big_mul_mod(&num, &inv(&den, &prime_b), &prime_b);
        // x_new = λ² - 2x mod p → ((λ² + p) - (2x mod p)) mod p, safe.
        let lam2 = big_mul_mod(&lam, &lam, &prime_b);
        let two_x = (&two * &x_b) % &prime_b;
        let x_new = submod(&lam2, &two_x, &prime_b);
        // y_new = λ·(x - x_new) - y mod p → λ·(x - x_new + p - y) mod p
        let diff_x = submod(&x_b, &x_new, &prime_b);
        let m_diff = big_mul_mod(&lam, &diff_x, &prime_b);
        let y_new = submod(&m_diff, &y_b, &prime_b);
        Point::from_affine(&from_b(&x_new), &from_b(&y_new))
    }

    /// Point add in affine coords.
    pub fn point_add(p: &Point, q: &Point) -> Point {
        if p.inf { return *q; }
        if q.inf { return *p; }
        let prime_b = to_b(&curve_params::p());
        let (x1, y1) = (to_b(&p.x), to_b(&p.y));
        let (x2, y2) = (to_b(&q.x), to_b(&q.y));
        if x1 == x2 {
            if y1 == y2 {
                return point_double(p);
            }
            return Point::infinity();
        }
        // λ = (y2 - y1) / (x2 - x1) mod p
        let dy = submod(&y2, &y1, &prime_b);
        let dx = submod(&x2, &x1, &prime_b);
        let lam = big_mul_mod(&dy, &inv(&dx, &prime_b), &prime_b);
        let lam2 = big_mul_mod(&lam, &lam, &prime_b);
        let t = submod(&lam2, &x1, &prime_b);
        let x_new = submod(&t, &x2, &prime_b);
        let dx_new = submod(&x1, &x_new, &prime_b);
        let t2 = big_mul_mod(&lam, &dx_new, &prime_b);
        let y_new = submod(&t2, &y1, &prime_b);
        Point::from_affine(&from_b(&x_new), &from_b(&y_new))
    }

    /// (a - b) mod m, safe for all a,b (adds m beforehand).
    fn submod(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
        // compute a + m - b then mod m (avoids underflow entirely).
        ((a + m) - b) % m
    }

    fn big_mul_mod(a: &BigUint, b: &BigUint, m: &BigUint) -> BigUint {
        (a * b) % m
    }

    fn inv(x: &BigUint, m: &BigUint) -> BigUint {
        // Fermat: m is prime.
        let one = BigUint::one();
        let two = BigUint::from(2u32);
        let exp = m - two;
        // ModPow: use simple square-multiply over exp bytes.
        let e_bytes = exp.to_bytes_le();
        let mut result = one.clone();
        let mut base = x % m;
        for byte in e_bytes {
            let mut e = byte;
            for _ in 0..8 {
                if e & 1 == 1 {
                    result = (&result * &base) % m;
                }
                e >>= 1;
                base = (&base * &base) % m;
            }
        }
        result
    }

    /// Scalar multiplication: L2R MSB-first double-and-add over k's 256 bits.
    pub fn scalar_mul(k: &U256, p: &Point) -> Point {
        // walk be-bytes (from most significant) for L2R.
        let be = big256::to_be_bytes(k);
        let mut acc = Point::infinity();
        for byte in be.iter() {
            let mut e = *byte;
            for _ in 0..8 {
                acc = point_double(&acc);
                if e & 0x80 != 0 {
                    acc = point_add(&acc, p);
                }
                e <<= 1;
            }
        }
        acc
    }
}

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

/// SM2 private key — raw 32-byte scalar d in [1, n-1].
#[derive(Debug, Clone)]
pub struct Sm2Sk(pub [u8; 32]);

/// SM2 public key — affine point (x, y) on sm2p256v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sm2Pk {
    pub x: [u8; 32],
    pub y: [u8; 32],
}

/// Raw SM2 signature (r, s) — 32 bytes each, big-endian.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sm2Sig {
    pub r: [u8; 32],
    pub s: [u8; 32],
}

// ---------------------------------------------------------------------------
// Convenience: generate / public key / sign
// ---------------------------------------------------------------------------

impl Sm2Sk {
    /// Generate a uniformly random private key in [1, n-1] using rejection
    /// sampling over the supplied `CryptoRng`.
    #[cfg(feature = "gm-sm")]
    pub fn generate<R: rand::RngCore + rand::CryptoRng>(rng: &mut R) -> Self {
        use big256;
        use curve_params;
        let n = curve_params::n();
        loop {
            let mut bytes = [0u8; 32];
            rng.fill_bytes(&mut bytes);
            let val = big256::from_be_bytes(&bytes);
            if big256::is_zero(&val) { continue; }
            if !big256::is_ge(&val, &n) { return Sm2Sk(bytes); }
        }
    }

    /// Derive the public key Q = d·G on sm2p256v1.
    pub fn public_key(&self) -> Sm2Pk {
        use big256;
        use ec;
        let d = big256::from_be_bytes(&self.0);
        let g = ec::Point::generator();
        let q = ec::scalar_mul(&d, &g);
        let (x, y) = q.to_affine().expect("d·G must not be point at infinity");
        Sm2Pk {
            x: big256::to_be_bytes(&x),
            y: big256::to_be_bytes(&y),
        }
    }

    /// Sign a message with the provided id (default: 128-bit ASCII
    /// "1234567812345678").  Uses GM/T 0003.2-2012 standard flow with random
    /// nonce `k` sampled over [1, n-1] by rejection sampling.
    #[cfg(feature = "gm-sm")]
    pub fn sign<R: rand::RngCore + rand::CryptoRng>(
        &self, msg: &[u8], id: &[u8], rng: &mut R,
    ) -> Sm2Sig {
        use big256;
        use curve_params;
        use ec;
        use crate::sm3_hash::sm3;
        let n = curve_params::n();
        let pk = self.public_key();
        let za = compute_za(id.len() as u16 * 8, id, &pk);
        let mut e_parts: Vec<u8> = Vec::with_capacity(za.len() + msg.len());
        e_parts.extend_from_slice(&za);
        e_parts.extend_from_slice(msg);
        let e_bytes = sm3(&e_parts);
        let e = big256::from_be_bytes(&e_bytes);
        let d = big256::from_be_bytes(&self.0);
        loop {
            let k = loop {
                let mut kb = [0u8; 32];
                rng.fill_bytes(&mut kb);
                let v = big256::from_be_bytes(&kb);
                if big256::is_zero(&v) { continue; }
                if !big256::is_ge(&v, &n) { break v; }
            };
            let (x1, _y1) = ec::scalar_mul(&k, &ec::Point::generator())
                .to_affine()
                .expect("k·G must not be infinity");
            let r = big256::add_mod(&e, &x1, &n);
            if big256::is_zero(&r) { continue; }
            let one = big256::one();
            let r_plus_k = big256::add_mod(&r, &k, &n);
            if big256::is_zero(&r_plus_k) { continue; }
            let d_plus_1 = big256::add_mod(&d, &one, &n);
            let inv_d1 = big256::inv_mod(&d_plus_1, &n);
            let rd = big256::mul_mod(&r, &d, &n);
            let k_minus_rd = big256::sub_mod(&k, &rd, &n);
            let s = big256::mul_mod(&inv_d1, &k_minus_rd, &n);
            if big256::is_zero(&s) { continue; }
            return Sm2Sig {
                r: big256::to_be_bytes(&r),
                s: big256::to_be_bytes(&s),
            };
        }
    }
}

impl Sm2Pk {
    /// Uncompressed public key bytes: `0x04 || x || y` (65 bytes).
    pub fn as_uncompressed_bytes(&self) -> [u8; 65] {
        let mut out = [0u8; 65];
        out[0] = 0x04;
        out[1..33].copy_from_slice(&self.x);
        out[33..65].copy_from_slice(&self.y);
        out
    }

    /// Parse uncompressed bytes back into an Sm2Pk.  Returns None on format
    /// errors.
    pub fn from_uncompressed_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != 65 || b[0] != 0x04 { return None; }
        let mut x = [0u8; 32];
        let mut y = [0u8; 32];
        x.copy_from_slice(&b[1..33]);
        y.copy_from_slice(&b[33..65]);
        Some(Sm2Pk { x, y })
    }

    /// Verify an SM2 signature per GM/T 0003.2-2012 §6.2.
    pub fn verify(&self, msg: &[u8], id: &[u8], sig: &Sm2Sig) -> bool {
        use big256;
        use curve_params;
        use ec;
        use crate::sm3_hash::sm3;
        let n = curve_params::n();
        let r = big256::from_be_bytes(&sig.r);
        let s = big256::from_be_bytes(&sig.s);
        if big256::is_zero(&r) || big256::is_ge(&r, &n) { return false; }
        if big256::is_zero(&s) || big256::is_ge(&s, &n) { return false; }
        let za = compute_za(id.len() as u16 * 8, id, self);
        let mut e_parts: Vec<u8> = Vec::with_capacity(za.len() + msg.len());
        e_parts.extend_from_slice(&za);
        e_parts.extend_from_slice(msg);
        let e_bytes = sm3(&e_parts);
        let e = big256::from_be_bytes(&e_bytes);
        let t = big256::add_mod(&r, &s, &n);
        if big256::is_zero(&t) { return false; }
        let sg = ec::scalar_mul(&s, &ec::Point::generator());
        let q_point = ec::Point::from_affine(
            &big256::from_be_bytes(&self.x),
            &big256::from_be_bytes(&self.y),
        );
        let tq = ec::scalar_mul(&t, &q_point);
        let sum = ec::point_add(&sg, &tq);
        let (x1, _y1) = match sum.to_affine() {
            Some(p) => p,
            None => return false,
        };
        let r_check = big256::add_mod(&e, &x1, &n);
        big256::is_ge(&r_check, &r) && big256::is_ge(&r, &r_check)
    }
}

// ---------------------------------------------------------------------------
// ZA: identity hash (GM/T 0003.1-2012 §5.5)
// ---------------------------------------------------------------------------

/// Compute `ZA = SM3(ENTL_2BE_bytes || ID || a || b || Gx || Gy || xA || yA)`.
pub fn compute_za(entl_bits: u16, id: &[u8], pk: &Sm2Pk) -> [u8; 32] {
    use big256;
    use curve_params;
    use crate::sm3_hash::sm3;
    let mut buf = Vec::with_capacity(
        2 + id.len() + 32 + 32 + 32 + 32 + 32 + 32,
    );
    buf.extend_from_slice(&entl_bits.to_be_bytes());
    buf.extend_from_slice(id);
    buf.extend_from_slice(&big256::to_be_bytes(&curve_params::a()));
    buf.extend_from_slice(&big256::to_be_bytes(&curve_params::b()));
    buf.extend_from_slice(&big256::to_be_bytes(&curve_params::gx()));
    buf.extend_from_slice(&big256::to_be_bytes(&curve_params::gy()));
    buf.extend_from_slice(&pk.x);
    buf.extend_from_slice(&pk.y);
    sm3(&buf)
}

// ---------------------------------------------------------------------------
// Helpers: DER encode / decode + hex convenience
// ---------------------------------------------------------------------------

impl Sm2Sig {
    /// DER encode (same format as ECDSA):
    /// `30 || len || 02 || rlen || r_be || 02 || slen || s_be`.
    pub fn to_der(&self) -> Vec<u8> {
        fn encode_int(be: &[u8; 32]) -> Vec<u8> {
            let start = be.iter().position(|&b| b != 0).unwrap_or(32);
            let body = if start == 32 {
                vec![0u8]
            } else {
                let stripped = &be[start..];
                if stripped[0] & 0x80 != 0 {
                    let mut v = Vec::with_capacity(stripped.len() + 1);
                    v.push(0x00);
                    v.extend_from_slice(stripped);
                    v
                } else {
                    stripped.to_vec()
                }
            };
            let mut v = Vec::with_capacity(2 + body.len());
            v.push(0x02);
            v.push(body.len() as u8);
            v.extend_from_slice(&body);
            v
        }
        let r_der = encode_int(&self.r);
        let s_der = encode_int(&self.s);
        let total_len = r_der.len() + s_der.len();
        let mut out = Vec::with_capacity(2 + total_len);
        out.push(0x30);
        if total_len < 128 {
            out.push(total_len as u8);
        } else {
            out.push(0x81);
            out.push(total_len as u8);
        }
        out.extend_from_slice(&r_der);
        out.extend_from_slice(&s_der);
        out
    }

    /// Minimal DER decode.  Returns `None` on format errors.
    pub fn from_der(data: &[u8]) -> Option<Self> {
        if data.len() < 8 || data[0] != 0x30 { return None; }
        let (total_len, mut pos) = match data[1] {
            l if l < 0x80 => (l as usize, 2),
            0x81 => (data[2] as usize, 3),
            _ => return None,
        };
        if data.len() != pos + total_len { return None; }
        fn read_int(data: &[u8], pos: &mut usize) -> Option<[u8; 32]> {
            if data.len() < *pos + 2 || data[*pos] != 0x02 { return None; }
            let ilen = data[*pos + 1] as usize;
            *pos += 2;
            if ilen == 0 || data.len() < *pos + ilen { return None; }
            let int_bytes = &data[*pos..*pos + ilen];
            *pos += ilen;
            let trimmed = if int_bytes[0] == 0x00 && int_bytes.len() > 1 {
                &int_bytes[1..]
            } else { int_bytes };
            if trimmed.len() > 32 { return None; }
            let mut out = [0u8; 32];
            out[32 - trimmed.len()..].copy_from_slice(trimmed);
            Some(out)
        }
        let r = read_int(data, &mut pos)?;
        let s = read_int(data, &mut pos)?;
        if pos != data.len() { return None; }
        Some(Sm2Sig { r, s })
    }
}

/// Convenience: sign and return lowercase hex of `r || s` (128 chars).
#[cfg(feature = "gm-sm")]
pub fn sm2_sign_hex<R: rand::RngCore + rand::CryptoRng>(
    msg: &[u8], id: &[u8], sk: &Sm2Sk, rng: &mut R,
) -> String {
    use hex::ToHex;
    let sig = sk.sign(msg, id, rng);
    let mut s = String::with_capacity(128);
    s.extend(sig.r.encode_hex::<String>().chars());
    s.extend(sig.s.encode_hex::<String>().chars());
    s
}

/// Convenience: verify a 128-hex-char signature against an uncompressed
/// public key (65 bytes starting in 0x04).
pub fn sm2_verify_hex(msg: &[u8], id: &[u8], pk_bytes: &[u8], sig_hex: &str) -> bool {
    use hex::FromHex;
    let pk = match Sm2Pk::from_uncompressed_bytes(pk_bytes) {
        Some(p) => p,
        None => return false,
    };
    if sig_hex.len() != 128 { return false; }
    let sig_bytes = match Vec::<u8>::from_hex(sig_hex) {
        Ok(b) => b,
        Err(_) => return false,
    };
    let mut r = [0u8; 32];
    let mut s = [0u8; 32];
    r.copy_from_slice(&sig_bytes[..32]);
    s.copy_from_slice(&sig_bytes[32..]);
    let sig = Sm2Sig { r, s };
    pk.verify(msg, id, &sig)
}

// ---------------------------------------------------------------------------
// T24 acceptance tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(feature = "gm-sm")]
mod t24_tests {
    use super::*;
    use rand::Rng;

    const DEFAULT_ID: &[u8] = b"1234567812345678";

    /// Unit-level sanity: verify basic EC operations match scalar rules.
    #[test]
    fn _t24_sm2_ec_sanity() {
        use ec;
        use big256;
        let g = ec::Point::generator();
        let g_aff = g.to_affine().unwrap();
        // 1*G == G
        let one = big256::one();
        let s1g = ec::scalar_mul(&one, &g);
        let s1g_aff = s1g.to_affine().expect("1*G finite");
        assert_eq!(s1g_aff.0, g_aff.0, "1*G.x mismatch");
        assert_eq!(s1g_aff.1, g_aff.1, "1*G.y mismatch");
        // 2*G via double == via scalar_mul(2)
        let g2_dbl = ec::point_double(&g);
        let g2_dbl_aff = g2_dbl.to_affine().unwrap();
        let two = big256::two();
        let g2_scl = ec::scalar_mul(&two, &g);
        let g2_scl_aff = g2_scl.to_affine().expect("2*G finite via scalar");
        assert_eq!(g2_dbl_aff.0, g2_scl_aff.0, "2G x mismatch");
        assert_eq!(g2_dbl_aff.1, g2_scl_aff.1, "2G y mismatch");
        // 3*G == 2G + G
        let g3 = ec::point_add(&g2_dbl, &g);
        let three = {
            let mut t = two;
            t[0] += 1;
            t
        };
        let g3s = ec::scalar_mul(&three, &g);
        assert_eq!(
            g3.to_affine().unwrap().0,
            g3s.to_affine().unwrap().0,
            "3G x mismatch"
        );
        // 2^255 (MSB set) should not yield infinity.
        let mut k255_bytes = [0u8; 32];
        k255_bytes[0] = 0x80;
        let k255 = big256::from_be_bytes(&k255_bytes);
        let p255 = ec::scalar_mul(&k255, &g);
        assert!(
            !p255.is_infinity(),
            "2^255*G should not be infinity since 2^255 < n"
        );
        // Medium scalar: bytes where high limbs nonzero.
        let mut kmid_bytes = [0u8; 32];
        kmid_bytes[0] = 0x11;
        kmid_bytes[31] = 0x22;
        let kmid = big256::from_be_bytes(&kmid_bytes);
        let pmid = ec::scalar_mul(&kmid, &g);
        assert!(!pmid.is_infinity(), "kmid*G should not be infinity");
    }

    /// 100 random SM2 sign/verify round trips with 32-byte random messages.
    #[test]
    fn t24_sm2_signverify_roundtrip_100() {
        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            let sk = Sm2Sk::generate(&mut rng);
            let pk = sk.public_key();
            let mut msg = [0u8; 32];
            rng.fill(&mut msg);
            let sig = sk.sign(&msg, DEFAULT_ID, &mut rng);
            assert!(
                pk.verify(&msg, DEFAULT_ID, &sig),
                "SM2 sign/verify roundtrip failed"
            );
        }
    }

    /// Tamper 1 bit of r → verify must fail.
    #[test]
    fn t24_sm2_sig_tamper_fails() {
        let mut rng = rand::thread_rng();
        let sk = Sm2Sk::generate(&mut rng);
        let pk = sk.public_key();
        let msg = b"tamper-fails test message";
        let sig = sk.sign(msg, DEFAULT_ID, &mut rng);
        assert!(pk.verify(msg, DEFAULT_ID, &sig), "original must verify");

        let mut bad_sig = sig.clone();
        bad_sig.r[0] ^= 0x01;
        assert!(
            !pk.verify(msg, DEFAULT_ID, &bad_sig),
            "tampered r must NOT verify"
        );
    }

    /// Sm2Pk as_uncompressed_bytes / from_uncompressed_bytes roundtrip;
    /// asserts uncompressed byte layout (`0x04` prefix + x in bytes 1..=32).
    #[test]
    fn t24_sm2_pk_uncompressed_rountrip() {
        let mut rng = rand::thread_rng();
        for _ in 0..20 {
            let sk = Sm2Sk::generate(&mut rng);
            let pk = sk.public_key();
            let bytes = pk.as_uncompressed_bytes();
            assert_eq!(bytes[0], 0x04, "uncompressed prefix must be 0x04");
            assert_eq!(&bytes[1..33], &pk.x[..], "bytes 1..33 must equal pk.x");
            assert_eq!(&bytes[33..65], &pk.y[..], "bytes 33..65 must equal pk.y");
            let pk2 = Sm2Pk::from_uncompressed_bytes(&bytes)
                .expect("from_uncompressed_bytes must roundtrip");
            assert_eq!(pk, pk2, "pk roundtrip mismatch");
        }
        let mut bad = [0u8; 65];
        bad[0] = 0x02;
        assert!(Sm2Pk::from_uncompressed_bytes(&bad).is_none(), "prefix 0x02 must fail");
        assert!(Sm2Pk::from_uncompressed_bytes(&[0u8; 10]).is_none(), "short slice must fail");
    }
}
