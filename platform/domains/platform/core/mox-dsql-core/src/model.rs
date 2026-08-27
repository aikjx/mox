// mox-dsql-core 核心数据结构定义
use serde::{Deserialize, Serialize};

/// SQL操作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum OperationType {
    Read,
    Write,
}

impl Default for OperationType {
    fn default() -> Self {
        Self::Read
    }
}

/// 结果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ResultType {
    Map,    // 单行Map
    List,   // 多行List<Map>
    Single, // 单值
    Count,  // 计数
    Update, // 更新行数
}

impl Default for ResultType {
    fn default() -> Self {
        Self::List
    }
}

/// SQL状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SqlStatus {
    Draft,
    Active,
    Deprecated,
}

impl Default for SqlStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl SqlStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Active => "ACTIVE",
            Self::Deprecated => "DEPRECATED",
        }
    }
}

/// 数据源类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Datasource {
    pub id: i64,
    pub datasource_code: String,
    pub name: String,
    pub db_type: String,
    pub connection_str: String,
    pub username: Option<String>,
    pub password_enc: Option<String>,
    pub pool_max_size: i32,
    pub pool_min_size: i32,
    pub status: String,
}

/// 参数定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub data_type: String,      // STRING/INT/LONG/DECIMAL/DATETIME/BOOL
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub validation: Option<ParamValidation>,
}

/// 参数校验规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamValidation {
    #[serde(rename = "type")]
    pub rule_type: String, // regex/range/enum/not_empty
    pub pattern: Option<String>,
    pub min: Option<serde_json::Value>,
    pub max: Option<serde_json::Value>,
    pub enum_values: Option<Vec<String>>,
}

/// SQL定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlDefinition {
    pub id: i64,
    pub sql_code: String,
    pub sql_name: String,
    pub description: Option<String>,
    pub datasource_code: String,
    pub sql_template: String,
    pub param_defs: Vec<ParamDef>,
    pub result_type: ResultType,
    pub operation_type: OperationType,
    pub cache_enabled: bool,
    pub cache_ttl: i32,
    pub permission_code: Option<String>,
    pub entity_code: Option<String>,
    pub status: SqlStatus,
    pub version: i32,
    pub version_hash: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// SQL创建请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateSqlRequest {
    pub sql_code: String,
    pub sql_name: String,
    pub description: Option<String>,
    pub datasource_code: String,
    pub sql_template: String,
    pub param_defs: Vec<ParamDef>,
    pub result_type: ResultType,
    pub operation_type: OperationType,
    pub cache_enabled: Option<bool>,
    pub cache_ttl: Option<i32>,
    pub permission_code: Option<String>,
    pub entity_code: Option<String>,
    pub created_by: Option<String>,
}

/// SQL更新请求
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateSqlRequest {
    pub sql_name: Option<String>,
    pub description: Option<String>,
    pub datasource_code: Option<String>,
    pub sql_template: Option<String>,
    pub param_defs: Option<Vec<ParamDef>>,
    pub result_type: Option<ResultType>,
    pub operation_type: Option<OperationType>,
    pub cache_enabled: Option<bool>,
    pub cache_ttl: Option<i32>,
    pub permission_code: Option<String>,
    pub entity_code: Option<String>,
    pub status: Option<SqlStatus>,
    pub change_note: Option<String>,
}

/// 执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteRequest {
    pub sql_code: String,
    pub params: serde_json::Value, // Object: {param_name: value}
    pub trace_id: Option<String>,
}

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub sql_code: String,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub row_count: Option<i64>,
    pub duration_ms: u64,
    pub cache_hit: bool,
    pub error: Option<String>,
    pub trace_id: Option<String>,
}

/// 审计日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub trace_id: Option<String>,
    pub sql_code: String,
    pub datasource_code: Option<String>,
    pub params: Option<String>,
    pub row_count: Option<i64>,
    pub duration_ms: Option<i64>,
    pub success: bool,
    pub error_msg: Option<String>,
    pub is_slow: bool,
    pub cache_hit: bool,
    pub created_at: String,
}

/// 分页查询参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageQuery {
    pub page: i64,
    pub page_size: i64,
    pub keyword: Option<String>,
    pub status: Option<String>,
    pub entity_code: Option<String>,
    pub datasource_code: Option<String>,
}

impl Default for PageQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 20,
            keyword: None,
            status: None,
            entity_code: None,
            datasource_code: None,
        }
    }
}

/// 分页结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResult<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}
