//! Voice 代理（T7）：/voice/** → http://localhost:3717/voice/**
//!
//! 如果 3717 不可达，则返回 AC-22 标准 fallback JSON（而不是 502），保证前端 UI 不崩溃。
//!
//! 依赖：gateway/runtime 的 Cargo.toml 已有 reqwest。

use axum::{
    body::{to_bytes, Body},
    extract::{OriginalUri, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::any,
    Router,
};
use serde::Serialize;

#[derive(Clone)]
pub struct VoiceProxyState {
    pub upstream_base: String,
    pub client: reqwest::Client,
}

impl Default for VoiceProxyState {
    fn default() -> Self {
        Self {
            upstream_base: "http://127.0.0.1:3717".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
struct VoiceFallback {
    ok: bool,
    upstream_unreachable: bool,
    upstream_base: String,
    error_hint: &'static str,
    fallback_action: &'static str,
    tts: VoiceFallbackTTS,
}
#[derive(Debug, Serialize)]
struct VoiceFallbackTTS {
    active: &'static str,
    engines: [&'static str; 2],
    browser_tts_available: bool,
}

pub async fn voice_proxy_handler(
    State(state): State<VoiceProxyState>,
    method: Method,
    headers: HeaderMap,
    OriginalUri(uri): OriginalUri,
    body: Body,
) -> Response {
    // 仅代理 /voice/ 开头路径
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/voice/health".into());
    if !path_and_query.starts_with("/voice/") {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "voice proxy only handles /voice/**"})),
        )
            .into_response();
    }
    let upstream = format!("{}{}", state.upstream_base, path_and_query);

    let body_bytes = match to_bytes(body, 10 * 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return voice_fallback(&state.upstream_base, format!("读取请求体失败：{}", e))
                .into_response()
        }
    };

    let mut req_builder = state
        .client
        .request(method, &upstream)
        .timeout(std::time::Duration::from_secs(10));
    for (k, v) in headers.iter() {
        let ks = k.as_str().to_ascii_lowercase();
        if ks == "host" || ks == "content-length" {
            continue;
        }
        if let Ok(s) = v.to_str() {
            req_builder = req_builder.header(k.as_str(), s);
        }
    }
    req_builder = req_builder.body(body_bytes);

    match req_builder.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            let mut builder = Response::builder().status(status);
            for (k, v) in resp.headers() {
                let ks = k.as_str().to_ascii_lowercase();
                if ks == "connection" || ks == "transfer-encoding" || ks == "content-encoding" {
                    continue;
                }
                if let Ok(s) = v.to_str() {
                    builder = builder.header(k.as_str(), s);
                }
            }
            match resp.bytes().await {
                Ok(b) => builder
                    .body(Body::from(b))
                    .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response()),
                Err(e) => voice_fallback(
                    &state.upstream_base,
                    format!("读取上游响应失败：{}", e),
                )
                .into_response(),
            }
        }
        Err(e) => {
            voice_fallback(&state.upstream_base, format!("上游 unreachable：{}", e))
                .into_response()
        }
    }
}

fn voice_fallback(base: &str, _hint: String) -> (StatusCode, Json<VoiceFallback>) {
    (
        StatusCode::OK, // 200 OK：前端 ChatView 按 ok=false 自动切"浏览器TTS回退"（AC-22）
        Json(VoiceFallback {
            ok: false,
            upstream_unreachable: true,
            upstream_base: base.to_string(),
            error_hint: "xiaobai_voice 服务未启动（端口 3717）",
            fallback_action: "自动切换：本地浏览器 TTS（Web Speech Synthesis）→ T14 朗读三层回退",
            tts: VoiceFallbackTTS {
                active: "browser_tts",
                engines: ["cosyvoice2", "fish_s2_pro"],
                browser_tts_available: true,
            },
        }),
    )
}

pub fn voice_proxy_routes(state: VoiceProxyState) -> Router {
    Router::new()
        // 代理所有 /voice/** 方法与路径
        .route("/voice/{*path}", any(voice_proxy_handler))
        .with_state(state)
}
