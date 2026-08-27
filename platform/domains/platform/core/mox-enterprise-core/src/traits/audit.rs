//! 审计日志Trait — Audit Logger
//!
//! 企业级审计日志抽象，可替换实现：文件/数据库/消息队列/SIEM系统。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 审计严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverity {
    /// 信息（普通操作）
    Info,
    /// 警告（可疑操作）
    Warning,
    /// 高危（敏感操作）
    High,
    /// 严重（违规操作）
    Critical,
}

impl AuditSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Info => "info",
            AuditSeverity::Warning => "warning",
            AuditSeverity::High => "high",
            AuditSeverity::Critical => "critical",
        }
    }
}

/// 审计事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// 事件ID
    pub event_id: String,
    /// 事件类型（如 "user.login", "data.export", "config.change"）
    pub event_type: String,
    /// 严重级别
    pub severity: AuditSeverity,
    /// 操作人ID
    pub actor_id: String,
    /// 操作人类型（user/system/service）
    pub actor_type: String,
    /// 操作人IP
    #[serde(default)]
    pub actor_ip: Option<String>,
    /// 租户ID
    #[serde(default)]
    pub tenant_id: Option<String>,
    /// 目标资源类型
    pub resource_type: String,
    /// 目标资源ID
    pub resource_id: String,
    /// 操作动作（create/read/update/delete/execute）
    pub action: String,
    /// 操作结果（success/failure）
    pub result: String,
    /// 错误信息（失败时）
    #[serde(default)]
    pub error_message: Option<String>,
    /// 请求参数（脱敏后）
    #[serde(default)]
    pub request_params: serde_json::Value,
    /// 响应摘要
    #[serde(default)]
    pub response_summary: Option<String>,
    /// 时间戳（RFC3339）
    pub timestamp: String,
    /// 追踪ID
    #[serde(default)]
    pub trace_id: Option<String>,
    /// 附加属性
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl AuditEvent {
    /// 创建基础审计事件
    pub fn new(event_type: impl Into<String>, actor_id: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            event_type: event_type.into(),
            severity: AuditSeverity::Info,
            actor_id: actor_id.into(),
            actor_type: "user".into(),
            actor_ip: None,
            tenant_id: None,
            resource_type: String::new(),
            resource_id: String::new(),
            action: action.into(),
            result: "success".into(),
            error_message: None,
            request_params: serde_json::Value::Null,
            response_summary: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            trace_id: None,
            attributes: HashMap::new(),
        }
    }

    pub fn with_severity(mut self, severity: AuditSeverity) -> Self { self.severity = severity; self }
    pub fn with_resource(mut self, rtype: impl Into<String>, rid: impl Into<String>) -> Self {
        self.resource_type = rtype.into(); self.resource_id = rid.into(); self
    }
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self { self.tenant_id = Some(tenant_id.into()); self }
    pub fn with_trace(mut self, trace_id: impl Into<String>) -> Self { self.trace_id = Some(trace_id.into()); self }
    pub fn failure(mut self, error: impl Into<String>) -> Self { self.result = "failure".into(); self.error_message = Some(error.into()); self }
}

/// 审计结果
pub type AuditResult = anyhow::Result<()>;

/// 审计日志器Trait
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// 记录审计事件
    async fn log(&self, event: AuditEvent) -> AuditResult;

    /// 批量记录审计事件
    async fn log_batch(&self, events: Vec<AuditEvent>) -> AuditResult {
        for event in events {
            self.log(event).await?;
        }
        Ok(())
    }

    /// 查询审计事件（可选实现，用于审计查询接口）
    async fn query(&self, _filter: AuditQueryFilter) -> anyhow::Result<Vec<AuditEvent>> {
        Err(anyhow::anyhow!("query not supported by this audit logger"))
    }

    /// 刷新/刷新缓冲区（可选实现）
    async fn flush(&self) -> AuditResult { Ok(()) }
}

/// 审计查询过滤器
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditQueryFilter {
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub actor_id: Option<String>,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub severity_min: Option<AuditSeverity>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub offset: Option<u32>,
}
