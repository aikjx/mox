// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! SigV4 鉴权中间件：对每个请求验证 Authorization: AWS4-HMAC-SHA256 ... 签名。
//! 调用 mox_data_standards_core::sigv4::sigv4_auth_header 生成预期签名再比对。
//! 失败 → 403 SignatureDoesNotMatch。

use crate::error::{S3Error, S3Result};
use mox_data_standards_core::sigv4::sigv4_auth_header;
use std::collections::BTreeMap;

/// 鉴权凭证存储（AK/SK 查找表）。
#[derive(Debug, Clone, Default)]
pub struct CredentialStore {
    // ak -> (user_id, sk)
    inner: std::collections::HashMap<String, (String, String)>,
}

impl CredentialStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, ak: String, user_id: String, sk: String) {
        self.inner.insert(ak, (user_id, sk));
    }

    pub fn get_sk(&self, ak: &str) -> Option<String> {
        self.inner.get(ak).map(|(_, sk)| sk.clone())
    }

    pub fn get_user(&self, ak: &str) -> Option<String> {
        self.inner.get(ak).map(|(u, _)| u.clone())
    }
}

/// 解析 Authorization header，提取 component。
#[derive(Debug, Clone, Default)]
pub struct SigV4Parts {
    pub algorithm: String,
    pub ak: String,
    pub date: String, // YYYYMMDD
    pub region: String,
    pub service: String,
    pub signed_headers: Vec<String>,
    pub signature: String,
}

pub fn parse_authorization(auth: &str) -> S3Result<SigV4Parts> {
    // AWS4-HMAC-SHA256 Credential=AK/date/region/service/aws4_request, SignedHeaders=..., Signature=...
    let trimmed = auth.trim();
    let (algo, rest) = trimmed.split_once(' ').ok_or(S3Error::SignatureDoesNotMatch)?;
    if algo != "AWS4-HMAC-SHA256" {
        return Err(S3Error::SignatureDoesNotMatch);
    }

    let mut parts = SigV4Parts { algorithm: algo.to_string(), ..Default::default() };
    for chunk in rest.split(',') {
        let chunk = chunk.trim();
        if let Some((k, v)) = chunk.split_once('=') {
            match k {
                "Credential" => {
                    let segs: Vec<&str> = v.split('/').collect();
                    if segs.len() != 5 {
                        return Err(S3Error::SignatureDoesNotMatch);
                    }
                    parts.ak = segs[0].to_string();
                    parts.date = segs[1].to_string();
                    parts.region = segs[2].to_string();
                    parts.service = segs[3].to_string();
                    if segs[4] != "aws4_request" {
                        return Err(S3Error::SignatureDoesNotMatch);
                    }
                },
                "SignedHeaders" => {
                    parts.signed_headers = v.split(';').map(|s| s.to_string()).collect();
                },
                "Signature" => {
                    parts.signature = v.to_string();
                },
                _ => {},
            }
        }
    }
    if parts.ak.is_empty() || parts.signature.is_empty() {
        return Err(S3Error::SignatureDoesNotMatch);
    }
    Ok(parts)
}

/// 从 query 提取 X-Amz-* 风格（presigned URL 场景）。
pub fn parse_query_creds(
    query: &[(String, String)],
) -> Option<(String, String, String, String, String)> {
    // (X-Amz-Algorithm, X-Amz-Credential, X-Amz-Date, X-Amz-SignedHeaders, X-Amz-Signature)
    let mut algo = None;
    let mut cred = None;
    let mut date = None;
    let mut signed = None;
    let mut sig = None;
    for (k, v) in query {
        match k.as_str() {
            "X-Amz-Algorithm" => algo = Some(v.clone()),
            "X-Amz-Credential" => cred = Some(v.clone()),
            "X-Amz-Date" => date = Some(v.clone()),
            "X-Amz-SignedHeaders" => signed = Some(v.clone()),
            "X-Amz-Signature" => sig = Some(v.clone()),
            _ => {},
        }
    }
    match (algo, cred, date, signed, sig) {
        (Some(a), Some(c), Some(d), Some(s), Some(sig_v)) => Some((a, c, d, s, sig_v)),
        _ => None,
    }
}

/// 验证请求签名（核心逻辑）。
pub fn verify_request(
    method: &str,
    uri: &str,
    query_sorted: &[(String, String)],
    headers: &BTreeMap<String, String>,
    payload_sha256: &str,
    cred_store: &CredentialStore,
) -> S3Result<String> {
    // 1) 优先用 Authorization header；如果没有，尝试 query presigned
    let (parts, date_str, datetime_str, signature_received) = {
        if let Some(auth) = headers.get("authorization") {
            let p = parse_authorization(auth)?;
            let datetime = headers.get("x-amz-date").cloned().unwrap_or_default();
            (p.clone(), p.date.clone(), datetime, p.signature.clone())
        } else if let Some((_algo, cred, date, signed_headers, sig)) =
            parse_query_creds(query_sorted)
        {
            let segs: Vec<&str> = cred.split('/').collect();
            if segs.len() != 5 {
                return Err(S3Error::SignatureDoesNotMatch);
            }
            let parts = SigV4Parts {
                algorithm: "AWS4-HMAC-SHA256".to_string(),
                ak: segs[0].to_string(),
                date: segs[1].to_string(),
                region: segs[2].to_string(),
                service: segs[3].to_string(),
                signed_headers: signed_headers.split(';').map(|s| s.to_string()).collect(),
                signature: sig.clone(),
            };
            (parts, segs[1].to_string(), date, sig)
        } else {
            return Err(S3Error::AccessDenied);
        }
    };

    // 2) 查 AK → SK
    let sk = cred_store.get_sk(&parts.ak).ok_or(S3Error::SignatureDoesNotMatch)?;
    let user_id = cred_store.get_user(&parts.ak).unwrap_or_default();

    // 3) 基于 signed_headers 构造 headers pair（按小写排序）
    let mut signed_headers_lower: Vec<String> =
        parts.signed_headers.iter().map(|s| s.to_lowercase()).collect();
    signed_headers_lower.sort();
    let mut header_pairs: Vec<(&str, &str)> = Vec::new();
    for sh in &signed_headers_lower {
        if let Some(v) = headers.get(sh) {
            header_pairs.push((sh.as_str(), v.as_str()));
        } else {
            return Err(S3Error::SignatureDoesNotMatch);
        }
    }

    // 4) query 对也要排序（不含 X-Amz-Signature 本身）
    let mut qpairs: Vec<(String, String)> = query_sorted.to_vec();
    qpairs.retain(|(k, _)| k != "X-Amz-Signature");
    qpairs.sort_by(|a, b| a.0.cmp(&b.0));
    let qrefs: Vec<(&str, &str)> = qpairs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();

    // 5) 生成预期签名
    let (expected_auth, _) = sigv4_auth_header(
        &parts.ak,
        &sk,
        &parts.region,
        &parts.service,
        method,
        uri,
        &qrefs,
        &header_pairs,
        payload_sha256,
        Some(&date_str),
        Some(&datetime_str),
    );

    // 6) 比对：从 expected_auth 提取 Signature= 后的值
    let expected_sig = expected_auth
        .rsplit_once("Signature=")
        .map(|(_, s)| s.to_string())
        .ok_or(S3Error::InternalError("sig gen".into()))?;

    if expected_sig != signature_received {
        return Err(S3Error::SignatureDoesNotMatch);
    }

    Ok(user_id)
}
