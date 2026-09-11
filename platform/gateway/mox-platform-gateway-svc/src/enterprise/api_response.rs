//! 企业级 API 统一响应工具模块
//!
//! 提供统一的成功/错误响应格式、分页支持、错误码定义
//! 所有企业级功能模块必须使用本模块的响应函数，确保格式归一化

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

/// 统一响应码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiCode {
    /// 成功
    Success = 0,
    /// 参数错误
    BadRequest = 400,
    /// 未授权
    Unauthorized = 401,
    /// 禁止访问
    Forbidden = 403,
    /// 资源不存在
    NotFound = 404,
    /// 方法不允许
    MethodNotAllowed = 405,
    /// 冲突
    Conflict = 409,
    /// 请求过多
    TooManyRequests = 429,
    /// 服务器内部错误
    InternalError = 500,
    /// 服务不可用
    ServiceUnavailable = 503,
}

impl ApiCode {
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    pub fn to_status_code(self) -> StatusCode {
        StatusCode::from_u16(self.as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// 分页参数
#[derive(Debug, Clone, Serialize)]
pub struct Pagination {
    /// 当前页码（从1开始）
    pub page: usize,
    /// 每页大小
    pub page_size: usize,
    /// 总记录数
    pub total: usize,
    /// 总页数
    pub total_pages: usize,
    /// 是否有下一页
    pub has_next: bool,
    /// 是否有上一页
    pub has_prev: bool,
}

impl Pagination {
    pub fn new(page: usize, page_size: usize, total: usize) -> Self {
        let page = if page == 0 { 1 } else { page };
        let page_size = if page_size == 0 { 20 } else { page_size.min(100) };
        let total_pages = if total == 0 { 1 } else { (total + page_size - 1) / page_size };
        Self {
            page,
            page_size,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        }
    }

    /// 计算偏移量
    pub fn offset(&self) -> usize {
        (self.page - 1) * self.page_size
    }

    /// 对列表进行分页
    pub fn paginate<T: Clone>(&self, items: &[T]) -> Vec<T> {
        let start = self.offset().min(items.len());
        let end = (start + self.page_size).min(items.len());
        items[start..end].to_vec()
    }
}

/// 成功响应（带数据）
pub fn success<T: Serialize>(data: T) -> Response {
    Json(json!({
        "code": ApiCode::Success.as_u16(),
        "message": "success",
        "data": data,
    })).into_response()
}

/// 成功响应（带数据和消息）
pub fn success_with_message<T: Serialize>(message: &str, data: T) -> Response {
    Json(json!({
        "code": ApiCode::Success.as_u16(),
        "message": message,
        "data": data,
    })).into_response()
}

/// 成功响应（仅消息，无数据）
pub fn success_message(message: &str) -> Response {
    Json(json!({
        "code": ApiCode::Success.as_u16(),
        "message": message,
    })).into_response()
}

/// 列表成功响应（带分页）
pub fn success_list<T: Serialize>(items: T, pagination: &Pagination) -> Response {
    Json(json!({
        "code": ApiCode::Success.as_u16(),
        "message": "success",
        "data": items,
        "pagination": {
            "page": pagination.page,
            "page_size": pagination.page_size,
            "total": pagination.total,
            "total_pages": pagination.total_pages,
            "has_next": pagination.has_next,
            "has_prev": pagination.has_prev,
        },
    })).into_response()
}

/// 错误响应
pub fn error(code: ApiCode, message: &str) -> Response {
    let status = code.to_status_code();
    (status, Json(json!({
        "code": code.as_u16(),
        "message": message,
    }))).into_response()
}

/// 错误响应（带详情）
pub fn error_with_details(code: ApiCode, message: &str, details: serde_json::Value) -> Response {
    let status = code.to_status_code();
    (status, Json(json!({
        "code": code.as_u16(),
        "message": message,
        "details": details,
    }))).into_response()
}

/// 400 参数错误
pub fn bad_request(message: &str) -> Response {
    error(ApiCode::BadRequest, message)
}

/// 401 未授权
pub fn unauthorized(message: &str) -> Response {
    error(ApiCode::Unauthorized, message)
}

/// 403 禁止访问
pub fn forbidden(message: &str) -> Response {
    error(ApiCode::Forbidden, message)
}

/// 404 资源不存在
pub fn not_found(message: &str) -> Response {
    error(ApiCode::NotFound, message)
}

/// 409 冲突
pub fn conflict(message: &str) -> Response {
    error(ApiCode::Conflict, message)
}

/// 500 服务器内部错误
pub fn internal_error(message: &str) -> Response {
    error(ApiCode::InternalError, message)
}

/// 验证字段是否为空
pub fn validate_required(field: &str, value: &str) -> Result<(), Response> {
    if value.trim().is_empty() {
        return Err(bad_request(&format!("字段 '{}' 不能为空", field)));
    }
    Ok(())
}

/// 验证字段长度
pub fn validate_length(field: &str, value: &str, min: usize, max: usize) -> Result<(), Response> {
    let len = value.chars().count();
    if len < min {
        return Err(bad_request(&format!("字段 '{}' 长度不能小于 {}（当前 {}）", field, min, len)));
    }
    if len > max {
        return Err(bad_request(&format!("字段 '{}' 长度不能大于 {}（当前 {}）", field, max, len)));
    }
    Ok(())
}

/// 验证枚举值是否合法
pub fn validate_enum(field: &str, value: &str, valid_values: &[&str]) -> Result<(), Response> {
    if !valid_values.contains(&value) {
        return Err(bad_request(&format!(
            "字段 '{}' 值 '{}' 不合法，允许的值：{}",
            field, value, valid_values.join(", ")
        )));
    }
    Ok(())
}

/// 从查询参数中解析分页参数
pub fn parse_pagination(params: &std::collections::HashMap<String, String>) -> (usize, usize) {
    let page = params.get("page")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1);
    let page_size = params.get("page_size")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(20);
    (page, page_size)
}

/// 从查询参数中解析排序参数
pub fn parse_sort(params: &std::collections::HashMap<String, String>) -> Option<(String, bool)> {
    params.get("sort_by").map(|field| {
        let order = params.get("sort_order")
            .map(|v| v.eq_ignore_ascii_case("desc"))
            .unwrap_or(false);
        (field.clone(), order)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_new() {
        let p = Pagination::new(1, 20, 100);
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 20);
        assert_eq!(p.total, 100);
        assert_eq!(p.total_pages, 5);
        assert!(p.has_next);
        assert!(!p.has_prev);
    }

    #[test]
    fn test_pagination_offset() {
        let p = Pagination::new(3, 20, 100);
        assert_eq!(p.offset(), 40);
    }

    #[test]
    fn test_pagination_paginate() {
        let items: Vec<i32> = (1..=100).collect();
        let p = Pagination::new(2, 10, 100);
        let page = p.paginate(&items);
        assert_eq!(page.len(), 10);
        assert_eq!(page[0], 11);
        assert_eq!(page[9], 20);
    }

    #[test]
    fn test_validate_required() {
        assert!(validate_required("name", "test").is_ok());
        assert!(validate_required("name", "").is_err());
        assert!(validate_required("name", "  ").is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("name", "test", 1, 10).is_ok());
        assert!(validate_length("name", "", 1, 10).is_err());
        assert!(validate_length("name", "12345678901", 1, 10).is_err());
    }

    #[test]
    fn test_validate_enum() {
        assert!(validate_enum("status", "active", &["active", "inactive"]).is_ok());
        assert!(validate_enum("status", "unknown", &["active", "inactive"]).is_err());
    }
}
