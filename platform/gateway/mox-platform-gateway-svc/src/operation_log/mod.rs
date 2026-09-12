//! 操作日志查询模块
//!
//! 提供：操作日志查询 / 日志详情 / 日志统计 / 日志导出 / 登录日志

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 操作日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    /// 日志ID
    pub log_id: String,
    /// 租户ID
    pub tenant_id: Option<String>,
    /// 用户ID
    pub user_id: String,
    /// 用户名
    pub username: String,
    /// 操作类型
    pub operation_type: String,
    /// 操作模块
    pub module: String,
    /// 操作描述
    pub description: String,
    /// 请求方法
    pub method: Option<String>,
    /// 请求路径
    pub path: Option<String>,
    /// 请求参数
    pub params: Option<String>,
    /// 操作结果（success/failure）
    pub result: String,
    /// 错误信息
    pub error_message: Option<String>,
    /// 客户端IP
    pub client_ip: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 耗时（毫秒）
    pub duration_ms: Option<i64>,
    /// 操作时间
    pub created_at: String,
}

/// 登录日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLog {
    /// 日志ID
    pub log_id: String,
    /// 租户ID
    pub tenant_id: Option<String>,
    /// 用户ID
    pub user_id: Option<String>,
    /// 用户名
    pub username: String,
    /// 登录类型（password/sso/oauth2/sms）
    pub login_type: String,
    /// 登录结果（success/failure）
    pub result: String,
    /// 失败原因
    pub failure_reason: Option<String>,
    /// 客户端IP
    pub client_ip: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 登录时间
    pub login_at: String,
}

/// 操作日志统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationLogStats {
    /// 总操作数
    pub total_operations: i64,
    /// 成功数
    pub success_count: i64,
    /// 失败数
    pub failure_count: i64,
    /// 成功率
    pub success_rate: f64,
    /// 平均耗时（毫秒）
    pub avg_duration_ms: f64,
    /// 按模块统计
    pub by_module: HashMap<String, i64>,
    /// 按操作类型统计
    pub by_operation_type: HashMap<String, i64>,
}

/// 内置示例操作日志
pub fn sample_operation_logs() -> Vec<OperationLog> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        OperationLog {
            log_id: "log_001".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            user_id: "user_001".to_string(),
            username: "admin".to_string(),
            operation_type: "CREATE".to_string(),
            module: "user".to_string(),
            description: "创建用户 zhangsan".to_string(),
            method: Some("POST".to_string()),
            path: Some("/api/enterprise/admin/users".to_string()),
            params: Some("{\"username\":\"zhangsan\"}".to_string()),
            result: "success".to_string(),
            error_message: None,
            client_ip: Some("192.168.1.100".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            duration_ms: Some(45),
            created_at: now.clone(),
        },
        OperationLog {
            log_id: "log_002".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            user_id: "user_001".to_string(),
            username: "admin".to_string(),
            operation_type: "UPDATE".to_string(),
            module: "config".to_string(),
            description: "更新系统配置 max_upload_size".to_string(),
            method: Some("PUT".to_string()),
            path: Some("/api/enterprise/config/items/max_upload_size".to_string()),
            params: Some("{\"config_value\":\"100MB\"}".to_string()),
            result: "success".to_string(),
            error_message: None,
            client_ip: Some("192.168.1.100".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            duration_ms: Some(23),
            created_at: now.clone(),
        },
        OperationLog {
            log_id: "log_003".to_string(),
            tenant_id: Some("tenant_002".to_string()),
            user_id: "user_002".to_string(),
            username: "zhangsan".to_string(),
            operation_type: "DELETE".to_string(),
            module: "document".to_string(),
            description: "删除文档 doc_001".to_string(),
            method: Some("DELETE".to_string()),
            path: Some("/api/enterprise/document/files/doc_001".to_string()),
            params: None,
            result: "failure".to_string(),
            error_message: Some("权限不足".to_string()),
            client_ip: Some("192.168.1.101".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            duration_ms: Some(12),
            created_at: now,
        },
    ]
}

/// 内置示例登录日志
pub fn sample_login_logs() -> Vec<LoginLog> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        LoginLog {
            log_id: "login_001".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            user_id: Some("user_001".to_string()),
            username: "admin".to_string(),
            login_type: "password".to_string(),
            result: "success".to_string(),
            failure_reason: None,
            client_ip: Some("192.168.1.100".to_string()),
            user_agent: Some("Mozilla/5.0".to_string()),
            login_at: now.clone(),
        },
        LoginLog {
            log_id: "login_002".to_string(),
            tenant_id: Some("tenant_001".to_string()),
            user_id: None,
            username: "unknown".to_string(),
            login_type: "password".to_string(),
            result: "failure".to_string(),
            failure_reason: Some("用户名或密码错误".to_string()),
            client_ip: Some("10.0.0.50".to_string()),
            user_agent: Some("curl/7.68.0".to_string()),
            login_at: now,
        },
    ]
}
