// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! S3 Multipart ETag 与 CRC32C 校验和 — 纯 Rust 实现，无外部额外依赖。
//!
//! AWS S3 分片上传 ETag 规则：
//!   ETag = md5( concat( decode_hex(part_etag_without_quotes) for part in parts ) ) + "-" + num_parts

// ============================================================================
// CRC32C (Castagnoli) — polynomial 0x1EDC6F41, reflected, 256-entry LUT
// ============================================================================
fn crc32c_lut() -> [u32; 256] {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut crc = i as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0x82F63B78 // reflected poly 0x1EDC6F41
            } else {
                crc >> 1
            };
        }
        *slot = crc;
    }
    table
}

/// CRC32C (Castagnoli) 校验和。
pub fn crc32c_checksum(data: &[u8]) -> u32 {
    let table = crc32c_lut();
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        let idx = ((crc ^ b as u32) & 0xFF) as usize;
        crc = (crc >> 8) ^ table[idx];
    }
    crc ^ 0xFFFF_FFFF
}

/// 将 crc32c 值转为 base64（AWS x-amz-checksum-crc32c header 格式）。
pub fn crc32c_base64(data: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let v = crc32c_checksum(data).to_be_bytes();
    STANDARD.encode(v)
}

// ============================================================================
// MD5 (RFC 1321) — compact pure-Rust implementation
// ============================================================================
struct Md5Ctx {
    state: [u32; 4],
    count: u64, // bits
    buffer: [u8; 64],
}

const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15,
    21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed, 0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

impl Md5Ctx {
    fn new() -> Self {
        Md5Ctx {
            state: [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476],
            count: 0,
            buffer: [0u8; 64],
        }
    }
    fn update(&mut self, mut data: &[u8]) {
        let mut buf_idx = ((self.count / 8) % 64) as usize;
        self.count += (data.len() as u64) * 8;
        // 先填满 buffer
        while !data.is_empty() && buf_idx < 64 {
            self.buffer[buf_idx] = data[0];
            data = &data[1..];
            buf_idx += 1;
            if buf_idx == 64 {
                Self::transform(&mut self.state, &self.buffer);
                buf_idx = 0;
            }
        }
        // 整组直接处理
        while data.len() >= 64 {
            let mut blk = [0u8; 64];
            blk.copy_from_slice(&data[..64]);
            Self::transform(&mut self.state, &blk);
            data = &data[64..];
        }
        // 剩余存入 buffer
        if !data.is_empty() {
            self.buffer[buf_idx..buf_idx + data.len()].copy_from_slice(data);
        }
    }
    fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.count;
        let idx = ((self.count / 8) % 64) as usize;
        let pad_len = if idx < 56 { 56 - idx } else { 120 - idx };
        let padding: Vec<u8> = (0..pad_len)
            .map(|i| if i == 0 { 0x80 } else { 0x00 })
            .collect();
        self.update(&padding);
        self.update(&bit_len.to_le_bytes());
        let mut out = [0u8; 16];
        for (i, &w) in self.state.iter().enumerate() {
            out[i * 4..(i + 1) * 4].copy_from_slice(&w.to_le_bytes());
        }
        out
    }
    fn transform(s: &mut [u32; 4], block: &[u8; 64]) {
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        let (mut a, mut b, mut c, mut d) = (s[0], s[1], s[2], s[3]);
        for i in 0..64 {
            let (f, g): (u32, u32) = match i {
                0..=15 => ((b & c) | ((!b) & d), i as u32),
                16..=31 => ((d & b) | ((!d) & c), ((i as u32) * 5 + 1) % 16),
                32..=47 => (b ^ c ^ d, ((i as u32) * 3 + 5) % 16),
                _ => (c ^ (b | (!d)), ((i as u32) * 7) % 16),
            };
            let rot = a
                .wrapping_add(f)
                .wrapping_add(K[i])
                .wrapping_add(m[g as usize]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rot.rotate_left(S[i]));
        }
        s[0] = s[0].wrapping_add(a);
        s[1] = s[1].wrapping_add(b);
        s[2] = s[2].wrapping_add(c);
        s[3] = s[3].wrapping_add(d);
    }
}

fn md5_sum(data: &[u8]) -> [u8; 16] {
    let mut ctx = Md5Ctx::new();
    ctx.update(data);
    ctx.finalize()
}

/// S3 分片上传最终 ETag 计算。
pub fn etag_multipart(parts_etags: &[&str]) -> String {
    let mut concat = Vec::new();
    for etag in parts_etags {
        let stripped = etag.trim_matches('"');
        let bytes = hex::decode(stripped).unwrap_or_default();
        concat.extend_from_slice(&bytes);
    }
    let digest = md5_sum(&concat);
    format!("{}-{}", hex::encode(digest), parts_etags.len())
}

// Tests internal to confirm md5 correctness
#[cfg(test)]
mod internal {
    use super::*;
    #[test]
    fn md5_empty() {
        assert_eq!(
            hex::encode(md5_sum(b"")),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
    }
    #[test]
    fn md5_a() {
        assert_eq!(
            hex::encode(md5_sum(b"a")),
            "0cc175b9c0f1b6a831c399e269772661"
        );
    }
    #[test]
    fn md5_abc() {
        assert_eq!(
            hex::encode(md5_sum(b"abc")),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }
    #[test]
    fn md5_64byte_boundary() {
        let s = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 64 a's
        assert_eq!(hex::encode(md5_sum(s)), "014842d480b571495a4a0363793f7367");
    }
    #[test]
    fn md5_65byte_multipart_update_consistent() {
        // 65 bytes: compare one-shot update vs split update
        let s = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"; // 65 a's
        let onshot = md5_sum(s);
        let mut ctx = Md5Ctx::new();
        ctx.update(&s[..30]);
        ctx.update(&s[30..50]);
        ctx.update(&s[50..]);
        let split = ctx.finalize();
        assert_eq!(onshot, split, "one-shot vs split must be same");
    }
    #[test]
    fn md5_long_1000_as() {
        let data = vec![b'a'; 1000];
        // multiple small updates vs one big
        let single = md5_sum(&data);
        let mut ctx = Md5Ctx::new();
        for chunk in data.chunks(37) {
            ctx.update(chunk);
        }
        assert_eq!(single, ctx.finalize());
    }
}
