// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! SM3 cryptographic hash algorithm (GM/T 0004-2012).
//!
//! Pure-Rust implementation of the Chinese Commercial Cryptography hash
//! standard, plus HMAC-SM3 (RFC 2104 construction over SM3 block = 64 B).

const SM3_IV: [u32; 8] = [
    0x7380166F,
    0x4914B2B9,
    0x172442D7,
    0xDA8A0600,
    0xA96F30BC,
    0x163138AA,
    0xE38DEE4D,
    0xB0FB0E4E,
];

const BLOCK_BYTES: usize = 64;
const DIGEST_BYTES: usize = 32;

#[inline(always)]
fn p0(x: u32) -> u32 {
    x ^ x.rotate_left(9) ^ x.rotate_left(17)
}

#[inline(always)]
fn p1(x: u32) -> u32 {
    x ^ x.rotate_left(15) ^ x.rotate_left(23)
}

#[inline(always)]
fn ff(j: usize, x: u32, y: u32, z: u32) -> u32 {
    // j is 0..63 inside the round loop
    if j < 16 {
        x ^ y ^ z
    } else {
        (x & y) | (x & z) | (y & z)
    }
}

#[inline(always)]
fn gg(j: usize, x: u32, y: u32, z: u32) -> u32 {
    if j < 16 {
        x ^ y ^ z
    } else {
        (x & y) | ((!x) & z)
    }
}

#[inline(always)]
fn tj(j: usize) -> u32 {
    if j < 16 {
        0x79CC4519
    } else {
        0x7A879D8A
    }
}

fn cf(state: &mut [u32; 8], block: &[u8; BLOCK_BYTES]) {
    // --- 1. Message expansion W[0..68] ---
    let mut w = [0u32; 68];
    for i in 0..16 {
        let mut b4 = [0u8; 4];
        b4.copy_from_slice(&block[i * 4..i * 4 + 4]);
        w[i] = u32::from_be_bytes(b4);
    }
    for j in 16usize..68 {
        // GM/T 0004-2012:
        //   W_j = P1(W_{j-16} XOR W_{j-9} XOR (W_{j-3} <<< 15))
        //         XOR (W_{j-13} <<< 7) XOR W_{j-6}
        let x = w[j - 16] ^ w[j - 9] ^ w[j - 3].rotate_left(15);
        w[j] = p1(x) ^ w[j - 13].rotate_left(7) ^ w[j - 6];
    }

    // --- 2. Extended schedule W'[0..64): W'_j = W_j XOR W_{j+4} ---
    let mut wp = [0u32; 64];
    for j in 0..64 {
        wp[j] = w[j] ^ w[j + 4];
    }

    // --- 3. Load 8 working registers from the chaining value ---
    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    // --- 4. Main loop: 64 rounds (j = 0..63 inclusive) ---
    for j in 0usize..64 {
        let t = tj(j);
        let r = (j as u32) % 32u32;
        // SS1 = ((A <<< 12) + E + (T_j <<< (j mod 32))) <<< 7
        let a12 = a.rotate_left(12);
        let sum = a12.wrapping_add(e).wrapping_add(t.rotate_left(r));
        let ss1 = sum.rotate_left(7);
        // SS2 = SS1 XOR (A <<< 12)
        let ss2 = ss1 ^ a12;
        // TT1 = FF_j(A,B,C) + D + SS2 + W'_j
        let tt1 = ff(j, a, b, c)
            .wrapping_add(d)
            .wrapping_add(ss2)
            .wrapping_add(wp[j]);
        // TT2 = GG_j(E,F,G) + H + SS1 + W_j
        let tt2 = gg(j, e, f, g)
            .wrapping_add(h)
            .wrapping_add(ss1)
            .wrapping_add(w[j]);

        // --- Working register update ---
        d = c;
        c = b.rotate_left(9);
        b = a;
        a = tt1;
        h = g;
        g = f.rotate_left(19);
        f = e;
        e = p0(tt2);
    }

    // --- 5. Davies-Meyer feed-forward XOR ---
    state[0] ^= a;
    state[1] ^= b;
    state[2] ^= c;
    state[3] ^= d;
    state[4] ^= e;
    state[5] ^= f;
    state[6] ^= g;
    state[7] ^= h;
}

/// Compute the 32-byte SM3 digest of `data`.
pub fn sm3(data: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut v = SM3_IV;
    let bit_len = (data.len() as u64).saturating_mul(8);

    let mut i = 0usize;
    while i + BLOCK_BYTES <= data.len() {
        let mut blk = [0u8; BLOCK_BYTES];
        blk.copy_from_slice(&data[i..i + BLOCK_BYTES]);
        cf(&mut v, &blk);
        i += BLOCK_BYTES;
    }

    let mut final_block = [0u8; BLOCK_BYTES];
    let rem = data.len() - i;
    final_block[..rem].copy_from_slice(&data[i..]);
    final_block[rem] = 0x80;
    if rem >= 56 {
        cf(&mut v, &final_block);
        final_block = [0u8; BLOCK_BYTES];
    }
    final_block[56..64].copy_from_slice(&bit_len.to_be_bytes());
    cf(&mut v, &final_block);

    let mut out = [0u8; DIGEST_BYTES];
    for k in 0..8 {
        out[k * 4..k * 4 + 4].copy_from_slice(&v[k].to_be_bytes());
    }
    out
}

pub fn sm3_hex(data: &[u8]) -> String {
    to_hex_lower(&sm3(data))
}

fn to_hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    const HX: &[u8; 16] = b"0123456789abcdef";
    for &b in bytes {
        s.push(HX[(b >> 4) as usize] as char);
        s.push(HX[(b & 0x0F) as usize] as char);
    }
    s
}

/// HMAC-SM3 (RFC 2104) over 64-byte blocks.
pub fn hmac_sm3(key: &[u8], data: &[u8]) -> [u8; DIGEST_BYTES] {
    let mut kb = [0u8; BLOCK_BYTES];
    if key.len() > BLOCK_BYTES {
        let kh = sm3(key);
        kb[..DIGEST_BYTES].copy_from_slice(&kh);
    } else {
        kb[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK_BYTES];
    let mut opad = [0x5Cu8; BLOCK_BYTES];
    for i in 0..BLOCK_BYTES {
        ipad[i] ^= kb[i];
        opad[i] ^= kb[i];
    }
    let mut inner = Vec::with_capacity(BLOCK_BYTES + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let ih = sm3(&inner);
    let mut outer = Vec::with_capacity(BLOCK_BYTES + DIGEST_BYTES);
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&ih);
    sm3(&outer)
}

pub fn hmac_sm3_hex(key: &[u8], data: &[u8]) -> String {
    to_hex_lower(&hmac_sm3(key, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GM/T 0004-2012 Appendix B.1 — 256-bit digest of the three-byte
    /// ASCII string "abc".
    #[test]
    fn t24_sm3_vector_abc() {
        let got = sm3_hex(b"abc");
        assert_eq!(
            got,
            "66c7f0f462eeedd9d1f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba8e0",
            "sm3(\"abc\") mismatch"
        );
    }

    /// GM/T 0004-2012 Appendix B.3 — digest of one million ASCII 'a' bytes.
    #[test]
    fn t24_sm3_vector_1m_a() {
        let big = vec![b'a'; 1_000_000];
        assert_eq!(
            sm3_hex(&big),
            "c8aaf89429554029e231941a2acc0ad61ff2a5acd8fadd25847a3a732b3b02c3"
        );
    }

    /// HMAC-SM3 self-consistency: 32 B output, distinct (k, m) → distinct
    /// tags, deterministic repeatability, and a known 0x0b×16 vs.
    /// "Hi There" tag length sanity check.
    #[test]
    fn t24_hmac_sm3_basic() {
        assert_eq!(hmac_sm3(&[0x0b; 16], b"Hi There").len(), 32);
        assert_ne!(
            hmac_sm3(&[0x0b; 16], b"Hi There"),
            hmac_sm3(&[0x0c; 16], b"Hi There")
        );
    }
}


#[test]
fn t24_debug_sm3_hex_len() {
    let d = sm3(b"abc");
    let h = sm3_hex(b"abc");
    eprintln!("digest_bytes_len={}, hex_len={}", d.len(), h.len());
    eprintln!("digest_bytes = {:02x?}", &d[..]);
    eprintln!("hex = {}", h);
    let expected_hex = "66c7f0f462eedd9d1d2f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba0e4";
    eprintln!("exp = {}", expected_hex);
    // parse expected to bytes
    let mut exp_bytes = [0u8; 32];
    for i in 0..32 {
        let byte_str = &expected_hex[i*2..i*2+2];
        exp_bytes[i] = u8::from_str_radix(byte_str, 16).unwrap();
    }
    eprintln!("byte-diff positions:");
    for i in 0..32 {
        if d[i] != exp_bytes[i] {
            eprintln!("  [{}] got {:02x} expected {:02x}", i, d[i], exp_bytes[i]);
        }
    }
}

#[test]
fn t24_debug_w_schedule_for_abc_block() {
    // Build the abc-padded block.
    let mut block = [0u8; 64];
    block[0] = b'a';
    block[1] = b'b';
    block[2] = b'c';
    block[3] = 0x80;
    let bit_len: u64 = 24;
    block[56..64].copy_from_slice(&bit_len.to_be_bytes());

    // Run W expansion using my inline code.
    let mut w = [0u32; 68];
    for i in 0..16 {
        let mut b4 = [0u8; 4];
        b4.copy_from_slice(&block[i * 4..i * 4 + 4]);
        w[i] = u32::from_be_bytes(b4);
    }
    eprintln!("W[0]  = {:08x}", w[0]);
    eprintln!("W[15] = {:08x}", w[15]);
    // Manually compute P1(0x61626380) step by step.
    let x0 = 0x61626380u32;
    eprintln!("x for W[16] = W[0] = {:08x}", x0);
    let x_r15 = x0.rotate_left(15);
    let x_r23 = x0.rotate_left(23);
    eprintln!("x.rol(15) = {:08x}, x.rol(23) = {:08x}", x_r15, x_r23);
    let p1_x = x0 ^ x_r15 ^ x_r23;
    eprintln!("P1(x) = {:08x}", p1_x);
    eprintln!("W[13] rol(7) should be 0 because W[13]=0: W[13]={:08x}", w[13]);
    eprintln!("W[10] should be 0: W[10]={:08x}", w[10]);

    for j in 16usize..68 {
        let x = w[j - 16] ^ w[j - 9] ^ w[j - 3].rotate_left(15);
        w[j] = p1(x) ^ w[j - 13].rotate_left(7) ^ w[j - 6];
    }
    eprintln!("Computed W[16] = {:08x}", w[16]);
    eprintln!("Computed W[17] = {:08x}", w[17]);
    eprintln!("Computed W[18] = {:08x}", w[18]);

    // Run CF with full register dump after 64 rounds (BEFORE feed-forward XOR).
    let mut v = SM3_IV;
    // Inline CF but capture (a,b,c,d,e,f,g,h) BEFORE XOR with chain.
    let mut ww = [0u32; 68];
    for i in 0..16 {
        let mut b4 = [0u8; 4];
        b4.copy_from_slice(&block[i * 4..i * 4 + 4]);
        ww[i] = u32::from_be_bytes(b4);
    }
    for j in 16usize..68 {
        let x = ww[j - 16] ^ ww[j - 9] ^ ww[j - 3].rotate_left(15);
        ww[j] = p1(x) ^ ww[j - 13].rotate_left(7) ^ ww[j - 6];
    }
    let mut wp = [0u32; 64];
    for j in 0..64 { wp[j] = ww[j] ^ ww[j + 4]; }
    let mut a = v[0]; let mut b = v[1]; let mut c = v[2]; let mut d = v[3];
    let mut e = v[4]; let mut f = v[5]; let mut g = v[6]; let mut h = v[7];
    for j in 0usize..64 {
        let t = tj(j);
        let r = (j as u32) % 32u32;
        let a12 = a.rotate_left(12);
        let sum = a12.wrapping_add(e).wrapping_add(t.rotate_left(r));
        let ss1 = sum.rotate_left(7);
        let ss2 = ss1 ^ a12;
        let tt1 = ff(j, a, b, c).wrapping_add(d).wrapping_add(ss2).wrapping_add(wp[j]);
        let tt2 = gg(j, e, f, g).wrapping_add(h).wrapping_add(ss1).wrapping_add(ww[j]);
        d = c; c = b.rotate_left(9); b = a; a = tt1;
        h = g; g = f.rotate_left(19); f = e; e = p0(tt2);
    }
    eprintln!("After 64 rounds (BEFORE feed-forward):");
    eprintln!("  A={:08x} B={:08x} C={:08x} D={:08x}", a, b, c, d);
    eprintln!("  E={:08x} F={:08x} G={:08x} H={:08x}", e, f, g, h);
    v[0] ^= a; v[1] ^= b; v[2] ^= c; v[3] ^= d;
    v[4] ^= e; v[5] ^= f; v[6] ^= g; v[7] ^= h;
    let mut out = [0u8; 32];
    for k in 0..8 { out[k * 4..k * 4 + 4].copy_from_slice(&v[k].to_be_bytes()); }
    eprintln!("Final digest bytes: {:02x?}", &out[..]);

    // -------- Try W VARIANT: wrap ROTL terms inside P1 differently --------
    // Variant: W[j] = P1(W[j-16] ^ W[j-9] ^ W[j-3].rol(15) ^ W[j-13].rol(7)) ^ W[j-6]
    let mut wv = [0u32; 68];
    for i in 0..16 { wv[i] = ww[i]; }
    for j in 16usize..68 {
        let x = wv[j-16] ^ wv[j-9] ^ wv[j-3].rotate_left(15) ^ wv[j-13].rotate_left(7);
        wv[j] = p1(x) ^ wv[j-6];
    }
    let mut wpv = [0u32; 64];
    for j in 0..64 { wpv[j] = wv[j] ^ wv[j+4]; }
    let mut av = v[0]^a; let mut bv = v[1]^b; let mut cv = v[2]^c; let mut dv = v[3]^d;
    let mut ev = v[4]^e; let mut fv = v[5]^f; let mut gv = v[6]^g; let mut hv = v[7]^h;
    for j in 0..64 {
        let t = tj(j);
        let r = (j as u32) % 32u32;
        let a12 = av.rotate_left(12);
        let sum = a12.wrapping_add(ev).wrapping_add(t.rotate_left(r));
        let ss1 = sum.rotate_left(7);
        let ss2 = ss1 ^ a12;
        let tt1 = ff(j, av, bv, cv).wrapping_add(dv).wrapping_add(ss2).wrapping_add(wpv[j]);
        let tt2 = gg(j, ev, fv, gv).wrapping_add(hv).wrapping_add(ss1).wrapping_add(wv[j]);
        dv = cv; cv = bv.rotate_left(9); bv = av; av = tt1;
        hv = gv; gv = fv.rotate_left(19); fv = ev; ev = p0(tt2);
    }
    let iv = SM3_IV;
    let o2 = [iv[0]^av, iv[1]^bv, iv[2]^cv, iv[3]^dv, iv[4]^ev, iv[5]^fv, iv[6]^gv, iv[7]^hv];
    let mut out2 = [0u8; 32];
    for k in 0..8 { out2[k*4..k*4+4].copy_from_slice(&o2[k].to_be_bytes()); }
    use std::fmt::Write;
    let mut h2 = String::new();
    for &b in &out2 { let _ = write!(h2, "{:02x}", b); }
    eprintln!("Variant W digest hex: {}", h2);
    let exp = "66c7f0f462eedd9d1d2f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba0e4";
    eprintln!("Variant matches expected: {}", h2 == exp);
}
#[test]
fn t24_debug_try_ss1_variants() {
    let exp = "66c7f0f462eedd9d1d2f2d46bdc10e4e24167c4875cf2f7a2297da02b8f4ba0e4";
    let mut block = [0u8; 64];
    block[0] = b'a'; block[1] = b'b'; block[2] = b'c'; block[3] = 0x80;
    block[56..64].copy_from_slice(&24u64.to_be_bytes());

    // Standard W schedule.
    let mut w = [0u32; 68];
    for i in 0..16 {
        let mut b4 = [0u8; 4];
        b4.copy_from_slice(&block[i * 4..i * 4 + 4]);
        w[i] = u32::from_be_bytes(b4);
    }
    for j in 16usize..68 {
        let x = w[j - 16] ^ w[j - 9] ^ w[j - 3].rotate_left(15);
        w[j] = p1(x) ^ w[j - 13].rotate_left(7) ^ w[j - 6];
    }
    let wp: Vec<u32> = (0..64).map(|j| w[j] ^ w[j+4]).collect();

    let run = |ss1_fn: &dyn Fn(u32,u32,u32,u32)->(u32,u32)| -> String {
        let mut a = SM3_IV[0]; let mut b = SM3_IV[1]; let mut c = SM3_IV[2]; let mut d = SM3_IV[3];
        let mut e = SM3_IV[4]; let mut f = SM3_IV[5]; let mut g = SM3_IV[6]; let mut h = SM3_IV[7];
        for j in 0usize..64 {
            let t = tj(j);
            let r = (j as u32) % 32;
            let (ss1, ss2) = ss1_fn(a, e, t, r);
            let tt1 = ff(j, a, b, c).wrapping_add(d).wrapping_add(ss2).wrapping_add(wp[j]);
            let tt2 = gg(j, e, f, g).wrapping_add(h).wrapping_add(ss1).wrapping_add(w[j]);
            d = c; c = b.rotate_left(9); b = a; a = tt1;
            h = g; g = f.rotate_left(19); f = e; e = p0(tt2);
        }
        let mut out = [0u8; 32];
        let v = [a^SM3_IV[0], b^SM3_IV[1], c^SM3_IV[2], d^SM3_IV[3],
                 e^SM3_IV[4], f^SM3_IV[5], g^SM3_IV[6], h^SM3_IV[7]];
        for k in 0..8 { out[k*4..k*4+4].copy_from_slice(&v[k].to_be_bytes()); }
        use std::fmt::Write;
        let mut s = String::new();
        for &b in &out { let _ = write!(s, "{:02x}", b); }
        s
    };

    // Variants.
    let v1 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a12 = a.rotate_left(12);
        let s = a12.wrapping_add(e).wrapping_add(t.rotate_left(r));
        let ss1 = s.rotate_left(7);
        (ss1, ss1 ^ a12)
    });
    eprintln!("V1 (task spec SS1): {} match={}", &v1[..20], v1==exp);

    // Variant: A rotate_left(9) instead of 12.
    let v2 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a9 = a.rotate_left(9);
        let s = a9.wrapping_add(e).wrapping_add(t.rotate_left(r));
        let ss1 = s.rotate_left(7);
        (ss1, ss1 ^ a9)
    });
    eprintln!("V2 (A rol 9):      {} match={}", &v2[..20], v2==exp);

    // Variant: T is NOT rotated, just added as-is.
    let v3 = run(&|a:u32,e:u32,t:u32,_r:u32|{
        let a12 = a.rotate_left(12);
        let s = a12.wrapping_add(e).wrapping_add(t);
        let ss1 = s.rotate_left(7);
        (ss1, ss1 ^ a12)
    });
    eprintln!("V3 (T not rotated): {} match={}", &v3[..20], v3==exp);

    // Variant: T rotated by (7-r) or something unusual.
    // Actually let's try T.rotate_left(j), which for j<32 is same as j%32 but for j>=32 differs if impl bug.
    // For Rust rotate_left, the count is masked with 31 for u32, so j and j%32 are identical.

    // Variant: rotate_right instead of left on T.
    let v4 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a12 = a.rotate_left(12);
        let s = a12.wrapping_add(e).wrapping_add(t.rotate_right(r));
        let ss1 = s.rotate_left(7);
        (ss1, ss1 ^ a12)
    });
    eprintln!("V4 (T.ror(r)):     {} match={}", &v4[..20], v4==exp);

    // Variant: A.rotate_left(12) + E then .rol(7), then +T.rol(r) as SS1.
    let v5 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a12 = a.rotate_left(12);
        let s = (a12.wrapping_add(e)).rotate_left(7).wrapping_add(t.rotate_left(r));
        let ss1 = s;
        (ss1, ss1 ^ a12)
    });
    eprintln!("V5 (alt bracket):  {} match={}", &v5[..20], v5==exp);

    // Variant: Rol(T, j%32 + 7) instead of rol then + then rol 7.
    let v6 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a12 = a.rotate_left(12);
        let s = a12.wrapping_add(e).wrapping_add(t.rotate_left((r+7)%32));
        let ss1 = s; // no outer rol
        (ss1, ss1 ^ a12)
    });
    eprintln!("V6 (rol r+7, no 7):{} match={}", &v6[..20], v6==exp);

    // Another popular variant - swapped rotate order, a rol 12 then +E rol 7? No...
    // Actually try: ((A <<< 12) + (E <<< 7) + T_j <<< j) <<< 0? No.

    // Variant: Try E.rotate_left(something):
    let v7 = run(&|a:u32,e:u32,t:u32,r:u32|{
        let a12 = a.rotate_left(12);
        let e7 = e.rotate_left(7);
        let s = a12.wrapping_add(e7).wrapping_add(t.rotate_left(r));
        let ss1 = s;
        let ss2 = ss1 ^ a.rotate_left(12);
        (ss1, ss2)
    });
    eprintln!("V7 (E.rol(7)):     {} match={}", &v7[..20], v7==exp);
}
#[test]
fn t24_sample2_standard() {
    // Sample 2 from GM/T 0004-2012 Appendix B.2: 64 bytes, 1 block
    let s = b"abcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcdabcd";
    assert_eq!(s.len(), 64, "need exactly 1 block for sample 2");
    let got = sm3_hex(s);
    // GM/T 0004-2012 Appendix B.2 官方向量
    let exp = "debe9ff92275b8a138604889c18e5a4d6fdb70e5387e5765293dcba39c0c5732";
    eprintln!("sample2 got: {}", got);
    eprintln!("sample2 exp: {}", exp);
    assert_eq!(got, exp, "SM3 sample2 64B single block mismatch");
}

#[test]
fn t24_1ma_sanity_check_blocks() {
    let n = 1_000_000;
    let data: Vec<u8> = std::iter::repeat(b'a').take(n).collect();
    let got = sm3_hex(&data);
    let exp = "c8aaf89429554029e231941a2acc0ad61ff2a5acd8fadd25847a3a732b3b02c3";
    eprintln!("1ma got first 24: {}", &got[..24]);
    eprintln!("1ma exp first 24: {}", &exp[..24]);
    // Now compare with: SM3 of the first 2 blocks (128 a's) + remaining expected chaining? Use a short multi-block.
}

#[test]
fn t24_short_multi_65_a() {
    // 65 a's: first block = 64 a's fully processed; second block = 1 'a' + padding.
    // Compute and output, but also compute manually using cf exposed.
    let data65: Vec<u8> = std::iter::repeat(b'a').take(65).collect();
    let got = sm3_hex(&data65);
    eprintln!("65a result: {}", got);

    // Manual: v0 = IV. cf(v0, block=64 a's). v1 = new state. pad final block (a + 0x80 + zeros + length).
    let mut v = SM3_IV;
    let mut blk1 = [b'a'; 64];
    cf(&mut v, &blk1);
    let mut fb = [0u8; 64];
    fb[0] = b'a';
    fb[1] = 0x80;
    let bit_len: u64 = 65 * 8;
    fb[56..64].copy_from_slice(&bit_len.to_be_bytes());
    cf(&mut v, &fb);
    let mut out = [0u8; 32];
    for k in 0..8 { out[k*4..k*4+4].copy_from_slice(&v[k].to_be_bytes()); }
    use std::fmt::Write;
    let mut manual = String::new();
    for &b in &out { let _ = write!(manual, "{:02x}", b); }
    eprintln!("65a manual: {}", manual);
    assert_eq!(got, manual, "SM3(65a) differs between sm3() API and manual cf()");
}
#[test]
fn t24_debug_1ma_api_vs_manual_cf() {
    let n = 1_000_000usize;
    let data: Vec<u8> = std::iter::repeat(b'a').take(n).collect();

    // (1) Via public sm3() API.
    let api_hex = sm3_hex(&data);

    // (2) Manual: step 15625 full blocks + 1 padding block using cf().
    let mut v = SM3_IV;
    let full_blocks = n / BLOCK_BYTES; // = 15625
    assert_eq!(n % BLOCK_BYTES, 0, "1M is exactly full blocks");
    let blk_a = [b'a'; BLOCK_BYTES];
    for _ in 0..full_blocks {
        cf(&mut v, &blk_a);
    }
    let mut fb = [0u8; BLOCK_BYTES];
    fb[0] = 0x80; // no remaining a's.
    let bit_len: u64 = (n as u64) * 8;
    fb[56..64].copy_from_slice(&bit_len.to_be_bytes());
    cf(&mut v, &fb);
    let mut out = [0u8; 32];
    for k in 0..8 { out[k*4..k*4+4].copy_from_slice(&v[k].to_be_bytes()); }
    use std::fmt::Write;
    let mut manual = String::new();
    for &b in &out { let _ = write!(manual, "{:02x}", b); }

    eprintln!("api:    {}", api_hex);
    eprintln!("manual: {}", manual);
    let exp = "c8aaf89429554029e231941a2acc0ad61ff2a5acd8fadd25847a3a732b3b02c3";
    eprintln!("exp:    {}", exp);

    assert_eq!(api_hex, manual, "sm3() API differs from manual cf for 1M a's");
    assert_eq!(api_hex, exp, "SM3(1M a) mismatch standard");
}