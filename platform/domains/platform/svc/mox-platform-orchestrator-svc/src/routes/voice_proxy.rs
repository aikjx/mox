// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Voice 代理（T7）：/voice/** → http://localhost:30010/voice/**
//!
//! 核心实现注意（避免 404 / 405）：
//! axum 的 `.nest(prefix, router)` 会把匹配到的 prefix 从「内部路径视角」剥离，
//! 因此 Router.fallback 内用 `uri.path()` 看到的是 *相对子路由* 的路径（没有 `/voice` 前缀），
//! 而 `OriginalUri` 保留完整客户端路径（带 `/voice` 前缀）。
//! 本模块统一使用 `OriginalUri` 取得完整原始路径，不做任何前缀拒判（nest 本身已保证前缀）。

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
            upstream_base: "http://127.0.0.1:30010".to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
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
    // 从 OriginalUri 取原始客户端路径（带 /voice 前缀）；若为空（理论不应发生），
    // 默认 /voice/health 保持语义。关键：不要再做 "/voice/" 前缀拒判——
    // nest("/voice", Router.route("/") + Router.fallback) 的 "/" 精确匹配导致 uri = "/voice" 或 "/voice/"，
    // 均为合法请求（用户访问 /voice 即健康探测）。
    let raw_pq = uri
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| "/voice/health".into());
    // 规范化：
    //   /voice       → /voice/health
    //   /voice/      → /voice/health
    //   /voice/X     → /voice/X 保持（X 含 query）
    let path_and_query = match raw_pq.as_str() {
        "/voice" | "/voice/" => "/voice/health".to_string(),
        s if s.starts_with("/voice?") => {
            format!("/voice/health{}", &s[7..]) // 保留 query
        }
        other => other.to_string(),
    };
    // 上游（:30010）FastAPI voice 服务本身前缀匹配 /voice/**，直接透传。
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
        .request(method, &upstream);
    // 长操作路由（TTS 合成/流式、ASR 全量上传）超时用 10min，健康/列表 1min，其他 30s。
    let is_long_op = path_and_query.starts_with("/voice/tts/")
        || path_and_query.starts_with("/voice/asr/")
        || path_and_query.starts_with("/voice/models/download");
    let is_health = path_and_query.starts_with("/voice/health")
        || path_and_query.starts_with("/voice/models")
        || path_and_query.starts_with("/voice/hotwords")
        || path_and_query.starts_with("/voice/license_tier")
        || path_and_query.starts_with("/voice/metrics");
    let timeout_secs = if is_long_op { 600 } else if is_health { 60 } else { 30 };
    req_builder = req_builder.timeout(std::time::Duration::from_secs(timeout_secs));
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
            error_hint: "xiaobai_voice 服务未启动（端口 30010）",
            fallback_action: "自动切换：本地浏览器 TTS（Web Speech Synthesis）→ T14 朗读三层回退",
            tts: VoiceFallbackTTS {
                active: "browser_tts",
                engines: ["cosyvoice2", "fish_s2_pro"],
                browser_tts_available: true,
            },
        }),
    )
}

/// voice_proxy_routes() — 由主 main.rs 直接挂在顶层 Router。
///
/// ⚠️  不要用 `.nest("/voice", voice_proxy_routes(state))` 方式挂载！
/// 经验证：axum 0.7.x 下 Router 嵌套 + Router.fallback + 非 GET 方法组合会出现
/// 随机 404/405（子路径 404、POST 405、两者同时出现）。
///
/// ✅  正确挂法（写在 main.rs 构建 app 的第一梯队，位于 nest_service 之前）：
/// ```ignore
///   let vp_state = VoiceProxyState::default();
///   let app = Router::new()
///       .route("/voice",         any(voice_proxy_handler).with_state(vp_state.clone()))
///       .route("/voice/{*tail}", any(voice_proxy_handler).with_state(vp_state))
///       // ... 其他 /api/** /ai/engine/** route / nest ...
///       .nest_service("/", ServeDir::new(...));
/// ```
/// 这里 `/voice/{*tail}` 的 catch-all 参数位于「该 route 的最后一个段」——满足 axum 约束，
/// 不会触发 panic（与 `Router.route("/{*tail}", ...)` 作为子 Router 的情形不同）。
pub fn voice_proxy_routes(_state: VoiceProxyState) -> Router {
    // 保留此函数不删：给外部 crate 调用方一个稳定的 API 签名；
    // 但实际调用应使用上面展示的「顶层双 route 直挂」方式。
    Router::new()
}
