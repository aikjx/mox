// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 归一化线格式（wire format）：接口 `data` 的 gzip 压缩 + SM4-GCM 加密
//!
//! 密文信封（替换统一 API 信封的 `data` 字段值）：
//!
//! ```json
//! {"crypto": {"alg": "SM4-GCM", "zip": "gzip", "nonce": "<b64(12B)>", "ct": "<b64>", "tag": "<b64(16B)>"}}
//! ```
//!
//! 明文 = gzip(canonical_json(data))；AAD 固定为 [`AAD`]，把信封协议版本
//! 绑进认证标签，防止降级/跨协议重放。

use base64::Engine as _;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use rand::RngCore;
use std::io::{Read, Write};

use crate::config::CryptoConfig;
use crate::sm4_gcm::{sm4_gcm_open, sm4_gcm_seal};

/// 协商请求/响应头（客户端表明支持解密；服务端加密后原样回带）
pub const HEADER: &str = "x-mox-crypto";
/// 协商值：SM4-GCM 认证加密 + gzip 压缩
pub const NEGOTIATE: &str = "sm4-gcm+gzip";
/// 附加认证数据（协议版本绑定）
const AAD: &[u8] = b"mox-api-crypto/v1";
/// 密文信封的字段名（`data.crypto`）
const ENVELOPE_KEY: &str = "crypto";

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::STANDARD;

/// 明文 → 密文信封（gzip → SM4-GCM，随机 96-bit nonce）
pub fn seal_data(cfg: &CryptoConfig, data: &serde_json::Value) -> Result<serde_json::Value, String> {
    let json = serde_json::to_vec(data).map_err(|e| format!("data 序列化失败: {e}"))?;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&json).map_err(|e| format!("gzip 写入失败: {e}"))?;
    let gz = enc.finish().map_err(|e| format!("gzip 收尾失败: {e}"))?;

    let mut nonce = [0u8; 12];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let (ct, tag) = sm4_gcm_seal(cfg.key, nonce, AAD, &gz);

    Ok(serde_json::json!({
        ENVELOPE_KEY: {
            "alg": "SM4-GCM",
            "zip": "gzip",
            "nonce": B64.encode(nonce),
            "ct": B64.encode(&ct),
            "tag": B64.encode(tag),
        }
    }))
}

/// 判断一个 JSON 值是否为密文信封
pub fn is_crypto_envelope(v: &serde_json::Value) -> bool {
    v.is_object() && v.get(ENVELOPE_KEY).is_some()
}

/// 密文信封 → 明文（SM4-GCM 校验 → gunzip）；tag 不符即失败（认证加密）
pub fn open_data(cfg: &CryptoConfig, envelope: &serde_json::Value) -> Result<serde_json::Value, String> {
    let c = envelope
        .get(ENVELOPE_KEY)
        .ok_or_else(|| "缺少 crypto 信封字段".to_string())?;
    let nonce_b64 = str_field(c, "nonce")?;
    let ct_b64 = str_field(c, "ct")?;
    let tag_b64 = str_field(c, "tag")?;
    if c.get("alg").and_then(|v| v.as_str()) != Some("SM4-GCM") {
        return Err("不支持的加密算法".into());
    }

    let nonce = B64.decode(nonce_b64).map_err(|e| format!("nonce base64: {e}"))?;
    let ct = B64.decode(ct_b64).map_err(|e| format!("ct base64: {e}"))?;
    let tag = B64.decode(tag_b64).map_err(|e| format!("tag base64: {e}"))?;
    if nonce.len() != 12 || tag.len() != 16 {
        return Err("nonce/tag 长度非法".into());
    }
    let mut n = [0u8; 12];
    n.copy_from_slice(&nonce);
    let mut t = [0u8; 16];
    t.copy_from_slice(&tag);

    let gz = sm4_gcm_open(cfg.key, n, AAD, &ct, t).map_err(|_| "SM4-GCM 认证失败（数据被篡改或密钥不符）".to_string())?;
    let mut json = Vec::new();
    GzDecoder::new(&gz[..])
        .read_to_end(&mut json)
        .map_err(|e| format!("gunzip 失败: {e}"))?;
    serde_json::from_slice(&json).map_err(|e| format!("明文 JSON 解析失败: {e}"))
}

fn str_field<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(|f| f.as_str())
        .ok_or_else(|| format!("crypto 信封缺少字段 {key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CryptoConfig {
        CryptoConfig::resolve("sm4", Some("00112233445566778899aabbccddeeff".into()))
    }

    #[test]
    fn roundtrip_preserves_value() {
        let data = serde_json::json!({"tasks": [{"title": "中文标题", "n": 42}], "ok": true});
        let env = seal_data(&cfg(), &data).unwrap();
        assert!(is_crypto_envelope(&env));
        assert_eq!(open_data(&cfg(), &env).unwrap(), data);
    }

    #[test]
    fn tampered_ct_rejected() {
        let data = serde_json::json!({"v": 1});
        let mut env = seal_data(&cfg(), &data).unwrap();
        {
            let c = env["crypto"].as_object_mut().unwrap();
            let mut ct = B64.decode(c["ct"].as_str().unwrap()).unwrap();
            ct[0] ^= 0xFF;
            c["ct"] = serde_json::Value::String(B64.encode(ct));
        }
        assert!(open_data(&cfg(), &env).unwrap_err().contains("认证失败"));
    }

    #[test]
    fn wrong_key_rejected() {
        let env = seal_data(&cfg(), &serde_json::json!({"v": 1})).unwrap();
        let other = CryptoConfig::resolve("sm4", Some("ffeeddccbbaa99887766554433221100".into()));
        assert!(open_data(&other, &env).is_err());
    }

    #[test]
    fn compression_effective_on_repetitive_payload() {
        let big = serde_json::json!({
            "rows": vec![serde_json::json!({"expert": "code-expert-001", "report_id": "r-1234567890", "steps": ["分析", "验证", "产出"]}); 200]
        });
        let plain = serde_json::to_vec(&big).unwrap().len();
        let env = seal_data(&cfg(), &big).unwrap();
        let ct_len = B64.decode(env["crypto"]["ct"].as_str().unwrap()).unwrap().len();
        assert!(
            ct_len * 4 < plain,
            "重复性大 payload 应显著压缩: plain={plain} ct={ct_len}"
        );
    }

    #[test]
    fn non_envelope_rejected() {
        assert!(open_data(&cfg(), &serde_json::json!({"x": 1})).is_err());
        assert!(!is_crypto_envelope(&serde_json::json!({"x": 1})));
    }
}
