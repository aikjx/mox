//! 批量操作框架
//!
//! 统一的批量操作、导入导出、模板管理工具。
//! 支持：批量创建/更新/删除、CSV/JSON导入导出、模板CRUD与应用

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 批量操作请求
#[derive(Debug, Deserialize)]
pub struct BatchOperationRequest<T> {
    /// 操作类型：create / update / delete / upsert
    pub operation: String,
    /// 操作数据列表
    pub items: Vec<T>,
    /// 失败时是否继续（true=跳过失败项继续，false=遇到失败即停止）
    pub continue_on_error: Option<bool>,
    /// 幂等键字段（用于upsert时判断是否已存在）
    pub id_field: Option<String>,
}

/// 批量操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    /// 总操作数
    pub total: i64,
    /// 成功数
    pub success: i64,
    /// 失败数
    pub failed: i64,
    /// 跳过数
    pub skipped: i64,
    /// 成功的ID列表
    pub success_ids: Vec<String>,
    /// 失败详情
    pub failures: Vec<BatchFailure>,
    /// 操作耗时（毫秒）
    pub duration_ms: i64,
}

/// 批量操作失败详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchFailure {
    /// 索引（在items中的位置）
    pub index: usize,
    /// 失败项的ID（如果有）
    pub item_id: Option<String>,
    /// 错误信息
    pub error: String,
    /// 错误代码
    pub error_code: Option<String>,
}

impl Default for BatchOperationResult {
    fn default() -> Self {
        Self {
            total: 0,
            success: 0,
            failed: 0,
            skipped: 0,
            success_ids: Vec::new(),
            failures: Vec::new(),
            duration_ms: 0,
        }
    }
}

/// 导入请求
#[derive(Debug, Deserialize)]
pub struct ImportRequest {
    /// 导入格式：csv / json / excel
    pub format: String,
    /// 导入模式：create / update / upsert
    pub mode: String,
    /// 数据内容（CSV文本或JSON数组）
    pub data: String,
    /// 字段映射（源字段名 -> 目标字段名）
    pub field_mapping: Option<HashMap<String, String>>,
    /// 是否跳过第一行（CSV表头）
    pub skip_header: Option<bool>,
    /// 失败时是否继续
    pub continue_on_error: Option<bool>,
    /// 模板ID（使用模板的字段映射和验证规则）
    pub template_id: Option<String>,
}

/// 导出请求
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    /// 导出格式：csv / json / excel
    pub format: String,
    /// 导出的字段列表（空=全部字段）
    pub fields: Option<Vec<String>>,
    /// 筛选条件
    pub filters: Option<HashMap<String, String>>,
    /// 排序字段
    pub sort_by: Option<String>,
    /// 排序方向：asc / desc
    pub sort_order: Option<String>,
    /// 导出数量限制（0=全部）
    pub limit: Option<i64>,
    /// 是否包含表头（CSV）
    pub include_header: Option<bool>,
    /// 模板ID（使用模板的字段和格式）
    pub template_id: Option<String>,
}

/// 导出结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// 导出格式
    pub format: String,
    /// 导出数据（CSV文本或JSON字符串）
    pub data: String,
    /// 导出记录数
    pub count: i64,
    /// 文件名
    pub filename: String,
    /// MIME类型
    pub content_type: String,
}

/// 导入导出模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExportTemplate {
    /// 模板ID
    pub template_id: String,
    /// 模板名称
    pub template_name: String,
    /// 模板类型：import / export / both
    pub template_type: String,
    /// 适用模块（如 user / department / dictionary）
    pub module: String,
    /// 模板描述
    pub description: Option<String>,
    /// 字段映射（源字段 -> 目标字段）
    pub field_mapping: HashMap<String, String>,
    /// 默认值（字段名 -> 默认值）
    pub default_values: HashMap<String, String>,
    /// 验证规则（字段名 -> 规则描述）
    pub validation_rules: HashMap<String, String>,
    /// 导出字段列表
    pub export_fields: Option<Vec<String>>,
    /// 是否系统内置
    pub is_system: bool,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 模板管理状态
pub struct TemplateState {
    /// 模板表
    pub templates: Arc<RwLock<HashMap<String, ImportExportTemplate>>>,
}

impl TemplateState {
    pub fn new() -> Self {
        Self {
            templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for TemplateState {
    fn default() -> Self {
        Self::new()
    }
}

// ==================== CSV 解析工具 ====================

/// 解析CSV文本为Vec<HashMap<String, String>>
pub fn parse_csv(csv_text: &str, skip_header: bool) -> Result<Vec<HashMap<String, String>>, String> {
    let lines: Vec<&str> = csv_text.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let start_idx = if skip_header { 1 } else { 0 };
    let headers: Vec<String> = if skip_header {
        parse_csv_line(lines[0])
    } else {
        (0..parse_csv_line(lines[0]).len()).map(|i| format!("col_{}", i)).collect()
    };

    let mut result = Vec::new();
    for (i, line) in lines.iter().enumerate().skip(start_idx) {
        if line.trim().is_empty() {
            continue;
        }
        let values = parse_csv_line(line);
        let mut row = HashMap::new();
        for (j, header) in headers.iter().enumerate() {
            let value = values.get(j).cloned().unwrap_or_default();
            row.insert(header.clone(), value);
        }
        result.push(row);
    }

    Ok(result)
}

/// 解析单行CSV（支持引号包裹的字段）
fn parse_csv_line(line: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        current.push('"');
                        chars.next();
                    } else {
                        in_quotes = false;
                    }
                } else {
                    in_quotes = true;
                }
            }
            ',' if !in_quotes => {
                result.push(current.clone());
                current.clear();
            }
            _ => {
                current.push(c);
            }
        }
    }
    result.push(current);
    result
}

/// 将Vec<HashMap<String, String>>序列化为CSV文本
pub fn to_csv(rows: &[HashMap<String, String>], fields: &[String], include_header: bool) -> String {
    let mut result = String::new();

    if include_header {
        result.push_str(&fields.join(","));
        result.push('\n');
    }

    for row in rows {
        let values: Vec<String> = fields.iter()
            .map(|f| {
                let val = row.get(f).cloned().unwrap_or_default();
                if val.contains(',') || val.contains('"') || val.contains('\n') {
                    format!("\"{}\"", val.replace('"', "\"\""))
                } else {
                    val
                }
            })
            .collect();
        result.push_str(&values.join(","));
        result.push('\n');
    }

    result
}

// ==================== 字段映射工具 ====================

/// 应用字段映射
pub fn apply_field_mapping(
    row: &HashMap<String, String>,
    mapping: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut result = HashMap::new();
    for (key, value) in row {
        let target_key = mapping.get(key).cloned().unwrap_or_else(|| key.clone());
        result.insert(target_key, value.clone());
    }
    result
}

/// 应用默认值
pub fn apply_default_values(
    row: &mut HashMap<String, String>,
    defaults: &HashMap<String, String>,
) {
    for (key, value) in defaults {
        if !row.contains_key(key) || row.get(key).map(|v| v.is_empty()).unwrap_or(true) {
            row.insert(key.clone(), value.clone());
        }
    }
}

// ==================== 内置模板 ====================

/// 获取内置模板列表
pub fn builtin_templates() -> Vec<ImportExportTemplate> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        ImportExportTemplate {
            template_id: "tpl_user_import".to_string(),
            template_name: "用户导入模板".to_string(),
            template_type: "import".to_string(),
            module: "user".to_string(),
            description: Some("批量导入用户的标准模板".to_string()),
            field_mapping: {
                let mut m = HashMap::new();
                m.insert("用户名".to_string(), "username".to_string());
                m.insert("姓名".to_string(), "full_name".to_string());
                m.insert("邮箱".to_string(), "email".to_string());
                m.insert("手机号".to_string(), "phone".to_string());
                m.insert("部门".to_string(), "department_id".to_string());
                m.insert("角色".to_string(), "role_ids".to_string());
                m
            },
            default_values: {
                let mut m = HashMap::new();
                m.insert("status".to_string(), "active".to_string());
                m
            },
            validation_rules: {
                let mut m = HashMap::new();
                m.insert("username".to_string(), "required,unique".to_string());
                m.insert("email".to_string(), "email".to_string());
                m
            },
            export_fields: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ImportExportTemplate {
            template_id: "tpl_department_import".to_string(),
            template_name: "部门导入模板".to_string(),
            template_type: "import".to_string(),
            module: "department".to_string(),
            description: Some("批量导入部门的标准模板".to_string()),
            field_mapping: {
                let mut m = HashMap::new();
                m.insert("部门名称".to_string(), "dept_name".to_string());
                m.insert("上级部门".to_string(), "parent_id".to_string());
                m.insert("部门编码".to_string(), "dept_code".to_string());
                m.insert("负责人".to_string(), "leader_id".to_string());
                m.insert("排序".to_string(), "sort_order".to_string());
                m
            },
            default_values: HashMap::new(),
            validation_rules: {
                let mut m = HashMap::new();
                m.insert("dept_name".to_string(), "required".to_string());
                m
            },
            export_fields: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ImportExportTemplate {
            template_id: "tpl_dictionary_import".to_string(),
            template_name: "字典项导入模板".to_string(),
            template_type: "import".to_string(),
            module: "dictionary".to_string(),
            description: Some("批量导入字典项的标准模板".to_string()),
            field_mapping: {
                let mut m = HashMap::new();
                m.insert("字典类型".to_string(), "dict_type".to_string());
                m.insert("字典标签".to_string(), "dict_label".to_string());
                m.insert("字典值".to_string(), "dict_value".to_string());
                m.insert("排序".to_string(), "sort_order".to_string());
                m.insert("备注".to_string(), "remark".to_string());
                m
            },
            default_values: {
                let mut m = HashMap::new();
                m.insert("status".to_string(), "enabled".to_string());
                m
            },
            validation_rules: {
                let mut m = HashMap::new();
                m.insert("dict_type".to_string(), "required".to_string());
                m.insert("dict_label".to_string(), "required".to_string());
                m.insert("dict_value".to_string(), "required".to_string());
                m
            },
            export_fields: None,
            is_system: true,
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}
