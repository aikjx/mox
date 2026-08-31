//! 通用工具函数与类型

use serde::{Deserialize, Serialize};

/// 分页请求
#[derive(Debug, Clone, Deserialize)]
pub struct PageQuery {
    /// 页码，从 1 开始
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self { page: 1, page_size: 20 }
    }
}

impl PageQuery {
    /// 计算偏移量
    pub fn offset(&self) -> u64 {
        (self.page.max(1) - 1) * self.page_size.min(100)
    }

    /// 限制最大 page_size
    pub fn limit(&self) -> u64 {
        self.page_size.min(100)
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize)]
pub struct PageResult<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 总数
    pub total: u64,
    /// 当前页
    pub page: u64,
    /// 每页大小
    pub page_size: u64,
    /// 总页数
    pub total_pages: u64,
}

impl<T> PageResult<T> {
    /// 创建分页结果
    pub fn new(items: Vec<T>, total: u64, page: u64, page_size: u64) -> Self {
        let total_pages = if page_size == 0 { 0 } else { (total + page_size - 1) / page_size };
        Self { items, total, page, page_size, total_pages }
    }
}

/// 统一 API 响应
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    /// 错误码，0 表示成功
    pub code: i32,
    /// 消息
    pub message: String,
    /// 数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 请求 ID
    pub request_id: String,
}

impl<T> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T, request_id: impl Into<String>) -> Self {
        Self {
            code: 0,
            message: "success".into(),
            data: Some(data),
            request_id: request_id.into(),
        }
    }

    /// 错误响应
    pub fn error(code: i32, message: impl Into<String>, request_id: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
            request_id: request_id.into(),
        }
    }
}
