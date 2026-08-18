//! # 标准接口契约（RFC 9457 Problem+JSON）
//!
//! 本模块提供算子统一系统对外 HTTP 接口的**最规范标准错误契约**：
//!
//! 1. [`ProblemDetail`] — 完全对齐 IETF RFC 9457 `application/problem+json`
//!    标准字段（`type` / `title` / `status` / `detail` / `instance`）+ 业界通用的
//!    业务扩展码 `code`。
//! 2. [`ApiError`] / [`ApiResult`] — handler 可直接 `return Err(ApiError::BadRequest(..))`，
//!    自动序列化为 Problem+JSON 并带正确 HTTP 状态码。
//! 3. [`standardize_response`] — **零侵入响应中间件**：拦截「HTTP 200 + 业务
//!    `{success:false}`」的伪成功响应，改写为正确的 4xx 状态码 + Problem+JSON。
//!    这是修复现网缺陷（前端把每次失败当成功）的标准化地基，无需改动任何 handler。
//!
//! 所有对外接口的错误响应统一为：
//! ```json
//! {
//!   "type": "about:blank",
//!   "title": "Bad Request",
//!   "status": 400,
//!   "detail": "算子 linear 不存在",
//!   "instance": "/api/execute",
//!   "code": "BAD_REQUEST"
//! }
//! ```

// 预留公开 API / 未接入管线的能力面（如插件总线、算子目录、优化器 DAG、RBAC 之外的合规结构）：显式允许 dead_code 而非删除，避免破坏能力面；后续接入时自然消除。
#![allow(dead_code)]
use axum::body::to_bytes;
use axum::extract::Request;
use axum::http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// RFC 9457 Problem+JSON 统一错误体。
///
/// 字段严格对齐 RFC 9457；`code` 为业界通用的业务错误码扩展（非 RFC 标准，
/// 但 OpenAPI / AWS / Google API 均使用类似扩展）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetail {
    /// `type`：问题类型 URI（RFC 9457 标准字段）。默认 `about:blank`。
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub problem_type: Option<String>,
    /// `title`：人类可读的错误概要（同 HTTP 状态码的规范原因短语）。
    pub title: String,
    /// `status`：HTTP 状态码（RFC 9457 标准字段，可空以兼容透传）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// `detail`：人类可读的具体错误说明。
    pub detail: String,
    /// `instance`：发生问题的具体资源 URI（RFC 9457 标准字段）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    /// 业务错误码（扩展字段）：如 `NOT_FOUND` / `BAD_REQUEST` / `BUSINESS_ERROR`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl ProblemDetail {
    /// 构造标准 Problem 错误体。
    pub fn new(status: StatusCode, detail: impl Into<String>, code: Option<String>) -> Self {
        Self {
            problem_type: Some("about:blank".to_string()),
            title: status_canonical_title(status),
            status: Some(status.as_u16()),
            detail: detail.into(),
            instance: None,
            code,
        }
    }

    /// 设置发生问题的具体资源 URI。
    pub fn with_instance(mut self, instance: impl Into<String>) -> Self {
        self.instance = Some(instance.into());
        self
    }
}

fn status_canonical_title(status: StatusCode) -> String {
    status
        .canonical_reason()
        .unwrap_or("Unknown Error")
        .to_string()
}

impl IntoResponse for ProblemDetail {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        (
            StatusCode::from_u16(self.status.unwrap_or(500))
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            headers,
            Json(self),
        )
            .into_response()
    }
}

/// 业务 handler 统一错误类型。
///
/// 直接 `return Err(ApiError::Forbidden(..))` 即可返回标准 Problem+JSON。
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized,
    Forbidden(String),
    Conflict(String),
    Internal(String),
    ServiceUnavailable(String),
}

impl ApiError {
    fn parts(&self) -> (StatusCode, String, Option<String>) {
        match self {
            ApiError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone(), Some("NOT_FOUND".into())),
            ApiError::BadRequest(m) => {
                (StatusCode::BAD_REQUEST, m.clone(), Some("BAD_REQUEST".into()))
            }
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "未授权访问".into(),
                Some("UNAUTHORIZED".into()),
            ),
            ApiError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone(), Some("FORBIDDEN".into())),
            ApiError::Conflict(m) => (StatusCode::CONFLICT, m.clone(), Some("CONFLICT".into())),
            ApiError::Internal(m) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                m.clone(),
                Some("INTERNAL_ERROR".into()),
            ),
            ApiError::ServiceUnavailable(m) => (
                StatusCode::SERVICE_UNAVAILABLE,
                m.clone(),
                Some("SERVICE_UNAVAILABLE".into()),
            ),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, detail, code) = self.parts();
        ProblemDetail::new(status, detail, code).into_response()
    }
}

/// handler 返回类型别名：`Result<T, ApiError>`。
pub type ApiResult<T> = Result<T, ApiError>;

/// 响应标准化中间件。
///
/// 修复现网缺陷：后端业务错误以 `HTTP 200 + {success:false}` 返回，导致前端
/// 把每次失败当成功处理。本中间件拦截此类伪成功响应，改写为正确的 4xx 状态码
/// 以及 RFC 9457 Problem+JSON，同时保留 `error` / `message` / `code` / `instance`
/// 等字段语义。非失败响应原样透传，不影响任何正常逻辑。
pub async fn standardize_response(req: Request, next: Next) -> Response {
    let response = next.run(req).await;
    if !response.status().is_success() {
        return response;
    }
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !content_type.contains("application/json") {
        return response;
    }
    let (parts, body) = response.into_parts();
    let _ = parts; // 成功响应原样重建为 200 + application/json（保留 body）
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR).into_response(),
    };
    let val: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return rebuild_ok(val_lossy(&bytes)),
    };
    let is_failure = val
        .get("success")
        .and_then(|v| v.as_bool())
        .map(|b| !b)
        .unwrap_or(false);
    if !is_failure {
        return rebuild_ok(val);
    }

    let detail = val
        .get("error")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("message").and_then(|v| v.as_str()))
        .unwrap_or("请求处理失败")
        .to_string();
    let code = val
        .get("code")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("error_code").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .or_else(|| derive_code_from_failure(&val));
    let status = derive_status_from_failure(&val).unwrap_or(StatusCode::BAD_REQUEST);
    let mut problem = ProblemDetail::new(status, detail, code);
    problem.instance = val
        .get("instance")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (
        status,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )],
        Json(problem),
    )
        .into_response()
}

fn rebuild_ok(val: Value) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    (StatusCode::OK, headers, Json(val)).into_response()
}

/// 解析失败时的兜底：直接回原始字节（理论上不会触发，因为 content_type 已校验）。
fn val_lossy(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn derive_status_from_failure(val: &Value) -> Option<StatusCode> {
    // 优先读显式 HTTP 状态码字段
    for key in ["http_status", "status"] {
        if let Some(n) = val.get(key).and_then(|v| v.as_u64()) {
            if let Ok(s) = StatusCode::from_u16(n as u16) {
                return Some(s);
            }
        }
    }
    // 语义推断：资源不存在类错误 → 404（而非一律 400），
    // 使 `流程不存在`、`未找到记录` 等业务失败具备正确的 HTTP 语义。
    let detail = val
        .get("error")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("message").and_then(|v| v.as_str()))
        .unwrap_or("");
    let lower = detail.to_lowercase();
    if detail.contains("不存在")
        || detail.contains("未找到")
        || lower.contains("not found")
        || lower.contains("not_found")
        || lower.contains("no such")
    {
        return Some(StatusCode::NOT_FOUND);
    }
    None
}

fn derive_code_from_failure(val: &Value) -> Option<String> {
    let detail = val
        .get("error")
        .and_then(|v| v.as_str())
        .or_else(|| val.get("message").and_then(|v| v.as_str()))
        .unwrap_or("");
    let lower = detail.to_lowercase();
    if detail.contains("不存在") || detail.contains("未找到") || lower.contains("not found") {
        return Some("NOT_FOUND".to_string());
    }
    Some("BUSINESS_ERROR".to_string())
}
