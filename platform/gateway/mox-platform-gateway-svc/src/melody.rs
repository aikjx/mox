// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! Melody 域路由（L7）：`/melody/v1/*` —— melody2score 转谱桥接（:8012）
//!
//! # 架构
//! - 复用 Voice 域的上游状态（`crate::voice::VoiceState`）与代理函数，同一 melody2score 服务：
//!   音频识别 / 识别样例 / 识别录音 / 保存简谱 MD / 导出表格 / 下载产物；
//! - 上游不可达返回明确 502（不静默降级）；
//! - 上游基址同样由 `MOX_VOICE_UPSTREAM_URL` 覆盖（默认 `http://127.0.0.1:8012`）。
//!
//! # 状态
//! `ready`：桥接端点全部可路由；真实 200 依赖上游服务在线（与 Voice 域同理）。

use mox_api_protocol::{ApiResponse, api_error, api_ok};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde_json::{Value, json};
use std::time::Duration;

use crate::voice::{VoiceState, proxy_get};

/// POST 透传（请求体 + 可选认证头转发到上游指定路径）
async fn proxy_post(state: &VoiceState, path: &str, headers: HeaderMap, body: Value) -> ApiResponse<Value> {
    let url = format!("{}{}", state.base.trim_end_matches('/'), path);
    let mut req = state.client.post(&url).json(&body);
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        if let Ok(v) = auth.to_str() {
            req = req.header(header::AUTHORIZATION, v);
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

/// GET /melody/v1/health —— 上游转谱服务健康（真实探测）
async fn melody_health(State(s): State<VoiceState>) -> ApiResponse<Value> {
    proxy_get(&s, "/api/samples").await
}

/// POST /melody/v1/recognize —— 旋律识别（透传）
async fn melody_recognize(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    proxy_post(&s, "/api/recognize", headers, body).await
}

/// POST /melody/v1/recognize-sample —— 识别内置样例
async fn melody_recognize_sample(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    proxy_post(&s, "/api/recognize-sample", headers, body).await
}

/// POST /melody/v1/recognize-record —— 识别录音
async fn melody_recognize_record(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    proxy_post(&s, "/api/recognize-record", headers, body).await
}

/// POST /melody/v1/save-md —— 保存简谱 Markdown
async fn melody_save_md(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    proxy_post(&s, "/api/save-md", headers, body).await
}

/// POST /melody/v1/export-sheet —— 导出表格
async fn melody_export_sheet(
    State(s): State<VoiceState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    proxy_post(&s, "/api/export-sheet", headers, body).await
}

/// GET /melody/v1/download/{fname} —— 下载转谱产物（音频/MD/表格，流式透传）
async fn melody_download(
    State(s): State<VoiceState>,
    Path(fname): Path<String>,
) -> Response {
    let url = format!("{}{}", s.base.trim_end_matches('/'), format!("/api/download/{}", fname));
    match s.client.get(&url).send().await {
        Ok(resp) => {
            let status = resp.status();
            let ctype = resp
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_string();
            match resp.bytes().await {
                Ok(bytes) => Response::builder()
                    .status(status)
                    .header(header::CONTENT_TYPE, ctype)
                    .body(Body::from(bytes))
                    .unwrap_or_else(|_| Response::new(Body::from("stream error"))),
                Err(e) => Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::from(format!("upstream 响应读取失败: {}", e)))
                    .unwrap_or_else(|_| Response::new(Body::from("stream error"))),
            }
        }
        Err(e) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::from(format!("上游 melody2score 不可达: {}", e)))
            .unwrap_or_else(|_| Response::new(Body::from("stream error"))),
    }
}

/// 装配 Melody 域路由（自含上游状态，nest + 外层 `with_state(())`，与 Voice 同模式）
pub fn build_melody_router() -> Router<()> {
    Router::new()
        .nest(
            "/melody/v1",
            Router::new()
                .route("/health", get(melody_health))
                .route("/recognize", post(melody_recognize))
                .route("/recognize-sample", post(melody_recognize_sample))
                .route("/recognize-record", post(melody_recognize_record))
                .route("/save-md", post(melody_save_md))
                .route("/export-sheet", post(melody_export_sheet))
                .route("/download/{fname}", get(melody_download))
                .with_state(VoiceState::new()),
        )
        .with_state(())
}
