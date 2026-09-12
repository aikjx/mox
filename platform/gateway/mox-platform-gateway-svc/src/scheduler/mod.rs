//! 定时任务调度模块
//!
//! 支持：CRON表达式 / 固定间隔 / 一次性任务
//! 内置任务：数据同步 / 报表生成 / 缓存清理 / 日志归档 / 消息推送 / 备份

pub mod api;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待运行
    Pending,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
    /// 已暂停
    Paused,
}

impl TaskStatus {
    pub fn as_str(&self) -> &str {
        match self {
            TaskStatus::Pending => "pending",
            TaskStatus::Running => "running",
            TaskStatus::Completed => "completed",
            TaskStatus::Failed => "failed",
            TaskStatus::Cancelled => "cancelled",
            TaskStatus::Paused => "paused",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TaskStatus::Pending => "待运行",
            TaskStatus::Running => "运行中",
            TaskStatus::Completed => "已完成",
            TaskStatus::Failed => "失败",
            TaskStatus::Cancelled => "已取消",
            TaskStatus::Paused => "已暂停",
        }
    }
}

/// 任务类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    /// CRON表达式任务
    Cron,
    /// 固定间隔任务（秒）
    Interval,
    /// 一次性任务（指定时间）
    Once,
    /// 手动触发任务
    Manual,
}

impl TaskType {
    pub fn as_str(&self) -> &str {
        match self {
            TaskType::Cron => "cron",
            TaskType::Interval => "interval",
            TaskType::Once => "once",
            TaskType::Manual => "manual",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            TaskType::Cron => "CRON定时",
            TaskType::Interval => "固定间隔",
            TaskType::Once => "一次性",
            TaskType::Manual => "手动触发",
        }
    }
}

/// 任务优先级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    /// 低
    Low = 0,
    /// 普通
    Normal = 1,
    /// 高
    High = 2,
    /// 紧急
    Urgent = 3,
}

impl TaskPriority {
    pub fn as_str(&self) -> &str {
        match self {
            TaskPriority::Low => "low",
            TaskPriority::Normal => "normal",
            TaskPriority::High => "high",
            TaskPriority::Urgent => "urgent",
        }
    }
}

/// 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// 任务ID
    pub task_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 任务名称
    pub task_name: String,
    /// 任务描述
    pub description: Option<String>,
    /// 任务类型
    pub task_type: TaskType,
    /// 任务分类（数据同步/报表生成/缓存清理/日志归档/消息推送/备份/自定义）
    pub category: String,
    /// CRON表达式（task_type=cron时必填）
    pub cron_expression: Option<String>,
    /// 执行间隔秒数（task_type=interval时必填）
    pub interval_seconds: Option<u64>,
    /// 执行时间（task_type=once时必填，ISO8601格式）
    pub execute_at: Option<String>,
    /// 任务处理器类型（对应内置处理器或自定义处理器）
    pub handler_type: String,
    /// 任务参数
    pub params: Option<HashMap<String, serde_json::Value>>,
    /// 优先级
    pub priority: TaskPriority,
    /// 是否启用
    pub enabled: bool,
    /// 最大重试次数
    pub max_retry: u32,
    /// 重试间隔秒数
    pub retry_interval_seconds: u64,
    /// 超时时间秒数
    pub timeout_seconds: u64,
    /// 是否允许并发执行
    pub allow_concurrent: bool,
    /// 上次执行时间
    pub last_executed_at: Option<String>,
    /// 下次执行时间
    pub next_execution_at: Option<String>,
    /// 累计执行次数
    pub execution_count: u64,
    /// 累计成功次数
    pub success_count: u64,
    /// 累计失败次数
    pub failure_count: u64,
    /// 平均执行耗时（毫秒）
    pub avg_duration_ms: Option<u64>,
    /// 创建人
    pub created_by: String,
    /// 创建时间
    pub created_at: String,
    /// 更新时间
    pub updated_at: String,
}

/// 任务执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    /// 执行记录ID
    pub execution_id: String,
    /// 任务ID
    pub task_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 触发方式（scheduled/manual/retry/api）
    pub trigger_type: String,
    /// 执行状态
    pub status: TaskStatus,
    /// 开始时间
    pub started_at: String,
    /// 结束时间
    pub finished_at: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 重试次数
    pub retry_count: u32,
    /// 执行结果数据
    pub result_data: Option<serde_json::Value>,
    /// 错误信息
    pub error_message: Option<String>,
    /// 错误堆栈
    pub error_stack: Option<String>,
    /// 执行节点（分布式部署时标识执行节点）
    pub execution_node: Option<String>,
    /// 触发人（手动触发时）
    pub triggered_by: Option<String>,
}

/// 任务统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStats {
    /// 任务总数
    pub total: i64,
    /// 已启用任务数
    pub enabled: i64,
    /// 已禁用任务数
    pub disabled: i64,
    /// 运行中任务数
    pub running: i64,
    /// 今日执行次数
    pub today_executions: i64,
    /// 今日成功次数
    pub today_success: i64,
    /// 今日失败次数
    pub today_failures: i64,
    /// 成功率（百分比）
    pub success_rate: Option<f64>,
    /// 平均执行耗时（毫秒）
    pub avg_duration_ms: Option<u64>,
}

/// 创建任务请求
#[derive(Debug, Deserialize)]
pub struct CreateScheduledTaskRequest {
    pub task_name: String,
    pub description: Option<String>,
    pub task_type: TaskType,
    pub category: String,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<u64>,
    pub execute_at: Option<String>,
    pub handler_type: String,
    pub params: Option<HashMap<String, serde_json::Value>>,
    pub priority: Option<TaskPriority>,
    pub enabled: Option<bool>,
    pub max_retry: Option<u32>,
    pub retry_interval_seconds: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub allow_concurrent: Option<bool>,
}

/// 更新任务请求
#[derive(Debug, Deserialize)]
pub struct UpdateScheduledTaskRequest {
    pub task_name: Option<String>,
    pub description: Option<String>,
    pub task_type: Option<TaskType>,
    pub category: Option<String>,
    pub cron_expression: Option<String>,
    pub interval_seconds: Option<u64>,
    pub execute_at: Option<String>,
    pub handler_type: Option<String>,
    pub params: Option<HashMap<String, serde_json::Value>>,
    pub priority: Option<TaskPriority>,
    pub max_retry: Option<u32>,
    pub retry_interval_seconds: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub allow_concurrent: Option<bool>,
}

/// 手动触发任务请求
#[derive(Debug, Deserialize)]
pub struct TriggerTaskRequest {
    pub params: Option<HashMap<String, serde_json::Value>>,
}

/// 内置任务模板
pub fn builtin_task_templates() -> Vec<ScheduledTask> {
    let now = chrono::Utc::now().to_rfc3339();
    vec![
        ScheduledTask {
            task_id: "template_data_sync".to_string(),
            tenant_id: "system".to_string(),
            task_name: "OA/ERP数据同步".to_string(),
            description: Some("定时从SAP/Oracle/Workday/北森等系统同步组织架构、人员、部门数据".to_string()),
            task_type: TaskType::Cron,
            category: "data_sync".to_string(),
            cron_expression: Some("0 */30 * * * *".to_string()),
            interval_seconds: None,
            execute_at: None,
            handler_type: "integration_sync".to_string(),
            params: Some(HashMap::new()),
            priority: TaskPriority::Normal,
            enabled: false,
            max_retry: 3,
            retry_interval_seconds: 60,
            timeout_seconds: 300,
            allow_concurrent: false,
            last_executed_at: None,
            next_execution_at: None,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ScheduledTask {
            task_id: "template_report_generation".to_string(),
            tenant_id: "system".to_string(),
            task_name: "报表自动生成".to_string(),
            description: Some("定时生成日报/周报/月报，支持邮件推送".to_string()),
            task_type: TaskType::Cron,
            category: "report".to_string(),
            cron_expression: Some("0 0 1 * * *".to_string()),
            interval_seconds: None,
            execute_at: None,
            handler_type: "report_generation".to_string(),
            params: Some(HashMap::new()),
            priority: TaskPriority::Normal,
            enabled: false,
            max_retry: 2,
            retry_interval_seconds: 300,
            timeout_seconds: 600,
            allow_concurrent: false,
            last_executed_at: None,
            next_execution_at: None,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ScheduledTask {
            task_id: "template_cache_cleanup".to_string(),
            tenant_id: "system".to_string(),
            task_name: "缓存清理".to_string(),
            description: Some("定时清理过期缓存、临时文件、会话数据".to_string()),
            task_type: TaskType::Cron,
            category: "maintenance".to_string(),
            cron_expression: Some("0 0 3 * * *".to_string()),
            interval_seconds: None,
            execute_at: None,
            handler_type: "cache_cleanup".to_string(),
            params: Some(HashMap::new()),
            priority: TaskPriority::Low,
            enabled: true,
            max_retry: 1,
            retry_interval_seconds: 60,
            timeout_seconds: 300,
            allow_concurrent: false,
            last_executed_at: None,
            next_execution_at: None,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ScheduledTask {
            task_id: "template_log_archive".to_string(),
            tenant_id: "system".to_string(),
            task_name: "日志归档".to_string(),
            description: Some("定时归档历史日志、审计日志、操作日志，压缩存储".to_string()),
            task_type: TaskType::Cron,
            category: "maintenance".to_string(),
            cron_expression: Some("0 0 4 * * 0".to_string()),
            interval_seconds: None,
            execute_at: None,
            handler_type: "log_archive".to_string(),
            params: Some(HashMap::new()),
            priority: TaskPriority::Low,
            enabled: false,
            max_retry: 2,
            retry_interval_seconds: 300,
            timeout_seconds: 1800,
            allow_concurrent: false,
            last_executed_at: None,
            next_execution_at: None,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ScheduledTask {
            task_id: "template_message_push".to_string(),
            tenant_id: "system".to_string(),
            task_name: "消息批量推送".to_string(),
            description: Some("定时批量推送待办通知、审批提醒、系统公告".to_string()),
            task_type: TaskType::Interval,
            category: "message".to_string(),
            cron_expression: None,
            interval_seconds: Some(300),
            execute_at: None,
            handler_type: "message_push".to_string(),
            params: Some(HashMap::new()),
            priority: TaskPriority::Normal,
            enabled: true,
            max_retry: 3,
            retry_interval_seconds: 60,
            timeout_seconds: 120,
            allow_concurrent: false,
            last_executed_at: None,
            next_execution_at: None,
            execution_count: 0,
            success_count: 0,
            failure_count: 0,
            avg_duration_ms: None,
            created_by: "system".to_string(),
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

/// 支持的任务分类
pub fn supported_task_categories() -> Vec<(&'static str, &'static str)> {
    vec![
        ("data_sync", "数据同步"),
        ("report", "报表生成"),
        ("message", "消息推送"),
        ("maintenance", "系统维护"),
        ("backup", "数据备份"),
        ("monitor", "监控检查"),
        ("workflow", "流程处理"),
        ("custom", "自定义任务"),
    ]
}

/// 支持的任务处理器类型
pub fn supported_handler_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("integration_sync", "OA/ERP数据同步"),
        ("report_generation", "报表生成"),
        ("cache_cleanup", "缓存清理"),
        ("log_archive", "日志归档"),
        ("message_push", "消息推送"),
        ("data_backup", "数据备份"),
        ("health_check", "健康检查"),
        ("workflow_scan", "流程扫描"),
        ("custom_script", "自定义脚本"),
        ("http_callback", "HTTP回调"),
    ]
}
