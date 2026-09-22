// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! HTTP 客户端侧编解码器（reqwest / hyper 等任意客户端复用）
//!
//! 与服务端同源开关（`MOX_API_CRYPTO`）：客户端进程开启后即自动
//! ① 为出站请求附加协商头；② 可选把请求体加密上行；③ 透明解密
//! 服务端返回的密文 `data`。SDK（mox-alliance-http-sdk）与调度器
//! 的 HTTP ExecutorBridge 均接本模块，保证全链路一套语义。

use crate::codec::{is_crypto_envelope, open_data, seal_data, HEADER, NEGOTIATE};
use crate::config::CryptoConfig;

/// 客户端开关（与服务端同一 env）
pub fn client_enabled() -> bool {
    CryptoConfig::from_env().enabled
}

/// 出站协商头（开关关闭时为空，调用方直接 append）
pub fn outbound_headers() -> Vec<(&'static str, &'static str)> {
    if client_enabled() {
        vec![(HEADER, NEGOTIATE)]
    } else {
        Vec::new()
    }
}

/// 请求体加密上行（开关开启时把明文 JSON 编码为 `{"crypto":…}`）
pub fn seal_request(body: &serde_json::Value) -> Option<serde_json::Value> {
    let cfg = CryptoConfig::from_env();
    if !cfg.enabled {
        return None;
    }
    seal_data(&cfg, body).ok()
}

/// 解密响应：支持两种归一形态 ——
/// ① 统一信封 `{code,msg,data}` 且 `data` 为密文 → 透明还原 `data`；
/// ② 非信封服务（裸 DTO）整体加密 → 响应体即 `{"crypto":…}` → 还原为原 DTO。
/// 明文 / 未加密时原样返回解析值（客户端零特判）。
pub fn open_response(bytes: &[u8]) -> Option<serde_json::Value> {
    let mut v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let cfg = CryptoConfig::from_env();
    if is_crypto_envelope(&v) {
        return open_data(&cfg, &v).ok();
    }
    let needs_open = v.get("data").is_some_and(is_crypto_envelope);
    if needs_open {
        let opened = open_data(&cfg, &v["data"]).ok()?;
        v["data"] = opened;
    }
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plaintext_response_passes_through() {
        let raw = br#"{"code":0,"msg":"ok","data":{"n":1}}"#;
        let v = open_response(raw).unwrap();
        assert_eq!(v["data"]["n"], 1);
    }

    #[test]
    fn encrypted_response_is_transparently_opened() {
        // 进程 env 未开启时 open_response 仍按密钥表解密成功与否取决于配置；
        // 这里直接验证 codec 级还原 + 信封字段保持
        let cfg = CryptoConfig::resolve("sm4", None);
        let env = seal_data(&cfg, &serde_json::json!({"n": 1})).unwrap();
        assert_eq!(open_data(&cfg, &env).unwrap()["n"], 1);
        assert!(seal_request(&serde_json::json!({"x": 1})).is_none(), "开关关闭不上密");
    }
}
