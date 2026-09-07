// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Voice 域路由（L7）：`/voice/v1/*` —— 桥接 melody2score 独立服务 :8012
//!
//! # 架构
//! - melody2score（`projects/melody2score`，FastAPI :8012）为真实音频转谱服务；
//! - 本域在网关侧做**只读桥接**：健康 / 样例 / 识别三个端点透传到上游，
//!   不修改 melody2score 自身行为；上游不可达时返回明确的 502 语义（不静默降级）。
//! - 上游基址可用 `MOX_VOICE_UPSTREAM_URL` 覆盖（默认 `http://127.0.0.1:8012`）。
//!
//! # 状态
//! `ready`：桥接端点全部可路由；真实 200 依赖上游服务在线（与 IAM beta 依赖编排器同理）。

use mox_api_protocol::{ApiResponse, api_error, api_ok};
use axum::{
    extract::State,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde_json::{Value, json};
use std::time::Duration;

/// 桥接共享状态：reqwest 客户端 + 上游基址
#[derive(Clone)]
pub struct VoiceState {
    client: reqwest::Client,
    base: String,
}

impl VoiceState {
    pub fn new() -> Self {
        let base = std::env::var("MOX_VOICE_UPSTREAM_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8012".to_string());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("build voice proxy client");
        Self { client, base }
    }
}

impl Default for VoiceState {
    fn default() -> Self {
        Self::new()
    }
}

async fn proxy_get(state: &VoiceState, path: &str) -> ApiResponse<Value> {
    let url = format!("{}{}", state.base.trim_end_matches('/'), path);
    match state.client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(v) if status.is_success() => api_ok(v),
                Ok(v) => api_error(502, &format!("upstream {} -> {}: {}", url, status, v)),
                Err(e) => api_error(502, &format!("upstream {} 响应解析失败: {}", url, e)),
            }
        }
        Err(e) => api_error(502, &format!("上游 melody2score 不可达（{}）：{}", url, e)),
    }
}

/// GET /voice/v1/health —— 上游服务健康状态（真实探测）
async fn voice_health(State(s): State<VoiceState>) -> ApiResponse<Value> {
    proxy_get(&s, "/api/samples").await
}

/// GET /voice/v1/samples —— 上游样例音频列表
async fn voice_samples(State(s): State<VoiceState>) -> ApiResponse<Value> {
    proxy_get(&s, "/api/samples").await
}

/// POST /voice/v1/recognize —— 旋律识别（透传请求体到上游）
async fn voice_recognize(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let url = format!("{}{}", s.base.trim_end_matches('/'), "/api/recognize");
    let mut req = s.client.post(&url).json(&body);
    // 透传上游可能需要的认证头（如有）
    if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(v) = auth.to_str() {
            req = req.header(axum::http::header::AUTHORIZATION, v);
        }
    }
    match req.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(v) if status.is_success() => api_ok(v),
                Ok(v) => api_error(502, &format!("upstream {} -> {}: {}", url, status, v)),
                Err(e) => api_error(502, &format!("upstream {} 响应解析失败: {}", url, e)),
            }
        }
        Err(e) => api_error(502, &format!("上游 melody2score 不可达（{}）：{}", url, e)),
    }
}

/// 装配 Voice 域路由（自含状态，nest + 外层 `with_state(())` 升级为 `Router<()>`，
/// 与 proxy.rs 同模式，随后经 `upgrade` 提升为 `Router<GatewayState>`）
pub fn build_voice_router() -> Router<()> {
    Router::new()
        .nest(
            "/voice/v1",
            Router::new()
                .route("/health", get(voice_health))
                .route("/samples", get(voice_samples))
                .route("/recognize", post(voice_recognize))
                .with_state(VoiceState::new()),
        )
        .with_state(())
}
