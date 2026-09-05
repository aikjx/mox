// mox-dsql-core 核心数据结构定义
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

// ==================== 动态业务流程模型 ====================

/// 动态流程状态。
///
/// 流程和 SQL 使用同一套 Draft → Active → Deprecated 发布语义，
/// 使业务配置可以先校验、再发布，避免半成品配置直接进入生产执行路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ProcessStatus {
    Draft,
    Active,
    Deprecated,
}

impl Default for ProcessStatus {
    fn default() -> Self {
        Self::Draft
    }
}

impl ProcessStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Active => "ACTIVE",
            Self::Deprecated => "DEPRECATED",
        }
    }
}

/// 动态流程步骤。
///
/// `input_mapping` 的 key 是 SQL 参数名，value 是流程上下文路径，
/// 例如 `{ "tenant_id": "$.tenant_id", "keyword": "$.payload.keyword" }`。
/// 只允许引用上下文，不允许把上下文拼接进 SQL 文本，保持参数化执行。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStep {
    pub step_code: String,
    pub name: String,
    pub sql_code: String,
    #[serde(default)]
    pub input_mapping: HashMap<String, String>,
    pub output_key: Option<String>,
    /// 简单条件：`$.path == value`、`$.path != value`、`exists($.path)`。
    /// 复杂规则应下沉为一个只读 SQL 定义，避免在数据库中执行任意代码。
    pub when: Option<String>,
    #[serde(default)]
    pub continue_on_error: bool,
    /// 补偿SQL code：当流程事务回滚时，按逆序执行已成功步骤的补偿操作。
    /// 通常用于撤销写操作（如删除已插入的记录、恢复旧值）。
    #[serde(default)]
    pub compensation_sql_code: Option<String>,
}

/// 动态流程定义。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessDefinition {
    pub id: i64,
    pub process_code: String,
    pub process_name: String,
    pub description: Option<String>,
    pub version: i32,
    pub status: ProcessStatus,
    pub steps: Vec<ProcessStep>,
    /// 是否启用事务模式：启用后，任何步骤失败都会按逆序执行已成功步骤的补偿SQL。
    #[serde(default)]
    pub transactional: bool,
    pub permission_code: Option<String>,
    pub entity_code: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 创建动态流程请求。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateProcessRequest {
    pub process_code: String,
    pub process_name: String,
    pub description: Option<String>,
    pub steps: Vec<ProcessStep>,
    pub permission_code: Option<String>,
    pub entity_code: Option<String>,
    pub created_by: Option<String>,
}

/// 动态流程执行请求。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProcessRequest {
    pub process_code: String,
    #[serde(default)]
    pub context: serde_json::Value,
    pub trace_id: Option<String>,
}

/// 单步骤执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStepResult {
    pub step_code: String,
    pub executed: bool,
    pub skipped: bool,
    pub success: bool,
    /// 是否已执行补偿操作
    #[serde(default)]
    pub compensated: bool,
    pub output_key: Option<String>,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 动态流程执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteProcessResult {
    pub process_code: String,
    pub success: bool,
    pub context: serde_json::Value,
    pub steps: Vec<ProcessStepResult>,
    pub duration_ms: u64,
    pub trace_id: Option<String>,
    pub error: Option<String>,
}

/// 审计日志查询参数
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuditLogQuery {
    pub page: i64,
    pub page_size: i64,
    pub sql_code: Option<String>,
    pub datasource_code: Option<String>,
    pub trace_id: Option<String>,
    pub success: Option<bool>,
    pub is_slow: Option<bool>,
    pub cache_hit: Option<bool>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

impl AuditLogQuery {
    pub fn new() -> Self {
        Self { page: 1, page_size: 20, sql_code: None, datasource_code: None, trace_id: None, success: None, is_slow: None, cache_hit: None, start_time: None, end_time: None }
    }
}

/// 审计统计结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStats {
    pub total_count: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub success_rate: f64,
    pub slow_count: i64,
    pub cache_hit_count: i64,
    pub cache_hit_rate: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: i64,
    pub total_row_count: i64,
    pub start_time: String,
    pub end_time: String,
}


