//! SM4-GCM 自主实现 (GM/T 0002-2012 + NIST SP 800-38D).
//!
//! 纯 Rust 自研实现，不依赖任何外部加密库。

// ============================================================
// 1. SM4 常量: SBox, FK, CK
// ============================================================

/// GM/T 0002-2012 附录 B — SBox (256-entry LUT)
/// 行 = 高 4 位, 列 = 低 4 位
const SBOX: [u8; 256] = [
    0xD6, 0x90, 0xE9, 0xFE, 0xCC, 0xE1, 0x3D, 0xB7, 0x16, 0xB6, 0x14, 0xC2, 0x28, 0xFB, 0x2C, 0x05,
    0x2B, 0x67, 0x9A, 0x76, 0x2A, 0xBE, 0x04, 0xC3, 0xAA, 0x44, 0x13, 0x26, 0x49, 0x86, 0x06, 0x99,
    0x9C, 0x42, 0x50, 0xF4, 0x91, 0xEF, 0x98, 0x7A, 0x33, 0x54, 0x0B, 0x43, 0xED, 0xCF, 0xAC, 0x62,
    0xE4, 0xB3, 0x1C, 0xA9, 0xC9, 0x08, 0xE8, 0x95, 0x80, 0xDF, 0x94, 0xFA, 0x75, 0x8F, 0x3F, 0xA6,
    0x47, 0x07, 0xA7, 0xFC, 0xF3, 0x73, 0x17, 0xBA, 0x83, 0x59, 0x3C, 0x19, 0xE6, 0x85, 0x4F, 0xA8,
    0x68, 0x6B, 0x81, 0xB2, 0x71, 0x64, 0xDA, 0x8B, 0xF8, 0xEB, 0x0F, 0x4B, 0x70, 0x56, 0x9D, 0x35,
    0x1E, 0x24, 0x0E, 0x5E, 0x63, 0x58, 0xD1, 0xA2, 0x25, 0x22, 0x7C, 0x3B, 0x01, 0x21, 0x78, 0x87,
    0xD4, 0x00, 0x46, 0x57, 0x9F, 0xD3, 0x27, 0x52, 0x4C, 0x36, 0x02, 0xE7, 0xA0, 0xC4, 0xC8, 0x9E,
    0xEA, 0xBF, 0x8A, 0xD2, 0x40, 0xC7, 0x38, 0xB5, 0xA3, 0xF7, 0xF2, 0xCE, 0xF9, 0x61, 0x15, 0xA1,
    0xE0, 0xAE, 0x5D, 0xA4, 0x9B, 0x34, 0x1A, 0x55, 0xAD, 0x93, 0x32, 0x30, 0xF5, 0x8C, 0xB1, 0xE3,
    0x1D, 0xF6, 0xE2, 0x2E, 0x82, 0x66, 0xCA, 0x60, 0xC0, 0x29, 0x23, 0xAB, 0x0D, 0x53, 0x4E, 0x6F,
    0xD5, 0xDB, 0x37, 0x45, 0xDE, 0xFD, 0x8E, 0x2F, 0x03, 0xFF, 0x6A, 0x72, 0x6D, 0x6C, 0x5B, 0x51,
    0x8D, 0x1B, 0xAF, 0x92, 0xBB, 0xDD, 0xBC, 0x7F, 0x11, 0xD9, 0x5C, 0x41, 0x1F, 0x10, 0x5A, 0xD8,
    0x0A, 0xC1, 0x31, 0x88, 0xA5, 0xCD, 0x7B, 0xBD, 0x2D, 0x74, 0xD0, 0x12, 0xB8, 0xE5, 0xB4, 0xB0,
    0x89, 0x69, 0x97, 0x4A, 0x0C, 0x96, 0x77, 0x7E, 0x65, 0xB9, 0xF1, 0x09, 0xC5, 0x6E, 0xC6, 0x84,
    0x18, 0xF0, 0x7D, 0xEC, 0x3A, 0xDC, 0x4D, 0x20, 0x79, 0xEE, 0x5F, 0x3E, 0xD7, 0xCB, 0x39, 0x48,
];

/// 系统参数 FK
const FK: [u32; 4] = [0xA3B1BAC6, 0x56AA3350, 0x677D9197, 0xB27022DC];

/// 固定密钥 CK[i] (i=0..31), CK[i][j] = (4*i + j)*7 的 8-bit 值.
/// 使用 const 方式预计算, 避免运行时初始化.
const CK: [u32; 32] = {
    let mut arr = [0u32; 32];
    let mut i = 0usize;
    while i < 32 {
        let j0 = ((4 * i + 0) * 7) as u8;
        let j1 = ((4 * i + 1) * 7) as u8;
        let j2 = ((4 * i + 2) * 7) as u8;
        let j3 = ((4 * i + 3) * 7) as u8;
        let val: u32 = ((j0 as u32) << 24) | ((j1 as u32) << 16) | ((j2 as u32) << 8) | (j3 as u32);
        arr[i] = val;
        i += 1;
    }
    arr
};

// ============================================================
// 2. SM4 基本原语
// ============================================================

/// 非线性变换 τ: 4 字节 → 4 字节, SBox 查表逐字节
#[inline(always)]
fn tau(a: u32) -> u32 {
    let b = a.to_be_bytes();
    let mut out = [0u8; 4];
    out[0] = SBOX[b[0] as usize];
    out[1] = SBOX[b[1] as usize];
    out[2] = SBOX[b[2] as usize];
    out[3] = SBOX[b[3] as usize];
    u32::from_be_bytes(out)
}

/// 线性变换 L: B → C = B ⊕ (B<<<2) ⊕ (B<<<10) ⊕ (B<<<18) ⊕ (B<<<24)
#[inline(always)]
fn linear_l(b: u32) -> u32 {
    b ^ b.rotate_left(2) ^ b.rotate_left(10) ^ b.rotate_left(18) ^ b.rotate_left(24)
}

/// 线性变换 L' (密钥扩展使用): B → B ⊕ (B<<<13) ⊕ (B<<<23)
#[inline(always)]
fn linear_lp(b: u32) -> u32 {
    b ^ b.rotate_left(13) ^ b.rotate_left(23)
}

/// T 变换 = L(τ(a))  (加密轮函数使用)
#[inline(always)]
fn t(a: u32) -> u32 {
    linear_l(tau(a))
}

/// T' 变换 = L'(τ(a)) (密钥扩展使用)
#[inline(always)]
fn tp(a: u32) -> u32 {
    linear_lp(tau(a))
}

/// SM4 密钥扩展: 128-bit key → 32 × 32-bit 轮密钥 rk[0..31]
fn sm4_key_expansion(key: [u8; 16]) -> [u32; 32] {
    let mk: [u32; 4] = [
        u32::from_be_bytes(key[0..4].try_into().unwrap()),
        u32::from_be_bytes(key[4..8].try_into().unwrap()),
        u32::from_be_bytes(key[8..12].try_into().unwrap()),
        u32::from_be_bytes(key[12..16].try_into().unwrap()),
    ];
    let mut k = [0u32; 36];
    for i in 0..4 {
        k[i] = mk[i] ^ FK[i];
    }
    let mut rk = [0u32; 32];
    for i in 0..32 {
        let x = k[i + 1] ^ k[i + 2] ^ k[i + 3] ^ CK[i];
        let kip4 = k[i] ^ tp(x);
        k[i + 4] = kip4;
        rk[i] = kip4;
    }
    rk
}

/// SM4 单分组加密: 16B plaintext → 16B ciphertext, 32 轮 + 反序
fn sm4_encrypt_block(rk: &[u32; 32], pt: [u8; 16]) -> [u8; 16] {
    let mut x = [0u32; 36];
    x[0] = u32::from_be_bytes(pt[0..4].try_into().unwrap());
    x[1] = u32::from_be_bytes(pt[4..8].try_into().unwrap());
    x[2] = u32::from_be_bytes(pt[8..12].try_into().unwrap());
    x[3] = u32::from_be_bytes(pt[12..16].try_into().unwrap());

    for i in 0..32 {
        let v = x[i + 1] ^ x[i + 2] ^ x[i + 3] ^ rk[i];
        x[i + 4] = x[i] ^ t(v);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&x[35].to_be_bytes());
    out[4..8].copy_from_slice(&x[34].to_be_bytes());
    out[8..12].copy_from_slice(&x[33].to_be_bytes());
    out[12..16].copy_from_slice(&x[32].to_be_bytes());
    out
}

/// SM4 单分组解密: 使用相同 rk 反序 rk[31-i]
fn sm4_decrypt_block(rk: &[u32; 32], ct: [u8; 16]) -> [u8; 16] {
    let mut x = [0u32; 36];
    x[0] = u32::from_be_bytes(ct[0..4].try_into().unwrap());
    x[1] = u32::from_be_bytes(ct[4..8].try_into().unwrap());
    x[2] = u32::from_be_bytes(ct[8..12].try_into().unwrap());
    x[3] = u32::from_be_bytes(ct[12..16].try_into().unwrap());

    for i in 0..32 {
        let v = x[i + 1] ^ x[i + 2] ^ x[i + 3] ^ rk[31 - i];
        x[i + 4] = x[i] ^ t(v);
    }
    let mut out = [0u8; 16];
    out[0..4].copy_from_slice(&x[35].to_be_bytes());
    out[4..8].copy_from_slice(&x[34].to_be_bytes());
    out[8..12].copy_from_slice(&x[33].to_be_bytes());
    out[12..16].copy_from_slice(&x[32].to_be_bytes());
    out
}

// ============================================================
// 3. GCM 模式: GF(2^128) 乘法, GHASH, CTR, tag
// ============================================================

/// GF(2^128) 逐位乘法 (NIST SP 800-38D 算法 1)
/// 将 block 视为位串 X = x_127..x_0 (x_127 = MSB), 规约多项式
/// f(x) = x^128 + x^7 + x^2 + x + 1 (尾部异或 0x87).
fn gf128_mul(x: &[u8; 16], y: &[u8; 16]) -> [u8; 16] {
    let mut v = *x;
    let mut z = [0u8; 16];
    for i in 0usize..128 {
        let byte_i = i / 8;
        let bit_i = 7 - (i % 8); // MSB first per byte
        if (y[byte_i] >> bit_i) & 1 == 1 {
            for k in 0..16 { z[k] ^= v[k]; }
        }
        // V × x: 左移 1 bit; 若 MSB=1 则 Z[15] ^= 0x87
        let msb = (v[0] & 0x80) != 0;
        for k in 0..16 {
            let carry = if k + 1 < 16 { (v[k + 1] & 0x80) >> 7 } else { 0 };
            v[k] = (v[k] << 1) | carry;
        }
        if msb { v[15] ^= 0x87; }
    }
    z
}

/// GHASH(H, X): X 分 16B 块, Y_0 = 0; Y_i = (Y_{i-1} XOR X_i) * H
fn ghash(h: &[u8; 16], x: &[u8]) -> [u8; 16] {
    let mut y = [0u8; 16];
    let n = x.len();
    let blocks = n / 16;
    let rem = n % 16;
    for b in 0..blocks {
        let mut blk = [0u8; 16];
        blk.copy_from_slice(&x[b * 16..b * 16 + 16]);
        for k in 0..16 { y[k] ^= blk[k]; }
        y = gf128_mul(&y, h);
    }
    if rem > 0 {
        let mut blk = [0u8; 16];
        blk[..rem].copy_from_slice(&x[blocks * 16..]);
        for k in 0..16 { y[k] ^= blk[k]; }
        y = gf128_mul(&y, h);
    }
    y
}

/// 96-bit nonce → J0 = nonce || 0x00000001
fn gcm_j0(nonce: &[u8; 12]) -> [u8; 16] {
    let mut j0 = [0u8; 16];
    j0[0..12].copy_from_slice(nonce);
    j0[15] = 1;
    j0
}

/// counter 递增 (最右 32 bits BE 加 1)
fn gcm_inc(counter: &[u8; 16]) -> [u8; 16] {
    let mut c = *counter;
    let mut v = u32::from_be_bytes(c[12..16].try_into().unwrap());
    v = v.wrapping_add(1);
    c[12..16].copy_from_slice(&v.to_be_bytes());
    c
}

// ============================================================
// 4. 公开 API: seal / open
// ============================================================

/// SM4-GCM authenticated encryption.
///
/// * `key`   — 128-bit SM4 key
/// * `nonce` — 96-bit IV (每次加密必须唯一)
/// * `aad`   — 附加已认证数据 (不加密, 仅校验)
/// * `pt`    — 明文
///
/// 返回 `(密文, 128-bit tag)`
pub fn sm4_gcm_seal(
    key: [u8; 16],
    nonce: [u8; 12],
    aad: &[u8],
    pt: &[u8],
) -> (Vec<u8>, [u8; 16]) {
    let rk = sm4_key_expansion(key);
    // H = SM4(K, 0^128)
    let h_block = sm4_encrypt_block(&rk, [0u8; 16]);
    let j0 = gcm_j0(&nonce);
    let ekj0 = sm4_encrypt_block(&rk, j0);

    // CTR 起始 counter = J0 + 1 (跳过 J0, J0 留给 tag)
    let mut ctr = gcm_inc(&j0);
    let mut ct = Vec::with_capacity(pt.len());
    let n = pt.len();
    let blocks = n / 16;
    let rem = n % 16;
    for b in 0..blocks {
        let ks = sm4_encrypt_block(&rk, ctr);
        for k in 0..16 { ct.push(pt[b * 16 + k] ^ ks[k]); }
        ctr = gcm_inc(&ctr);
    }
    if rem > 0 {
        let ks = sm4_encrypt_block(&rk, ctr);
        for k in 0..rem { ct.push(pt[blocks * 16 + k] ^ ks[k]); }
    }

    // tag = GHASH(H, A || pad0 || C || pad0 || len(A)_64 || len(C)_64) XOR E(K, J0)
    let len_a = (aad.len() as u64) * 8;
    let len_c = (ct.len() as u64) * 8;
    let pad_a = (16 - (aad.len() % 16)) % 16;
    let pad_c = (16 - (ct.len() % 16)) % 16;
    let mut ghash_input = Vec::with_capacity(aad.len() + pad_a + ct.len() + pad_c + 16);
    ghash_input.extend_from_slice(aad);
    ghash_input.extend(std::iter::repeat(0u8).take(pad_a));
    ghash_input.extend_from_slice(&ct);
    ghash_input.extend(std::iter::repeat(0u8).take(pad_c));
    ghash_input.extend_from_slice(&len_a.to_be_bytes());
    ghash_input.extend_from_slice(&len_c.to_be_bytes());

    let s = ghash(&h_block, &ghash_input);
    let mut tag = [0u8; 16];
    for k in 0..16 { tag[k] = s[k] ^ ekj0[k]; }
    (ct, tag)
}

/// SM4-GCM authenticated decryption.
///
/// tag 校验失败返回 Err(String).
pub fn sm4_gcm_open(
    key: [u8; 16],
    nonce: [u8; 12],
    aad: &[u8],
    ct: &[u8],
    tag: [u8; 16],
) -> Result<Vec<u8>, String> {
    let rk = sm4_key_expansion(key);
    let h_block = sm4_encrypt_block(&rk, [0u8; 16]);
    let j0 = gcm_j0(&nonce);
    let ekj0 = sm4_encrypt_block(&rk, j0);

    let len_a = (aad.len() as u64) * 8;
    let len_c = (ct.len() as u64) * 8;
    let pad_a = (16 - (aad.len() % 16)) % 16;
    let pad_c = (16 - (ct.len() % 16)) % 16;
    let mut ghash_input = Vec::with_capacity(aad.len() + pad_a + ct.len() + pad_c + 16);
    ghash_input.extend_from_slice(aad);
    ghash_input.extend(std::iter::repeat(0u8).take(pad_a));
    ghash_input.extend_from_slice(ct);
    ghash_input.extend(std::iter::repeat(0u8).take(pad_c));
    ghash_input.extend_from_slice(&len_a.to_be_bytes());
    ghash_input.extend_from_slice(&len_c.to_be_bytes());
    let s = ghash(&h_block, &ghash_input);
    let mut expected_tag = [0u8; 16];
    for k in 0..16 { expected_tag[k] = s[k] ^ ekj0[k]; }

    // 恒时比较
    let mut diff = 0u8;
    for k in 0..16 { diff |= expected_tag[k] ^ tag[k]; }
    if diff != 0 {
        return Err("SM4-GCM tag verification failed".into());
    }

    // CTR 解密 (同加密 XOR keystream)
    let mut ctr = gcm_inc(&j0);
    let mut pt = Vec::with_capacity(ct.len());
    let n = ct.len();
    let blocks = n / 16;
    let rem = n % 16;
    for b in 0..blocks {
        let ks = sm4_encrypt_block(&rk, ctr);
        for k in 0..16 { pt.push(ct[b * 16 + k] ^ ks[k]); }
        ctr = gcm_inc(&ctr);
    }
    if rem > 0 {
        let ks = sm4_encrypt_block(&rk, ctr);
        for k in 0..rem { pt.push(ct[blocks * 16 + k] ^ ks[k]); }
    }
    Ok(pt)
}

// ============================================================
// 5. 单元测试
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn hex16(s: &str) -> [u8; 16] {
        let v = hex::decode(s).expect("valid hex");
        let mut out = [0u8; 16];
        out.copy_from_slice(&v);
        out
    }

    /// GM/T 0002-2012 Appendix D.1 标准向量
    /// MK = 01234567 89ABCDEF FEDCBA98 76543210
    /// PT = 01234567 89ABCDEF FEDCBA98 76543210
    /// CT (1 次) = 681EDF34 D206965E 86B3E94F 536E4246
    #[test]
    fn t24_sm4_ecb_vector_appendixD() {
        let key = hex16("0123456789abcdeffedcba9876543210");
        let pt  = hex16("0123456789abcdeffedcba9876543210");
        let expected_ct = hex16("681edf34d206965e86b3e94f536e4246");
        let rk = sm4_key_expansion(key);
        let ct = sm4_encrypt_block(&rk, pt);
        assert_eq!(
            ct, expected_ct,
            "SM4 Appendix D.1 mismatch\ngot:      {}\nexpected: {}",
            hex::encode(ct), hex::encode(expected_ct)
        );
        // roundtrip decrypt
        let pt2 = sm4_decrypt_block(&rk, ct);
        assert_eq!(pt2, pt, "SM4 decrypt(encrypt(pt)) != pt");

        // key=0x0102..10 pt=0x0102..10 做 100 次加解密 roundtrip
        let key2: [u8; 16] = [
            0x01,0x02,0x03,0x04,0x05,0x06,0x07,0x08,
            0x09,0x0a,0x0b,0x0c,0x0d,0x0e,0x0f,0x10,
        ];
        let pt2b = key2;
        let rk2 = sm4_key_expansion(key2);
        let mut x = pt2b;
        for _ in 0..100 { x = sm4_encrypt_block(&rk2, x); }
        for _ in 0..100 { x = sm4_decrypt_block(&rk2, x); }
        assert_eq!(x, pt2b, "100-iter SM4 encrypt-then-decrypt failed");

        // seal/open roundtrip 使用附录 D 的 key 和任意 short plaintext
        let nonce_d = [0u8; 12];
        let aad_d = b"";
        let pt_d: Vec<u8> = pt.to_vec();
        let (ct_d, tag_d) = sm4_gcm_seal(key, nonce_d, aad_d, &pt_d);
        let back = sm4_gcm_open(key, nonce_d, aad_d, &ct_d, tag_d)
            .expect("appendixD seal/open should succeed");
        assert_eq!(back, pt_d, "appendixD seal→open roundtrip failed");
    }

    /// 100 次随机 key/nonce/aad/pt seal→open roundtrip
    #[test]
    fn t24_sm4_gcm_roundtrip_100() {
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        for trial in 0usize..100 {
            let mut key = [0u8; 16];
            let mut nonce = [0u8; 12];
            rng.fill_bytes(&mut key);
            rng.fill_bytes(&mut nonce);
            let aad_len = (trial * 7) % 64;
            let pt_len = (trial * 13) % 256 + 1;
            let mut aad = vec![0u8; aad_len];
            let mut pt = vec![0u8; pt_len];
            rng.fill_bytes(&mut aad);
            rng.fill_bytes(&mut pt);

            let (ct, tag) = sm4_gcm_seal(key, nonce, &aad, &pt);
            let got_pt = sm4_gcm_open(key, nonce, &aad, &ct, tag)
                .unwrap_or_else(|e| panic!("trial {} open failed: {}", trial, e));
            assert_eq!(got_pt, pt, "trial {} roundtrip pt mismatch", trial);
            assert_eq!(ct.len(), pt_len, "trial {} ct len should equal pt len", trial);
        }
    }

    /// 篡改 tag 1 bit → open 失败
    #[test]
    fn t24_sm4_gcm_tag_corrupt_fails() {
        let mut key = [0u8; 16];
        let mut nonce = [0u8; 12];
        for i in 0..16 { key[i] = (i+1) as u8; }
        for i in 0..12 { nonce[i] = (i+10) as u8; }
        let aad = b"associated-data example";
        let pt = b"plaintext message 12345";
        let (ct, tag) = sm4_gcm_seal(key, nonce, aad, pt);

        let mut bad_tag = tag;
        bad_tag[7] ^= 1 << 3;
        let res = sm4_gcm_open(key, nonce, aad, &ct, bad_tag);
        assert!(res.is_err(), "corrupted tag should fail open");

        // 原 tag 仍然通过
        let ok = sm4_gcm_open(key, nonce, aad, &ct, tag).expect("valid tag should open");
        assert_eq!(ok, pt);
    }

    /// 篡改 ciphertext 1 bit → open 失败
    #[test]
    fn t24_sm4_gcm_ct_corrupt_fails() {
        let mut key = [0u8; 16];
        let mut nonce = [0u8; 12];
        for i in 0..16 { key[i] = (i+1) as u8; }
        for i in 0..12 { nonce[i] = (i+10) as u8; }
        let aad = b"aad";
        let pt = b"0123456789abcdef0123456789abcdef";
        let (ct, tag) = sm4_gcm_seal(key, nonce, aad, pt);

        let mut bad_ct = ct.clone();
        let n = bad_ct.len();
        bad_ct[n - 1] ^= 0x01;
        let res = sm4_gcm_open(key, nonce, aad, &bad_ct, tag);
        assert!(res.is_err(), "corrupted ct should fail open");

        let good = sm4_gcm_open(key, nonce, aad, &ct, tag).expect("good ct should pass");
        assert_eq!(good, pt);
    }

    /// 篡改 AAD 1 bit → open 失败
    #[test]
    fn t24_sm4_gcm_aad_corrupt_fails() {
        let mut key = [0u8; 16];
        let mut nonce = [0u8; 12];
        for i in 0..16 { key[i] = (i+1) as u8; }
        for i in 0..12 { nonce[i] = (i+10) as u8; }
        let aad = b"authenticated-but-not-encrypted data";
        let pt = b"payload here";
        let (ct, tag) = sm4_gcm_seal(key, nonce, aad, pt);

        let mut bad_aad = aad.to_vec();
        bad_aad[0] ^= 0x80;
        let res = sm4_gcm_open(key, nonce, &bad_aad, &ct, tag);
        assert!(res.is_err(), "corrupted aad should fail open");

        let good = sm4_gcm_open(key, nonce, aad, &ct, tag).expect("good aad should pass");
        assert_eq!(good, pt);
    }
}
