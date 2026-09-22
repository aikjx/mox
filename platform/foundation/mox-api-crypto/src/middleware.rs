// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! axum 中间件：请求体透明解密 + 统一信封 data 压缩/加密响应
//!
//! 挂载方式（各服务一行，全维度归一）：
//!
//! ```ignore
//! Router::new()
//!     // ...业务路由...
//!     .layer(axum::middleware::from_fn(mox_api_crypto::middleware::crypto_middleware))
//! ```
//!
//! 行为：`MOX_API_CRYPTO` 未开启时直通零开销；开启后仅对携带
//! `x-mox-crypto: sm4-gcm+gzip` 协商头的请求做加解密（存量明文客户端无感）。

use axum::body::{to_bytes, Body};
use axum::extract::Request;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use mox_api_protocol::ApiResponse;

use crate::codec::{is_crypto_envelope, open_data, seal_data, HEADER, NEGOTIATE};
use crate::config::CryptoConfig;

/// 缓冲上限：16 MiB（网关/内部服务单响应体安全界）
const MAX_BODY: usize = 16 * 1024 * 1024;

/// 传输加密中间件：配置取自进程环境变量（一键开关，见 [`crate::config`]）
pub async fn crypto_middleware(req: Request, next: Next) -> Response {
    crypto_middleware_with(CryptoConfig::from_env(), req, next).await
}

/// 参数化配置版（测试注入 / 自定义密钥源）
pub async fn crypto_middleware_with(cfg: CryptoConfig, req: Request, next: Next) -> Response {
    if !cfg.enabled || !has_negotiation(&req) {
        return next.run(req).await;
    }

    // ── 请求方向：客户端上送密文信封 {"crypto":…} → 透明解密为明文 JSON ──
    let (mut parts, body) = req.into_parts();
    let raw = match to_bytes(body, MAX_BODY).await {
        Ok(b) => b,
        Err(_) => return ApiResponse::<()>::error(400, "请求体读取失败或超限").into_response(),
    };
    let mut forward: Vec<u8> = raw.to_vec();
    if is_json(&parts.headers) && !raw.is_empty() {
        if let Ok(v) = serde_json::from_slice::<serde_json::Value>(&raw) {
            if is_crypto_envelope(&v) {
                forward = match open_data(&cfg, &v)
                    .map_err(|e| e.to_string())
                    .and_then(|plain| serde_json::to_vec(&plain).map_err(|e| e.to_string()))
                {
                    Ok(b) => b,
                    Err(e) => {
                        return ApiResponse::<()>::error(400, format!("请求体解密失败: {e}"))
                            .into_response()
                    }
                };
            }
        }
    }
    parts.headers.remove(CONTENT_LENGTH);
    if let Ok(cl) = HeaderValue::from_str(&forward.len().to_string()) {
        parts.headers.insert(CONTENT_LENGTH, cl);
    }
    let req = Request::from_parts(parts, Body::from(forward));

    // ── 响应方向：统一信封 data → gzip 压缩 → SM4-GCM 加密 ──
    let res = next.run(req).await;
    let (mut resp_parts, body) = res.into_parts();
    if !is_json(&resp_parts.headers) {
        return Response::from_parts(resp_parts, body);
    }
    let raw = match to_bytes(body, MAX_BODY).await {
        Ok(b) => b,
        Err(_) => {
            return Response::from_parts(resp_parts, Body::from("响应体超限"));
        }
    };
    let mut out: Vec<u8> = raw.to_vec();
    if let Ok(mut v) = serde_json::from_slice::<serde_json::Value>(&raw) {
        let data = match v.get("data") {
            Some(d) if !is_crypto_envelope(d) => Some(d.clone()),
            _ => None,
        };
        let seal_whole = data.is_none()
            && v.is_object()
            && v.get("code").is_none()
            && !is_crypto_envelope(&v);
        let target = if let Some(d) = data {
            Some(d)
        } else if seal_whole {
            // 归一化扩展：非统一信封的服务（如调度器/注册中心返回裸 DTO）
            // 携带协商头时整体加密响应体（体即 {"crypto":…}）
            Some(v.clone())
        } else {
            None
        };
        if let Some(payload) = target {
            match seal_data(&cfg, &payload) {
                Ok(env) => {
                    if seal_whole {
                        out = serde_json::to_vec(&env).unwrap_or(out);
                    } else {
                        v["data"] = env;
                        out = serde_json::to_vec(&v).unwrap_or(out);
                    }
                    resp_parts
                        .headers
                        .insert(HEADER, HeaderValue::from_static(NEGOTIATE));
                }
                Err(_) => return ApiResponse::<()>::error(500, "传输加密处理失败").into_response(),
            }
        }
    }
    resp_parts.headers.remove(CONTENT_LENGTH);
    if let Ok(cl) = HeaderValue::from_str(&out.len().to_string()) {
        resp_parts.headers.insert(CONTENT_LENGTH, cl);
    }
    Response::from_parts(resp_parts, Body::from(out))
}

fn has_negotiation(req: &Request) -> bool {
    req.headers()
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v == NEGOTIATE)
        .unwrap_or(false)
}

fn is_json(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("application/json"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Method, StatusCode};
    use axum::routing::get;
    use axum::Json;
    use tower::ServiceExt;

    fn cfg() -> CryptoConfig {
        CryptoConfig::resolve("sm4", None)
    }

    fn envelope(data: serde_json::Value) -> serde_json::Value {
        serde_json::json!({"code": 0, "msg": "ok", "data": data})
    }

    fn with_layer(app: axum::Router, cfg: CryptoConfig) -> axum::Router {
        app.layer(axum::middleware::from_fn(move |req, next| {
            let c = cfg.clone();
            async move { crypto_middleware_with(c, req, next).await }
        }))
    }

    async fn call(app: axum::Router, req: Request) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
        let res = app.oneshot(req).await.unwrap();
        let (parts, body) = res.into_parts();
        let bytes = to_bytes(body, MAX_BODY).await.unwrap();
        (parts.status, parts.headers, bytes.to_vec())
    }

    fn get_req(uri: &str, negotiate: bool) -> Request {
        let mut b = Request::builder().method(Method::GET).uri(uri);
        if negotiate {
            b = b.header(HEADER, NEGOTIATE);
        }
        b.body(Body::empty()).unwrap()
    }

    fn plain_app() -> axum::Router {
        async fn h() -> Json<serde_json::Value> {
            Json(envelope(serde_json::json!({"secret": "值", "n": 1})))
        }
        axum::Router::new().route("/p", get(h))
    }

    #[test]
    fn negotiated_request_gets_encrypted_data_roundtrip() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let app = with_layer(plain_app(), cfg());
            let (status, headers, bytes) = call(app, get_req("/p", true)).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(headers.get(HEADER).unwrap(), NEGOTIATE, "应回带协商头");
            let outer: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(
                is_crypto_envelope(&outer["data"]),
                "data 应为密文信封: {outer}"
            );
            assert_eq!(outer["code"], 0, "信封 code/msg 保持明文可路由");
            let inner = open_data(&cfg(), &outer["data"]).unwrap();
            assert_eq!(inner, serde_json::json!({"secret": "值", "n": 1}));
        });
    }

    #[test]
    fn no_negotiation_header_stays_plaintext() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let app = with_layer(plain_app(), cfg());
            let (_, headers, bytes) = call(app, get_req("/p", false)).await;
            assert!(headers.get(HEADER).is_none());
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(v["data"]["secret"], "值");
        });
    }

    #[test]
    fn disabled_switch_is_full_passthrough_even_with_negotiation() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let off = CryptoConfig::resolve("off", None);
            let app = with_layer(plain_app(), off);
            let (_, headers, bytes) = call(app, get_req("/p", true)).await;
            assert!(headers.get(HEADER).is_none());
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(v["data"]["secret"], "值", "开关关闭时即使协商也明文（一键回退）");
        });
    }

    #[test]
    fn encrypted_request_body_decrypted_for_handler() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            async fn echo(body: String) -> (StatusCode, Json<serde_json::Value>) {
                let v: serde_json::Value = serde_json::from_str(&body).unwrap();
                (StatusCode::OK, Json(envelope(v)))
            }
            let app = with_layer(axum::Router::new().route("/e", axum::routing::post(echo)), cfg());
            let payload = serde_json::json!({"title": "加密上行"});
            let sealed = seal_data(&cfg(), &payload).unwrap();
            let req = Request::builder()
                .method(Method::POST)
                .uri("/e")
                .header(HEADER, NEGOTIATE)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&sealed).unwrap()))
                .unwrap();
            let (_, _, bytes) = call(app, req).await;
            let outer: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            let inner = open_data(&cfg(), &outer["data"]).unwrap();
            assert_eq!(inner, payload, "handler 应收到解密后的明文并回显");
        });
    }

    #[test]
    fn tampered_request_body_rejected_400() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            let app = with_layer(plain_app(), cfg());
            let mut env = seal_data(&cfg(), &serde_json::json!({"a": 1})).unwrap();
            env["crypto"]["ct"] = serde_json::Value::String("AAAA".into());
            let req = Request::builder()
                .method(Method::POST)
                .uri("/p")
                .header(HEADER, NEGOTIATE)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&env).unwrap()))
                .unwrap();
            let (status, _, _) = call(app, req).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "认证失败必须拒绝");
        });
    }

    #[test]
    fn bare_dto_response_sealed_whole_and_restored() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            async fn raw() -> Json<serde_json::Value> {
                Json(serde_json::json!({"tasks": [], "total": 0}))
            }
            let app = with_layer(axum::Router::new().route("/raw", get(raw)), cfg());
            let (_, _, bytes) = call(app, get_req("/raw", true)).await;
            let outer: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(is_crypto_envelope(&outer), "裸 DTO 应整体加密为 crypto 信封");
            let inner = open_data(&cfg(), &outer).unwrap();
            assert_eq!(inner, serde_json::json!({"tasks": [], "total": 0}));
        });
    }

    #[test]
    fn non_json_response_untouched() {
        let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
        rt.block_on(async {
            async fn text() -> ([(axum::http::HeaderName, &'static str); 1], &'static str) {
                ([(axum::http::header::CONTENT_TYPE, "text/plain")], "hello")
            }
            let app = with_layer(axum::Router::new().route("/t", get(text)), cfg());
            let (_, _, bytes) = call(app, get_req("/t", true)).await;
            assert_eq!(&bytes, b"hello");
        });
    }
}
