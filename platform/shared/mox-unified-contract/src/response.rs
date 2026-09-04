// =============================================================================
// 统一响应信封（ApiResponse / ApiSuccess / ApiError / PagedResponse）
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的响应 JSON 结构。
// =============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// 统一响应信封
// =============================================================================

/// API 统一响应信封
///
/// 所有 API 响应必须使用此格式，禁止自定义响应结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 状态码（0 = 成功，非0 = 错误）
    pub code: i32,
    /// 消息（成功时为 "success"，错误时为中文错误消息）
    pub msg: String,
    /// 业务数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 追踪 ID（全链路透传）
    pub trace_id: String,
    /// 服务端处理耗时（毫秒）
    #[serde(default)]
    pub latency_ms: u64,
    /// 时间戳（ISO-8601）
    pub timestamp: String,
}

impl<T> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 0,
            msg: "success".to_string(),
            data: Some(data),
            trace_id: Uuid::new_v4().to_string(),
            latency_ms: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 成功响应（无数据）
    pub fn success_empty() -> Self
    where
        T: Default,
    {
        Self {
            code: 0,
            msg: "success".to_string(),
            data: None,
            trace_id: Uuid::new_v4().to_string(),
            latency_ms: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 错误响应
    pub fn error(code: i32, msg: impl Into<String>) -> Self {
        Self {
            code,
            msg: msg.into(),
            data: None,
            trace_id: Uuid::new_v4().to_string(),
            latency_ms: 0,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// 设置追踪 ID
    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = trace_id.into();
        self
    }

    /// 设置耗时
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// 是否错误
    pub fn is_error(&self) -> bool {
        self.code != 0
    }
}

// =============================================================================
// 成功响应（简化版）
// =============================================================================

/// 成功响应（简化版，用于快速构造）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSuccess<T> {
    /// 业务数据
    pub data: T,
    /// 追踪 ID
    pub trace_id: String,
}

impl<T> ApiSuccess<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            trace_id: Uuid::new_v4().to_string(),
        }
    }
}

// =============================================================================
// 错误响应（简化版）
// =============================================================================

/// 错误响应（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// 错误码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// 追踪 ID
    pub trace_id: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            detail: None,
            trace_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

// =============================================================================
// 分页响应
// =============================================================================

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagedResponse<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 分页信息
    pub pagination: PaginationInfo,
}

/// 分页信息
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaginationInfo {
    /// 当前页码（从1开始）
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
    /// 总记录数
    pub total: u64,
    /// 总页数
    pub total_pages: u32,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

impl PaginationInfo {
    /// 创建分页信息
    pub fn new(page: u32, page_size: u32, total: u64) -> Self {
        let total_pages = if page_size == 0 {
            0
        } else {
            ((total as f64) / (page_size as f64)).ceil() as u32
        };
        Self {
            page,
            page_size,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }
}

impl<T> PagedResponse<T> {
    /// 创建分页响应
    pub fn new(items: Vec<T>, page: u32, page_size: u32, total: u64) -> Self {
        Self {
            items,
            pagination: PaginationInfo::new(page, page_size, total),
        }
    }

    /// 空分页响应
    pub fn empty(page: u32, page_size: u32) -> Self {
        Self::new(vec![], page, page_size, 0)
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_response_success() {
        let resp = ApiResponse::success(vec![1, 2, 3]);
        assert_eq!(resp.code, 0);
        assert_eq!(resp.msg, "success");
        assert_eq!(resp.data, Some(vec![1, 2, 3]));
        assert!(resp.is_success());
    }

    #[test]
    fn api_response_error() {
        let resp: ApiResponse<()> = ApiResponse::error(40001, "参数错误");
        assert_eq!(resp.code, 40001);
        assert_eq!(resp.msg, "参数错误");
        assert!(resp.data.is_none());
        assert!(resp.is_error());
    }

    #[test]
    fn api_response_with_trace_and_latency() {
        let resp = ApiResponse::success("data")
            .with_trace_id("trace-123")
            .with_latency(150);
        assert_eq!(resp.trace_id, "trace-123");
        assert_eq!(resp.latency_ms, 150);
    }

    #[test]
    fn pagination_info_calculation() {
        let info = PaginationInfo::new(1, 10, 25);
        assert_eq!(info.page, 1);
        assert_eq!(info.page_size, 10);
        assert_eq!(info.total, 25);
        assert_eq!(info.total_pages, 3);
        assert!(info.has_next);
        assert!(!info.has_prev);

        let info_last = PaginationInfo::new(3, 10, 25);
        assert!(!info_last.has_next);
        assert!(info_last.has_prev);
    }

    #[test]
    fn paged_response_empty() {
        let resp: PagedResponse<i32> = PagedResponse::empty(1, 10);
        assert!(resp.items.is_empty());
        assert_eq!(resp.pagination.total, 0);
        assert_eq!(resp.pagination.total_pages, 0);
    }

    #[test]
    fn api_error_creation() {
        let err = ApiError::new("AI01001", "会话不存在")
            .with_detail("session_id=xxx not found");
        assert_eq!(err.code, "AI01001");
        assert_eq!(err.message, "会话不存在");
        assert_eq!(err.detail, Some("session_id=xxx not found".to_string()));
    }

    #[test]
    fn api_response_serialization() {
        let resp = ApiResponse::success(vec!["a", "b"]);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ApiResponse<Vec<String>> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, 0);
        assert_eq!(parsed.data, Some(vec!["a".to_string(), "b".to_string()]));
    }
}
