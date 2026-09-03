// =============================================================================
// MOX 统一 API 协议层 (mox-api-protocol)
// =============================================================================
// 设计原则：最精简 · 规范 · 标准 · 层次明确 · 模块化
//
// 层次：
//   foundation/mox-api-protocol  ← 本 crate（协议定义，无业务逻辑）
//   foundation/mox-error          ← 错误码体系（被本 crate re-export）
//   framework/*                    ← 工具层（TCP 服务器构建器等）
//   domains/*/svc                  ← 领域服务（使用本协议）
//   gateway/mox-platform-gateway   ← 网关聚合（统一入口）
//
// 响应格式（所有 HTTP 端点统一）：
//   成功：{ "code": 0, "message": "ok", "data": <T> }
//   失败：{ "code": <非0>, "message": "<错误描述>", "data": null }
//
// 错误码：复用 mox-error 的域编码体系（KG01001 / AI02099 / PL00999 ...）
// =============================================================================

#![allow(clippy::needless_doctest_main)]

use serde::{Deserialize, Serialize};

// ─── Re-export 错误码体系（单一事实源） ───────────────────────────────────
pub use mox_error::{
    define_domain_errors, ErrorDomain, ErrorLevel, MoxError, MoxResult,
};

// =============================================================================
// 统一响应封装 ApiResponse<T>
// =============================================================================

/// MOX 平台统一 API 响应体
///
/// 所有 HTTP 端点必须返回此结构（或经 axum IntoResponse 自动转换）。
///
/// # 示例
/// ```
/// use mox_api_protocol::ApiResponse;
///
/// let resp: ApiResponse<String> = ApiResponse::ok("hello".into());
/// assert_eq!(resp.code, 0);
/// assert_eq!(resp.message, "ok");
/// assert_eq!(resp.data.as_deref(), Some("hello"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 业务状态码：0 = 成功，非 0 = 失败（对应 mox-error 域编码或 HTTP 状态）
    pub code: i32,
    /// 人类可读消息
    pub message: String,
    /// 业务数据（失败时为 null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    /// 成功响应
    pub fn ok(data: T) -> Self {
        Self {
            code: 0,
            message: "ok".into(),
            data: Some(data),
        }
    }

    /// 成功但无数据（如 DELETE 操作）
    pub fn ok_empty() -> Self {
        Self {
            code: 0,
            message: "ok".into(),
            data: None,
        }
    }

    /// 错误响应（整数错误码）
    pub fn error(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// 从 MoxError 构建错误响应（code 取 HTTP 状态码，message 取错误消息）
    pub fn from_mox_error(err: &MoxError) -> Self {
        Self {
            code: err.http_status as i32,
            message: err.message.clone(),
            data: None,
        }
    }

    /// 转换 data 类型
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> ApiResponse<U> {
        ApiResponse {
            code: self.code,
            message: self.message,
            data: self.data.map(f),
        }
    }
}

impl<T: Serialize> ApiResponse<T> {
    /// 序列化为 JSON Value（便于在旧代码中临时嵌入）
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

// =============================================================================
// 分页封装 PaginatedResponse<T>
// =============================================================================

/// 统一分页响应
///
/// 所有列表查询端点的 `data` 字段使用此结构。
///
/// # 分页请求参数（统一）
/// - `page`: 页码，从 1 开始，默认 1
/// - `page_size`: 每页条数，默认 20，最大 100
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// 当前页数据
    pub items: Vec<T>,
    /// 总条数
    pub total: u64,
    /// 当前页码（从 1 开始）
    pub page: u32,
    /// 每页条数
    pub page_size: u32,
    /// 总页数
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    /// 创建分页响应（自动计算 total_pages）
    pub fn new(items: Vec<T>, total: u64, page: u32, page_size: u32) -> Self {
        let page_size = page_size.max(1);
        let total_pages = ((total as f64) / (page_size as f64)).ceil() as u32;
        Self {
            items,
            total,
            page,
            page_size,
            total_pages,
        }
    }

    /// 空分页
    pub fn empty(page: u32, page_size: u32) -> Self {
        Self::new(Vec::new(), 0, page, page_size)
    }

    /// 转换元素类型
    pub fn map_items<U, F: FnMut(T) -> U>(self, f: F) -> PaginatedResponse<U> {
        PaginatedResponse {
            items: self.items.into_iter().map(f).collect(),
            total: self.total,
            page: self.page,
            page_size: self.page_size,
            total_pages: self.total_pages,
        }
    }
}

/// 统一分页请求参数（axum Query 提取器）
#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    /// 页码，从 1 开始
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页条数，默认 20，最大 100
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

impl PageQuery {
    /// 规范化：page >= 1，1 <= page_size <= 100
    pub fn normalized(&self) -> (u32, u32) {
        let page = self.page.max(1);
        let page_size = self.page_size.clamp(1, 100);
        (page, page_size)
    }

    /// 计算 offset（用于数据库查询）
    pub fn offset(&self) -> u64 {
        let (page, page_size) = self.normalized();
        ((page - 1) as u64) * (page_size as u64)
    }

    /// 计算 limit
    pub fn limit(&self) -> u64 {
        let (_, page_size) = self.normalized();
        page_size as u64
    }
}

// =============================================================================
// 便捷构造函数
// =============================================================================

/// 成功响应快捷函数
pub fn api_ok<T>(data: T) -> ApiResponse<T> {
    ApiResponse::ok(data)
}

/// 成功无数据快捷函数
pub fn api_ok_empty() -> ApiResponse<serde_json::Value> {
    ApiResponse::ok_empty()
}

/// 错误响应快捷函数
pub fn api_error<T>(code: i32, message: impl Into<String>) -> ApiResponse<T> {
    ApiResponse::error(code, message)
}

/// 分页成功响应快捷函数
pub fn api_paged<T>(
    items: Vec<T>,
    total: u64,
    page: u32,
    page_size: u32,
) -> ApiResponse<PaginatedResponse<T>> {
    ApiResponse::ok(PaginatedResponse::new(items, total, page, page_size))
}

// =============================================================================
// Axum 集成：IntoResponse
// =============================================================================

#[cfg(feature = "axum-integration")]
impl<T: Serialize> axum::response::IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let status = if self.code == 0 {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::from_u16(self.code as u16)
                .unwrap_or(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        };
        (status, axum::Json(self)).into_response()
    }
}

// =============================================================================
// 统一时间格式工具
// =============================================================================

/// 时间格式：RFC3339（ISO 8601 子集），统一所有 API 的时间字段
pub mod time {
    use chrono::{DateTime, Utc};

    /// 当前时间 RFC3339 字符串
    pub fn now_rfc3339() -> String {
        Utc::now().to_rfc3339()
    }

    /// 当前时间戳（毫秒）
    pub fn now_millis() -> i64 {
        Utc::now().timestamp_millis()
    }

    /// 解析 RFC3339 时间
    pub fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok() {
        let resp = ApiResponse::ok(42);
        assert_eq!(resp.code, 0);
        assert_eq!(resp.message, "ok");
        assert_eq!(resp.data, Some(42));
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<()> = ApiResponse::error(404, "not found");
        assert_eq!(resp.code, 404);
        assert_eq!(resp.message, "not found");
        assert!(resp.data.is_none());
    }

    #[test]
    fn test_api_response_serialization() {
        let resp = ApiResponse::ok(vec![1, 2, 3]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":0"));
        assert!(json.contains("\"message\":\"ok\""));
        assert!(json.contains("\"data\":[1,2,3]"));
    }

    #[test]
    fn test_error_response_skips_data() {
        let resp: ApiResponse<()> = ApiResponse::error(500, "internal");
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("\"data\""));
    }

    #[test]
    fn test_paginated_response() {
        let paged = PaginatedResponse::new(vec!["a", "b"], 25, 1, 10);
        assert_eq!(paged.items.len(), 2);
        assert_eq!(paged.total, 25);
        assert_eq!(paged.page, 1);
        assert_eq!(paged.page_size, 10);
        assert_eq!(paged.total_pages, 3);
    }

    #[test]
    fn test_paginated_empty() {
        let paged: PaginatedResponse<String> = PaginatedResponse::empty(1, 20);
        assert!(paged.items.is_empty());
        assert_eq!(paged.total, 0);
        assert_eq!(paged.total_pages, 0);
    }

    #[test]
    fn test_page_query_normalized() {
        let q = PageQuery { page: 0, page_size: 200 };
        let (p, ps) = q.normalized();
        assert_eq!(p, 1);
        assert_eq!(ps, 100);
    }

    #[test]
    fn test_page_query_offset() {
        let q = PageQuery { page: 3, page_size: 20 };
        assert_eq!(q.offset(), 40);
        assert_eq!(q.limit(), 20);
    }

    #[test]
    fn test_api_ok_helper() {
        let resp = api_ok("test");
        assert_eq!(resp.code, 0);
        assert_eq!(resp.data.unwrap(), "test");
    }

    #[test]
    fn test_api_paged_helper() {
        let resp = api_paged(vec![1, 2], 10, 1, 5);
        assert_eq!(resp.code, 0);
        assert_eq!(resp.data.unwrap().total, 10);
    }

    #[test]
    fn test_from_mox_error() {
        let err = MoxError::not_found(ErrorDomain::Kg, 01, 001, "节点不存在");
        let resp: ApiResponse<()> = ApiResponse::from_mox_error(&err);
        assert_eq!(resp.code, 404);
        assert_eq!(resp.message, "节点不存在");
    }
}
