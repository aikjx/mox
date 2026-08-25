//! SM2 digital signature algorithm — pure Rust self-implemented.
//!
//! Follows GM/T 0003.2-2012 (256-bit recommended curve sm2p256v1).  Implements
//! 256-bit big integer module (`big256`) using `[u32; 8]` limbs, curve point
//! addition / doubling in Jacobian coordinates, scalar multiplication via
//! double-and-add, and the standard SM2 sign / verify flows including the
//! `ZA` identity hash preamble (`ENTL || ID || a || b || Gx || Gy || xA || yA`).
//!
//! Only raw `(r, s)` signature format is produced; DER conversion helpers are
//! also exposed.

#![allow(clippy::needless_range_loop)]

use core::convert::TryFrom;

// ---------------------------------------------------------------------------
// 256-bit big integer helpers (limbs: little-endian u32, len = 8)
// ---------------------------------------------------------------------------

mod big256 {
    /// 256-bit unsigned integer stored as 8 × u32 little-endian limbs.
    /// Limb 0 is the least significant word.
    pub type U256 = [u32; 8];

    const LIMBS: usize = 8;

    /// Compare two U256 (limbs LE): returns true iff x >= y.
    #[inline]
    pub fn is_ge(x: &U256, y: &U256) -> bool {
        for i in (0..LIMBS).rev() {
            if x[i] > y[i] {
                return true;
            }
            if x[i] < y[i] {
                return false;
            }
        }
        true
    }

    /// Add two U256 (limbs LE), returning (low, carry).
    #[inline]
    pub fn add(x: &U256, y: &U256) -> (U256, u32) {
        let mut r = [0u32; LIMBS];
        let mut carry: u64 = 0;
        for i in 0..LIMBS {
            let s = (x[i] as u64) + (y[i] as u64) + carry;
            r[i] = s as u32;
            carry = s >> 32;
        }
        (r, carry as u32)
    }

    /// Subtract two U256 (limbs LE).  x MUST be >= y.
    #[inline]
    pub fn sub(x: &U256, y: &U256) -> U256 {
        let mut r = [0u32; LIMBS];
        let mut borrow: i64 = 0;
        for i in 0..LIMBS {
            let s = (x[i] as i64) - (y[i] as i64) - borrow;
            if s < 0 {
                r[i] = (s + (1i64 << 32)) as u32;
                borrow = 1;
            } else {
                r[i] = s as u32;
                borrow = 0;
            }
        }
        r
    }

    /// (x + y) mod m.
    pub fn add_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        let (s, carry) = add(x, y);
        if carry != 0 || is_ge(&s, m) {
            // s + carry*2^256 - m.  If carry then s alone is already < m is
            // impossible — 2^256 > m for our curves (m is 256 bits).
            sub_wide(s, carry, m)
        } else {
            s
        }
    }

    /// (x - y) mod m, branchy on underflow.
    pub fn sub_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        if is_ge(x, y) {
            sub(x, y)
        } else {
            // m - (y - x)
            let diff = sub(y, x);
            sub(m, &diff)
        }
    }

    /// Subtract m from (x + carry*2^256), carry in {0,1}.
    fn sub_wide(x: U256, carry: u32, m: &U256) -> U256 {
        // x + carry*2^256 - m, result in [0, m).
        // Since x is < 2^256 and m is 256-bit, with carry ∈ {0,1} the
        // operation is: if carry == 1 the value is in [2^256, 2^257-1); after
        // one subtraction of m (< 2^256) we still may be >= m again.
        let mut acc = x;
        let mut c: i64 = carry as i64;
        // subtract limb by limb m from (acc + c*2^256).
        for i in 0..LIMBS {
            let s = (acc[i] as i64) - (m[i] as i64);
            if s < 0 {
                acc[i] = (s + (1i64 << 32)) as u32;
                c -= 1;
            } else {
                acc[i] = s as u32;
            }
        }
        // after subtraction c is the net carry-out. If c < 0 we added 2^256,
        // i.e. the subtraction underflowed; we then need to add m back.
        if c < 0 {
            let (s2, c2) = add(&acc, m);
            let _ = c2;
            return s2;
        }
        // If acc still >= m, subtract again.
        if is_ge(&acc, m) {
            sub(&acc, m)
        } else {
            acc
        }
    }

    /// Schoolbook multiply → 512-bit result as [u32; 16] LE limbs.
    fn mul_wide(x: &U256, y: &U256) -> [u32; 16] {
        let mut r = [0u32; 16];
        for i in 0..LIMBS {
            let mut carry: u64 = 0;
            for j in 0..LIMBS {
                let prod = (x[i] as u64) * (y[j] as u64) + (r[i + j] as u64) + carry;
                r[i + j] = prod as u32;
                carry = prod >> 32;
            }
            r[i + LIMBS] = carry as u32;
        }
        r
    }

    /// Reduce 512-bit wide integer mod m by a straightforward binary
    /// long-division: walk from the most significant bit (pos = 511) down,
    /// shifting a running remainder left by 1 and adding the next bit from
    /// the input; whenever the remainder >= m, subtract m.  Unquestionably
    /// correct for any 256-bit modulus.
    fn reduce_wide(lo: [u32; 16], m: &U256) -> U256 {
        let mut rem = zero();
        // Process bits from MSB (limb 15) down to LSB (limb 0).
        let mut limb_idx = 15i32;
        while limb_idx >= 0 {
            let limb = lo[limb_idx as usize];
            let mut bit_idx = 31i32;
            while bit_idx >= 0 {
                let bit: u32 = (limb >> bit_idx) & 1;
                // rem <<= 1
                let mut carry = 0u32;
                for i in 0..LIMBS {
                    let new = (rem[i] << 1) | carry;
                    carry = rem[i] >> 31;
                    rem[i] = new;
                }
                // rem[0] LSB = bit
                rem[0] |= bit;
                // if rem >= m: rem -= m
                if carry != 0 || is_ge(&rem, m) {
                    rem = sub_mod(&rem, m, m);
                }
                bit_idx -= 1;
            }
            limb_idx -= 1;
        }
        rem
    }

    /// (x * y) mod m.
    pub fn mul_mod(x: &U256, y: &U256, m: &U256) -> U256 {
        let wide = mul_wide(x, y);
        reduce_wide(wide, m)
    }

    /// Modular exponentiation via square-and-multiply.
    pub fn pow_mod(base: &U256, exp: &U256, m: &U256) -> U256 {
        let mut result = one();
        let mut b = *base;
        if is_ge(&b, m) {
            b = sub(&b, m);
        }
        for i in 0..LIMBS {
            let mut e = exp[i];
            for _ in 0..32 {
                if e & 1 == 1 {
                    result = mul_mod(&result, &b, m);
                }
                e >>= 1;
                b = mul_mod(&b, &b, m);
            }
        }
        result
    }

    /// Modular inverse via Fermat's little theorem.  m MUST be prime.
    pub fn inv_mod(x: &U256, m: &U256) -> U256 {
        let mut exp = sub(m, &two());
        pow_mod(x, &exp, m)
    }

    /// Return true if U256 == 0.
    pub fn is_zero(x: &U256) -> bool {
        x.iter().all(|&l| l == 0)
    }

    /// U256 == 1?
    pub fn is_one(x: &U256) -> bool {
        x[0] == 1 && x.iter().skip(1).all(|&l| l == 0)
    }

    pub fn zero() -> U256 {
        [0u32; 8]
    }
    pub fn one() -> U256 {
        let mut r = [0u32; 8];
        r[0] = 1;
        r
    }
    pub fn two() -> U256 {
        let mut r = [0u32; 8];
        r[0] = 2;
        r
    }

    /// Convert 32 big-endian bytes to U256 (limbs LE).
    pub fn from_be_bytes(b: &[u8; 32]) -> U256 {
        let mut r = [0u32; 8];
        for i in 0..8 {
            let mut w = 0u32;
            for j in 0..4 {
                w = (w << 8) | b[i * 4 + j] as u32;
            }
            r[7 - i] = w;
        }
        r
    }

    /// Convert U256 to 32 big-endian bytes.
    pub fn to_be_bytes(x: &U256) -> [u8; 32] {
        let mut r = [0u8; 32];
        for i in 0..8 {
            let w = x[7 - i];
            r[i * 4] = (w >> 24) as u8;
            r[i * 4 + 1] = (w >> 16) as u8;
            r[i * 4 + 2] = (w >> 8) as u8;
            r[i * 4 + 3] = w as u8;
        }
        r
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

    /// prime p = FFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF00000000FFFFFFFFFFFFFFFF
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
// Jacobian projective point arithmetic over sm2p256v1
// ---------------------------------------------------------------------------

mod ec {
    use super::big256::{self, U256};
    use super::curve_params;

    /// Jacobian projective coordinates: (X : Y : Z), with affine x = X/Z²,
    /// y = Y/Z³.  `infinity` is represented by Z = 0 and any X, Y.
    #[derive(Clone, Copy)]
    pub struct Point {
        pub x: U256,
        pub y: U256,
        pub z: U256,
    }

    impl Point {
        pub fn infinity() -> Self {
            Point {
                x: big256::zero(),
                y: big256::one(),
                z: big256::zero(),
            }
        }

        pub fn is_infinity(&self) -> bool {
            big256::is_zero(&self.z)
        }

        /// Create from affine coordinates (x, y).
        pub fn from_affine(x: &U256, y: &U256) -> Self {
            Point {
                x: *x,
                y: *y,
                z: big256::one(),
            }
        }

        /// Return affine coordinates if point is not at infinity.
        pub fn to_affine(&self) -> Option<(U256, U256)> {
            if self.is_infinity() {
                return None;
            }
            let p = curve_params::p();
            let z_inv = big256::inv_mod(&self.z, &p);
            let z_inv2 = big256::mul_mod(&z_inv, &z_inv, &p);
            let z_inv3 = big256::mul_mod(&z_inv2, &z_inv, &p);
            let ax = big256::mul_mod(&self.x, &z_inv2, &p);
            let ay = big256::mul_mod(&self.y, &z_inv3, &p);
            Some((ax, ay))
        }

        /// Generator G in projective.
        pub fn generator() -> Self {
            Point::from_affine(&curve_params::gx(), &curve_params::gy())
        }
    }

    /// Point double.
    pub fn point_double(p: &Point) -> Point {
        if p.is_infinity() {
            return Point::infinity();
        }
        let prime = curve_params::p();
        let a = curve_params::a();
        // Jacobian doubling formulas:
        //   M = 3(X + Z²)(X - Z²) + a·Z⁴
        //   S = 4X·Y²
        //   X' = M² - 2S
        //   Y' = M(S - X') - 8Y⁴
        //   Z' = 2Y·Z
        let x = p.x;
        let y = p.y;
        let z = p.z;

        let z2 = big256::mul_mod(&z, &z, &prime);
        let z4 = big256::mul_mod(&z2, &z2, &prime);
        let y2 = big256::mul_mod(&y, &y, &prime);
        let y4 = big256::mul_mod(&y2, &y2, &prime);
        // x + z2, x - z2
        let x_plus_z2 = big256::add_mod(&x, &z2, &prime);
        let x_minus_z2 = big256::sub_mod(&x, &z2, &prime);
        let three = limb(3);
        let two = big256::two();
        let eight = limb(8);
        let four = limb(4);
        let prod = big256::mul_mod(&x_plus_z2, &x_minus_z2, &prime);
        let three_prod = big256::mul_mod(&three, &prod, &prime);
        let a_z4 = big256::mul_mod(&a, &z4, &prime);
        let m = big256::add_mod(&three_prod, &a_z4, &prime);

        let four_x_y2 = big256::mul_mod(&four, &big256::mul_mod(&x, &y2, &prime), &prime);
        let m2 = big256::mul_mod(&m, &m, &prime);
        let two_s = big256::mul_mod(&two, &four_x_y2, &prime);
        let x_new = big256::sub_mod(&m2, &two_s, &prime);

        let diff_s_x = big256::sub_mod(&four_x_y2, &x_new, &prime);
        let m_diff = big256::mul_mod(&m, &diff_s_x, &prime);
        let eight_y4 = big256::mul_mod(&eight, &y4, &prime);
        let y_new = big256::sub_mod(&m_diff, &eight_y4, &prime);

        let two_y_z = big256::mul_mod(&two, &big256::mul_mod(&y, &z, &prime), &prime);
        Point {
            x: x_new,
            y: y_new,
            z: two_y_z,
        }
    }

    /// Point add (p + q).  p and q may be equal (double is used elsewhere).
    pub fn point_add(p: &Point, q: &Point) -> Point {
        if p.is_infinity() {
            return *q;
        }
        if q.is_infinity() {
            return *p;
        }
        let prime = curve_params::p();
        // Jacobian add formulas (mixed when q.z=1 handled by standard):
        // U1 = X1·Z2², U2 = X2·Z1²
        // S1 = Y1·Z2³, S2 = Y2·Z1³
        let z1_2 = big256::mul_mod(&p.z, &p.z, &prime);
        let z2_2 = big256::mul_mod(&q.z, &q.z, &prime);
        let z1_3 = big256::mul_mod(&z1_2, &p.z, &prime);
        let z2_3 = big256::mul_mod(&z2_2, &q.z, &prime);

        let u1 = big256::mul_mod(&p.x, &z2_2, &prime);
        let u2 = big256::mul_mod(&q.x, &z1_2, &prime);
        let s1 = big256::mul_mod(&p.y, &z2_3, &prime);
        let s2 = big256::mul_mod(&q.y, &z1_3, &prime);

        if big256::is_ge(&u1, &u2) && big256::is_ge(&u2, &u1) {
            // U1 == U2
            if big256::is_ge(&s1, &s2) && big256::is_ge(&s2, &s1) {
                return point_double(p);
            }
            return Point::infinity();
        }
        // H = U2 - U1, R = S2 - S1
        let h = big256::sub_mod(&u2, &u1, &prime);
        let r = big256::sub_mod(&s2, &s1, &prime);
        let h2 = big256::mul_mod(&h, &h, &prime);
        let h3 = big256::mul_mod(&h2, &h, &prime);
        // X3 = R² - H³ - 2·U1·H²
        let r2 = big256::mul_mod(&r, &r, &prime);
        let two = big256::two();
        let u1_h2 = big256::mul_mod(&u1, &h2, &prime);
        let two_u1_h2 = big256::mul_mod(&two, &u1_h2, &prime);
        let x3_inner = big256::sub_mod(&r2, &h3, &prime);
        let x3 = big256::sub_mod(&x3_inner, &two_u1_h2, &prime);
        // Y3 = R·(U1·H² - X3) - S1·H³
        let diff = big256::sub_mod(&u1_h2, &x3, &prime);
        let r_diff = big256::mul_mod(&r, &diff, &prime);
        let s1_h3 = big256::mul_mod(&s1, &h3, &prime);
        let y3 = big256::sub_mod(&r_diff, &s1_h3, &prime);
        // Z3 = H·Z1·Z2
        let z1z2 = big256::mul_mod(&p.z, &q.z, &prime);
        let z3 = big256::mul_mod(&h, &z1z2, &prime);
        Point { x: x3, y: y3, z: z3 }
    }

    fn limb(n: u32) -> U256 {
        let mut r = big256::zero();
        r[0] = n;
        r
    }

    /// Scalar multiplication using constant-time-ish double-and-add (not
    /// actually constant-time; this code is for correctness, not side-channel
    /// resistance).
    pub fn scalar_mul(k: &U256, p: &Point) -> Point {
        let mut acc = Point::infinity();
        let mut cur = *p;
        for i in 0..8 {
            let mut kw = k[i];
            for _ in 0..32 {
                if kw & 1 == 1 {
                    acc = point_add(&acc, &cur);
                }
                kw >>= 1;
                cur = point_double(&cur);
            }
        }
        acc
    }

    // Silence unused limb warning.
    #[allow(dead_code)]
    fn _limb_silence() -> U256 {
        limb(0)
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
// Convenience: generate / public key / point
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
            if big256::is_zero(&val) {
                continue;
            }
            if !big256::is_ge(&val, &n) {
                return Sm2Sk(bytes);
            }
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
        &self,
        msg: &[u8],
        id: &[u8],
        rng: &mut R,
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
            // k ∈ [1, n-1]
            let k = loop {
                let mut kb = [0u8; 32];
                rng.fill_bytes(&mut kb);
                let v = big256::from_be_bytes(&kb);
                if big256::is_zero(&v) {
                    continue;
                }
                if !big256::is_ge(&v, &n) {
                    break v;
                }
            };
            let (x1, _y1) = ec::scalar_mul(&k, &ec::Point::generator())
                .to_affine()
                .expect("k·G must not be infinity");
            // r = (e + x1) mod n
            let r = big256::add_mod(&e, &x1, &n);
            if big256::is_zero(&r) {
                continue;
            }
            let one = big256::one();
            let r_plus_k = big256::add_mod(&r, &k, &n);
            // If r + k == n ( == 0 mod n) → reject
            if big256::is_zero(&r_plus_k) {
                continue;
            }
            // (1 + d)^-1 mod n
            let d_plus_1 = big256::add_mod(&d, &one, &n);
            let inv_d1 = big256::inv_mod(&d_plus_1, &n);
            // s = (1+d)^-1 * (k - r*d) mod n
            let rd = big256::mul_mod(&r, &d, &n);
            let k_minus_rd = big256::sub_mod(&k, &rd, &n);
            let s = big256::mul_mod(&inv_d1, &k_minus_rd, &n);
            if big256::is_zero(&s) {
                continue;
            }
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

    /// Parse uncompressed bytes back into an Sm2Pk.  Returns None if the
    /// format is wrong (length != 65 or first byte != 0x04).
    pub fn from_uncompressed_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != 65 || b[0] != 0x04 {
            return None;
        }
        let mut x = [0u8; 32];
        let mut y = [0u8; 32];
        x.copy_from_slice(&b[1..33]);
        y.copy_from_slice(&b[33..65]);
        Some(Sm2Pk { x, y })
    }

    /// Verify an SM2 signature.
    pub fn verify(&self, msg: &[u8], id: &[u8], sig: &Sm2Sig) -> bool {
        use big256;
        use curve_params;
        use ec;
        use crate::sm3_hash::sm3;
        let n = curve_params::n();
        let r = big256::from_be_bytes(&sig.r);
        let s = big256::from_be_bytes(&sig.s);
        // r, s in [1, n-1]
        if big256::is_zero(&r) || big256::is_ge(&r, &n) {
            return false;
        }
        if big256::is_zero(&s) || big256::is_ge(&s, &n) {
            return false;
        }
        // ZA
        let za = compute_za(id.len() as u16 * 8, id, self);
        let mut e_parts: Vec<u8> = Vec::with_capacity(za.len() + msg.len());
        e_parts.extend_from_slice(&za);
        e_parts.extend_from_slice(msg);
        let e_bytes = sm3(&e_parts);
        let e = big256::from_be_bytes(&e_bytes);
        // t = (r + s) mod n; if t == 0 fail.
        let t = big256::add_mod(&r, &s, &n);
        if big256::is_zero(&t) {
            return false;
        }
        // (x1', y1') = s·G + t·Q
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
        // R = (e + x1) mod n must == r
        let r_check = big256::add_mod(&e, &x1, &n);
        if big256::is_ge(&r_check, &r) && big256::is_ge(&r, &r_check) {
            true
        } else {
            false
        }
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
            // strip leading 0x00, prepend 0x00 if high bit set.
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
        // Length: if < 128, single byte.
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
        if data.len() < 8 || data[0] != 0x30 {
            return None;
        }
        let (total_len, mut pos) = match data[1] {
            l if l < 0x80 => (l as usize, 2),
            0x81 => (data[2] as usize, 3),
            _ => return None,
        };
        if data.len() != pos + total_len {
            return None;
        }
        fn read_int(data: &[u8], pos: &mut usize) -> Option<[u8; 32]> {
            if data.len() < *pos + 2 || data[*pos] != 0x02 {
                return None;
            }
            let ilen = data[*pos + 1] as usize;
            *pos += 2;
            if ilen == 0 || data.len() < *pos + ilen {
                return None;
            }
            let int_bytes = &data[*pos..*pos + ilen];
            *pos += ilen;
            // Strip leading 0x00 padding if present (sign-byte convention)
            let trimmed = if int_bytes[0] == 0x00 && int_bytes.len() > 1 {
                &int_bytes[1..]
            } else {
                int_bytes
            };
            if trimmed.len() > 32 {
                return None;
            }
            let mut out = [0u8; 32];
            out[32 - trimmed.len()..].copy_from_slice(trimmed);
            Some(out)
        }
        let r = read_int(data, &mut pos)?;
        let s = read_int(data, &mut pos)?;
        if pos != data.len() {
            return None;
        }
        Some(Sm2Sig { r, s })
    }
}

/// Convenience: sign and return lowercase hex of `r || s` (128 chars).
#[cfg(feature = "gm-sm")]
pub fn sm2_sign_hex<R: rand::RngCore + rand::CryptoRng>(
    msg: &[u8],
    id: &[u8],
    sk: &Sm2Sk,
    rng: &mut R,
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
    if sig_hex.len() != 128 {
        return false;
    }
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
        // Test scalar with large high-limb: 2^255 (MSB set) should not yield infinity.
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
        // Also assert bad inputs return None.
        let mut bad = [0u8; 65];
        bad[0] = 0x02; // compressed
        assert!(Sm2Pk::from_uncompressed_bytes(&bad).is_none(), "prefix 0x02 must fail");
        assert!(Sm2Pk::from_uncompressed_bytes(&[0u8; 10]).is_none(), "short slice must fail");
    }
}
