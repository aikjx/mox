//! ETag 生成工具：小对象直接 MD5 + CRC32C；大对象复用 xuanji-standards。
//!
//! S3 约定：
//! - 普通 PUT → ETag = "md5_hex"（含引号）
//! - MPU → ETag = "md5(concat(md5(part_i)))-N"（含引号）

use hex::ToHex;
use md5::{Digest as Md5Digest, Md5};
use xuanji_standards::etag_crc32c::{crc32c_base64, crc32c_checksum, etag_multipart};

/// 小对象 ETag：MD5 hex，加引号。
pub fn etag_small(data: &[u8]) -> String {
    let mut h = Md5::new();
    h.update(data);
    format!("\"{}\"", h.finalize().encode_hex::<String>())
}

/// CRC32C base64 值（用于 x-amz-checksum-crc32c header）。
pub fn checksum_crc32c_base64(data: &[u8]) -> String {
    crc32c_base64(data)
}

/// CRC32C 数值。
pub fn checksum_crc32c(data: &[u8]) -> u32 {
    crc32c_checksum(data)
}

/// 复用 xuanji-standards 的 MPU 最终 ETag（含引号）。
pub fn etag_for_multipart(part_etags: &[&str]) -> String {
    // etag_multipart 返回 md5hex-N 格式，我们包引号
    format!("\"{}\"", etag_multipart(part_etags))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_small_abc() {
        let e = etag_small(b"abc");
        // md5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(e, "\"900150983cd24fb0d6963f7d28e17f72\"");
    }

    #[test]
    fn etag_small_empty() {
        let e = etag_small(b"");
        assert_eq!(e, "\"d41d8cd98f00b204e9800998ecf8427e\"");
    }

    #[test]
    fn crc32c_known() {
        // CRC32C("123456789") == 0xE3069283
        let v = checksum_crc32c(b"123456789");
        assert_eq!(v, 0xE3069283);
    }
}
