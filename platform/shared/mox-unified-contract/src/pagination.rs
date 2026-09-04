// =============================================================================
// 分页/排序/过滤的统一请求格式
// =============================================================================
// 跨端对齐：Python 和 前端必须使用相同的分页请求参数。
// =============================================================================

use serde::{Deserialize, Serialize};

// =============================================================================
// 分页请求
// =============================================================================

/// 分页请求参数
///
/// 所有列表查询接口必须使用此参数，禁止自定义分页参数。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaginationRequest {
    /// 页码（从1开始，默认1）
    #[serde(default = "default_page")]
    pub page: u32,
    /// 每页大小（默认20，最大100）
    #[serde(default = "default_page_size")]
    pub page_size: u32,
}

fn default_page() -> u32 {
    1
}

fn default_page_size() -> u32 {
    20
}

impl Default for PaginationRequest {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
        }
    }
}

impl PaginationRequest {
    /// 创建分页请求
    pub fn new(page: u32, page_size: u32) -> Self {
        Self {
            page: page.max(1),
            page_size: page_size.clamp(1, 100),
        }
    }

    /// 计算 SQL OFFSET
    pub fn offset(&self) -> u64 {
        ((self.page - 1) as u64) * (self.page_size as u64)
    }

    /// 计算 SQL LIMIT
    pub fn limit(&self) -> u64 {
        self.page_size as u64
    }

    /// 验证参数
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page 必须从 1 开始".to_string());
        }
        if self.page_size == 0 || self.page_size > 100 {
            return Err("page_size 必须在 1-100 之间".to_string());
        }
        Ok(())
    }
}

// =============================================================================
// 排序
// =============================================================================

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Desc
    }
}

impl SortOrder {
    /// SQL 排序关键字
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        }
    }
}

impl std::fmt::Display for SortOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "asc"),
            SortOrder::Desc => write!(f, "desc"),
        }
    }
}

/// 排序条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortCondition {
    /// 排序字段
    pub field: String,
    /// 排序方向
    #[serde(default)]
    pub order: SortOrder,
}

impl SortCondition {
    pub fn new(field: impl Into<String>, order: SortOrder) -> Self {
        Self {
            field: field.into(),
            order,
        }
    }
}

// =============================================================================
// 过滤
// =============================================================================

/// 过滤操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 包含（字符串模糊匹配）
    Contains,
    /// 以...开头
    StartsWith,
    /// 以...结尾
    EndsWith,
    /// 在列表中
    In,
    /// 不在列表中
    NotIn,
    /// 为空
    IsNull,
    /// 不为空
    IsNotNull,
    /// 区间
    Between,
}

impl FilterOperator {
    /// SQL 操作符
    pub fn as_sql(&self) -> &'static str {
        match self {
            FilterOperator::Eq => "=",
            FilterOperator::Ne => "!=",
            FilterOperator::Gt => ">",
            FilterOperator::Gte => ">=",
            FilterOperator::Lt => "<",
            FilterOperator::Lte => "<=",
            FilterOperator::Contains => "LIKE",
            FilterOperator::StartsWith => "LIKE",
            FilterOperator::EndsWith => "LIKE",
            FilterOperator::In => "IN",
            FilterOperator::NotIn => "NOT IN",
            FilterOperator::IsNull => "IS NULL",
            FilterOperator::IsNotNull => "IS NOT NULL",
            FilterOperator::Between => "BETWEEN",
        }
    }
}

/// 过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    /// 过滤字段
    pub field: String,
    /// 操作符
    pub operator: FilterOperator,
    /// 过滤值
    #[serde(default)]
    pub value: serde_json::Value,
}

impl FilterCondition {
    pub fn new(field: impl Into<String>, operator: FilterOperator, value: serde_json::Value) -> Self {
        Self {
            field: field.into(),
            operator,
            value,
        }
    }

    /// 等于条件
    pub fn eq(field: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        Self::new(field, FilterOperator::Eq, value.into())
    }

    /// 包含条件
    pub fn contains(field: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(field, FilterOperator::Contains, serde_json::Value::String(value.into()))
    }
}

// =============================================================================
// 统一查询请求（分页 + 排序 + 过滤）
// =============================================================================

/// 统一查询请求
///
/// 所有列表查询接口应使用此结构作为请求体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// 分页
    #[serde(default)]
    pub pagination: PaginationRequest,
    /// 排序条件列表
    #[serde(default)]
    pub sorts: Vec<SortCondition>,
    /// 过滤条件列表（AND 关系）
    #[serde(default)]
    pub filters: Vec<FilterCondition>,
    /// 搜索关键字（全局搜索）
    #[serde(default)]
    pub keyword: Option<String>,
}

impl Default for QueryRequest {
    fn default() -> Self {
        Self {
            pagination: PaginationRequest::default(),
            sorts: vec![],
            filters: vec![],
            keyword: None,
        }
    }
}

impl QueryRequest {
    /// 创建简单查询
    pub fn simple(page: u32, page_size: u32) -> Self {
        Self {
            pagination: PaginationRequest::new(page, page_size),
            ..Default::default()
        }
    }

    /// 添加排序
    pub fn with_sort(mut self, field: impl Into<String>, order: SortOrder) -> Self {
        self.sorts.push(SortCondition::new(field, order));
        self
    }

    /// 添加过滤
    pub fn with_filter(mut self, filter: FilterCondition) -> Self {
        self.filters.push(filter);
        self
    }

    /// 设置关键字
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keyword = Some(keyword.into());
        self
    }
}

// =============================================================================
// 测试
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_default() {
        let p = PaginationRequest::default();
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 20);
    }

    #[test]
    fn pagination_offset_limit() {
        let p = PaginationRequest::new(3, 20);
        assert_eq!(p.offset(), 40);
        assert_eq!(p.limit(), 20);
    }

    #[test]
    fn pagination_clamp() {
        let p = PaginationRequest::new(0, 200);
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 100);
    }

    #[test]
    fn pagination_validate() {
        assert!(PaginationRequest::new(1, 20).validate().is_ok());
        // new() 会 clamp，所以直接构造未 clamp 的值测试
        assert!(PaginationRequest { page: 1, page_size: 0 }.validate().is_err());
        assert!(PaginationRequest { page: 1, page_size: 101 }.validate().is_err());
    }

    #[test]
    fn sort_order_sql() {
        assert_eq!(SortOrder::Asc.as_sql(), "ASC");
        assert_eq!(SortOrder::Desc.as_sql(), "DESC");
    }

    #[test]
    fn filter_operator_sql() {
        assert_eq!(FilterOperator::Eq.as_sql(), "=");
        assert_eq!(FilterOperator::Gt.as_sql(), ">");
        assert_eq!(FilterOperator::Contains.as_sql(), "LIKE");
        assert_eq!(FilterOperator::In.as_sql(), "IN");
    }

    #[test]
    fn query_request_builder() {
        let query = QueryRequest::simple(1, 20)
            .with_sort("created_at", SortOrder::Desc)
            .with_filter(FilterCondition::eq("status", "active"))
            .with_keyword("rust");

        assert_eq!(query.pagination.page, 1);
        assert_eq!(query.sorts.len(), 1);
        assert_eq!(query.filters.len(), 1);
        assert_eq!(query.keyword, Some("rust".to_string()));
    }

    #[test]
    fn pagination_request_deserialization() {
        let json = r#"{"page": 2, "page_size": 50}"#;
        let p: PaginationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(p.page, 2);
        assert_eq!(p.page_size, 50);
    }

    #[test]
    fn pagination_request_default_deserialization() {
        let json = r#"{}"#;
        let p: PaginationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(p.page, 1);
        assert_eq!(p.page_size, 20);
    }
}
