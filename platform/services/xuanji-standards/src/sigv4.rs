//! S3 SigV4 签名算法 — 纯自研实现，不依赖 aws-sig-auth。
//!
//! 严格遵循 AWS Signature Version 4 规范：
//!   CanonicalRequest → StringToSign → Signature
//!
//! Reference: https://docs.aws.amazon.com/IAM/latest/UserGuide/create-signed-request.html

use hex::ToHex;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().encode_hex::<String>()
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key ok");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn uri_encode(s: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b'/' => {
                if encode_slash {
                    out.push_str("%2F");
                } else {
                    out.push('/');
                }
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// 生成 SigV4 Authorization 头与 x-amz-date 头。
///
/// # Arguments
/// - `ak`: Access Key ID
/// - `sk`: Secret Access Key
/// - `region`: AWS region (e.g. "us-east-1")
/// - `service`: AWS service (e.g. "s3")
/// - `method`: HTTP method (uppercase)
/// - `uri`: Request URI path (e.g. "/bucket/key")
/// - `query`: 已排序的 query 对 `[(k, v), ...]`，可为空
/// - `headers`: 已排序的 header 对 `[(name_lowercase, value), ...]`，至少包含 host
/// - `payload_sha256`: body 的 sha256 hex 字符串（空 body 用 `UNSIGNED-PAYLOAD` 或空串 sha256）
///
/// # Returns
/// `(Authorization: String, x_amz_date: String)`
#[allow(clippy::too_many_arguments)] // AWS SigV4 签名头参数齐全是协议要求，保留完整签名而不拆结构体
pub fn sigv4_auth_header(
    ak: &str,
    sk: &str,
    region: &str,
    service: &str,
    method: &str,
    uri: &str,
    query: &[(&str, &str)],
    headers: &[(&str, &str)],
    payload_sha256: &str,
    now_date: Option<&str>,     // YYYYMMDD format — 为可测试性注入
    now_datetime: Option<&str>, // YYYYMMDDTHHMMSSZ format
) -> (String, String) {
    let dt = now_datetime.unwrap_or("");
    let d = now_date.unwrap_or("");
    // 1) CanonicalRequest
    let canonical_uri = uri_encode(uri, false);
    let canonical_query = query
        .iter()
        .map(|(k, v)| format!("{}={}", uri_encode(k, true), uri_encode(v, true)))
        .collect::<Vec<_>>()
        .join("&");
    let canonical_headers = headers
        .iter()
        .map(|(k, v)| format!("{}:{}\n", k.to_lowercase(), v.trim()))
        .collect::<String>();
    let signed_headers = headers
        .iter()
        .map(|(k, _)| k.to_lowercase())
        .collect::<Vec<_>>()
        .join(";");
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.to_uppercase(),
        canonical_uri,
        canonical_query,
        canonical_headers,
        signed_headers,
        payload_sha256
    );
    let cr_hash = sha256_hex(canonical_request.as_bytes());

    // 2) StringToSign
    let algorithm = "AWS4-HMAC-SHA256";
    let credential_scope = format!("{}/{}/{}/aws4_request", d, region, service);
    let string_to_sign = format!("{}\n{}\n{}\n{}", algorithm, dt, credential_scope, cr_hash);

    // 3) Signature
    let k_date = hmac_sha256(format!("AWS4{}", sk).as_bytes(), d.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");
    let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes()).encode_hex::<String>();

    let auth = format!(
        "{} Credential={}/{}, SignedHeaders={}, Signature={}",
        algorithm, ak, credential_scope, signed_headers, signature
    );
    (auth, dt.to_string())
}
