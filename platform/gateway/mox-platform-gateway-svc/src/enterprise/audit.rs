//! 企业级审计日志模块
//!
//! 记录所有操作的审计日志，支持：
//! - 操作类型分类（创建/更新/删除/查询/登录/登出/审批/签署等）
//! - 操作人信息（用户ID/租户ID/部门ID/IP地址/User-Agent）
//! - 操作对象（资源类型/资源ID/变更前后数据）
//! - 操作结果（成功/失败/错误信息）
//! - 审计日志查询与导出

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 审计日志记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    /// 日志ID
    pub log_id: String,
    /// 租户ID
    pub tenant_id: String,
    /// 操作人用户ID
    pub user_id: String,
    /// 操作人用户名
    pub username: String,
    /// 操作人部门ID
    pub department_id: Option<String>,
    /// 操作类型
    pub action_type: AuditActionType,
    /// 操作模块
    pub module: String,
    /// 资源类型
    pub resource_type: String,
    /// 资源ID
    pub resource_id: Option<String>,
    /// 操作描述
    pub description: String,
    /// 操作前数据（JSON）
    pub before_data: Option<serde_json::Value>,
    /// 操作后数据（JSON）
    pub after_data: Option<serde_json::Value>,
    /// 变更字段列表
    pub changed_fields: Option<Vec<String>>,
    /// 操作结果
    pub result: AuditResult,
    /// 错误信息（失败时）
    pub error_message: Option<String>,
    /// 请求ID（链路追踪）
    pub request_id: Option<String>,
    /// 会话ID
    pub session_id: Option<String>,
    /// IP地址
    pub ip_address: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 操作耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 操作时间
    pub created_at: String,
}

/// 审计操作类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditActionType {
    /// 创建
    Create,
    /// 更新
    Update,
    /// 删除
    Delete,
    /// 查询
    Query,
    /// 导出
    Export,
    /// 导入
    Import,
    /// 登录
    Login,
    /// 登出
    Logout,
    /// 审批通过
    Approve,
    /// 审批拒绝
    Reject,
    /// 审批转交
    Transfer,
    /// 签署
    Sign,
    /// 撤回
    Withdraw,
    /// 发布
    Publish,
    /// 归档
    Archive,
    /// 权限变更
    PermissionChange,
    /// 配置变更
    ConfigChange,
    /// 系统操作
    System,
    /// 其他
    Other,
}

impl AuditActionType {
    pub fn as_str(&self) -> &str {
        match self {
            AuditActionType::Create => "create",
            AuditActionType::Update => "update",
            AuditActionType::Delete => "delete",
            AuditActionType::Query => "query",
            AuditActionType::Export => "export",
            AuditActionType::Import => "import",
            AuditActionType::Login => "login",
            AuditActionType::Logout => "logout",
            AuditActionType::Approve => "approve",
            AuditActionType::Reject => "reject",
            AuditActionType::Transfer => "transfer",
            AuditActionType::Sign => "sign",
            AuditActionType::Withdraw => "withdraw",
            AuditActionType::Publish => "publish",
            AuditActionType::Archive => "archive",
            AuditActionType::PermissionChange => "permission_change",
            AuditActionType::ConfigChange => "config_change",
            AuditActionType::System => "system",
            AuditActionType::Other => "other",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            AuditActionType::Create => "创建",
            AuditActionType::Update => "更新",
            AuditActionType::Delete => "删除",
            AuditActionType::Query => "查询",
            AuditActionType::Export => "导出",
            AuditActionType::Import => "导入",
            AuditActionType::Login => "登录",
            AuditActionType::Logout => "登出",
            AuditActionType::Approve => "审批通过",
            AuditActionType::Reject => "审批拒绝",
            AuditActionType::Transfer => "审批转交",
            AuditActionType::Sign => "签署",
            AuditActionType::Withdraw => "撤回",
            AuditActionType::Publish => "发布",
            AuditActionType::Archive => "归档",
            AuditActionType::PermissionChange => "权限变更",
            AuditActionType::ConfigChange => "配置变更",
            AuditActionType::System => "系统操作",
            AuditActionType::Other => "其他",
        }
    }
}

/// 审计操作结果
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 拒绝
    Denied,
    /// 异常
    Error,
}

impl AuditResult {
    pub fn as_str(&self) -> &str {
        match self {
            AuditResult::Success => "success",
            AuditResult::Failed => "failed",
            AuditResult::Denied => "denied",
            AuditResult::Error => "error",
        }
    }
}

/// 审计日志构建器
pub struct AuditLogBuilder {
    log: AuditLog,
}

impl AuditLogBuilder {
    /// 创建新的审计日志构建器
    pub fn new(action_type: AuditActionType, module: &str, resource_type: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            log: AuditLog {
                log_id: format!("audit_{}", uuid::Uuid::new_v4().simple()),
                tenant_id: "default".to_string(),
                user_id: "system".to_string(),
                username: "系统".to_string(),
                department_id: None,
                action_type,
                module: module.to_string(),
                resource_type: resource_type.to_string(),
                resource_id: None,
                description: String::new(),
                before_data: None,
                after_data: None,
                changed_fields: None,
                result: AuditResult::Success,
                error_message: None,
                request_id: None,
                session_id: None,
                ip_address: None,
                user_agent: None,
                duration_ms: None,
                created_at: now,
            },
        }
    }

    /// 设置租户ID
    pub fn tenant_id(mut self, tenant_id: &str) -> Self {
        self.log.tenant_id = tenant_id.to_string();
        self
    }

    /// 设置操作人
    pub fn operator(mut self, user_id: &str, username: &str) -> Self {
        self.log.user_id = user_id.to_string();
        self.log.username = username.to_string();
        self
    }

    /// 设置部门ID
    pub fn department_id(mut self, department_id: &str) -> Self {
        self.log.department_id = Some(department_id.to_string());
        self
    }

    /// 设置资源ID
    pub fn resource_id(mut self, resource_id: &str) -> Self {
        self.log.resource_id = Some(resource_id.to_string());
        self
    }

    /// 设置操作描述
    pub fn description(mut self, description: &str) -> Self {
        self.log.description = description.to_string();
        self
    }

    /// 设置操作前数据
    pub fn before_data(mut self, data: serde_json::Value) -> Self {
        self.log.before_data = Some(data);
        self
    }

    /// 设置操作后数据
    pub fn after_data(mut self, data: serde_json::Value) -> Self {
        self.log.after_data = Some(data);
        self
    }

    /// 设置变更字段
    pub fn changed_fields(mut self, fields: Vec<String>) -> Self {
        self.log.changed_fields = Some(fields);
        self
    }

    /// 设置操作结果
    pub fn result(mut self, result: AuditResult) -> Self {
        self.log.result = result;
        self
    }

    /// 设置错误信息
    pub fn error_message(mut self, error: &str) -> Self {
        self.log.error_message = Some(error.to_string());
        self.log.result = AuditResult::Failed;
        self
    }

    /// 设置请求ID
    pub fn request_id(mut self, request_id: &str) -> Self {
        self.log.request_id = Some(request_id.to_string());
        self
    }

    /// 设置IP地址
    pub fn ip_address(mut self, ip: &str) -> Self {
        self.log.ip_address = Some(ip.to_string());
        self
    }

    /// 设置User-Agent
    pub fn user_agent(mut self, ua: &str) -> Self {
        self.log.user_agent = Some(ua.to_string());
        self
    }

    /// 设置操作耗时
    pub fn duration_ms(mut self, duration: u64) -> Self {
        self.log.duration_ms = Some(duration);
        self
    }

    /// 构建审计日志
    pub fn build(self) -> AuditLog {
        self.log
    }
}

/// 审计日志服务状态
pub struct AuditState {
    /// 审计日志存储（内存，生产环境应迁移到数据库）
    pub logs: Arc<RwLock<Vec<AuditLog>>>,
    /// 最大保留条数
    pub max_logs: usize,
}

impl AuditState {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
            max_logs: 100_000,
        }
    }

    /// 记录审计日志
    pub async fn record(&self, log: AuditLog) {
        let mut logs = self.logs.write().await;
        logs.push(log);
        // 超过最大保留条数时，删除最旧的日志
        if logs.len() > self.max_logs {
            let remove_count = logs.len() - self.max_logs;
            logs.drain(0..remove_count);
        }
    }

    /// 快速记录成功操作
    pub async fn record_success(
        &self,
        action_type: AuditActionType,
        module: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        description: &str,
        user_id: &str,
        username: &str,
    ) {
        let mut builder = AuditLogBuilder::new(action_type, module, resource_type)
            .operator(user_id, username)
            .description(description)
            .result(AuditResult::Success);
        if let Some(rid) = resource_id {
            builder = builder.resource_id(rid);
        }
        self.record(builder.build()).await;
    }

    /// 快速记录失败操作
    pub async fn record_failure(
        &self,
        action_type: AuditActionType,
        module: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        description: &str,
        error: &str,
        user_id: &str,
        username: &str,
    ) {
        let mut builder = AuditLogBuilder::new(action_type, module, resource_type)
            .operator(user_id, username)
            .description(description)
            .error_message(error);
        if let Some(rid) = resource_id {
            builder = builder.resource_id(rid);
        }
        self.record(builder.build()).await;
    }

    /// 查询审计日志
    pub async fn query(
        &self,
        tenant_id: Option<&str>,
        user_id: Option<&str>,
        action_type: Option<&str>,
        module: Option<&str>,
        resource_type: Option<&str>,
        result: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
    ) -> Vec<AuditLog> {
        let logs = self.logs.read().await;
        logs.iter()
            .filter(|log| {
                if let Some(tid) = tenant_id {
                    if log.tenant_id != tid { return false; }
                }
                if let Some(uid) = user_id {
                    if log.user_id != uid { return false; }
                }
                if let Some(at) = action_type {
                    if log.action_type.as_str() != at { return false; }
                }
                if let Some(m) = module {
                    if log.module != m { return false; }
                }
                if let Some(rt) = resource_type {
                    if log.resource_type != rt { return false; }
                }
                if let Some(r) = result {
                    if log.result.as_str() != r { return false; }
                }
                if let Some(st) = start_time {
                    if log.created_at.as_str() < st { return false; }
                }
                if let Some(et) = end_time {
                    if log.created_at.as_str() > et { return false; }
                }
                true
            })
            .cloned()
            .collect()
    }

    /// 获取审计统计
    pub async fn stats(&self) -> HashMap<String, i64> {
        let logs = self.logs.read().await;
        let mut stats = HashMap::new();
        stats.insert("total".to_string(), logs.len() as i64);
        stats.insert("success".to_string(), logs.iter().filter(|l| l.result == AuditResult::Success).count() as i64);
        stats.insert("failed".to_string(), logs.iter().filter(|l| l.result == AuditResult::Failed).count() as i64);
        stats.insert("denied".to_string(), logs.iter().filter(|l| l.result == AuditResult::Denied).count() as i64);
        stats
    }
}

impl Default for AuditState {
    fn default() -> Self {
        Self::new()
    }
}
