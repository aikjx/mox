//! 查询过滤、排序与分页结果

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 过滤条件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field_code: String,
    pub operator: String,
    pub value: Value,
}

impl Filter {
    /// 转换为 SQL WHERE 子句片段（参数化，返回 (sql片段, 参数值)）
    pub fn to_sql(&self, slot_column: &str) -> (String, Option<String>) {
        match self.operator.to_lowercase().as_str() {
            "eq" => (format!("{} = ?", slot_column), Some(self.value_as_string())),
            "ne" => (format!("{} != ?", slot_column), Some(self.value_as_string())),
            "gt" => (format!("{} > ?", slot_column), Some(self.value_as_string())),
            "gte" => (format!("{} >= ?", slot_column), Some(self.value_as_string())),
            "lt" => (format!("{} < ?", slot_column), Some(self.value_as_string())),
            "lte" => (format!("{} <= ?", slot_column), Some(self.value_as_string())),
            "like" => (format!("{} LIKE ?", slot_column), Some(format!("%{}%", self.value_as_string()))),
            "in" => {
                if let Value::Array(arr) = &self.value {
                    let placeholders: Vec<String> = arr.iter().map(|_| "?".to_string()).collect();
                    (format!("{} IN ({})", slot_column, placeholders.join(",")), None)
                } else {
                    (format!("{} = ?", slot_column), Some(self.value_as_string()))
                }
            }
            "is_null" => (format!("{} IS NULL", slot_column), None),
            "is_not_null" => (format!("{} IS NOT NULL", slot_column), None),
            _ => (format!("{} = ?", slot_column), Some(self.value_as_string())),
        }
    }

    fn value_as_string(&self) -> String {
        match &self.value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            other => other.to_string(),
        }
    }
}

/// 排序规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortSpec {
    pub field: Option<String>,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for SortSpec {
    fn default() -> Self {
        Self { field: None, order: SortOrder::Desc }
    }
}

impl SortSpec {
    /// 转换为 SQL ORDER BY 子句
    pub fn to_sql(&self, slot_column: Option<&str>) -> String {
        match (&self.field, slot_column) {
            (Some(_f), Some(col)) => format!("ORDER BY {} {}", col, self.order.as_sql()),
            _ => "ORDER BY created_at DESC".to_string(),
        }
    }
}

impl SortOrder {
    pub fn as_sql(&self) -> &'static str {
        match self { SortOrder::Asc => "ASC", SortOrder::Desc => "DESC" }
    }
}

/// 分页列表结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResult {
    pub total: i64,
    pub items: Vec<Value>,
    pub page: i64,
    pub page_size: i64,
}
