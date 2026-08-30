// Copyright (c) 2026 璇玑 RelGraph · 全维归一化统一平台 (Unified Platform)
// Licensed under the MIT License.

//! 企业级治理：审计日志
//!
//! 记录平台所有关键操作的审计轨迹，支持：
//! - 操作审计（谁在什么时间做了什么）
//! - 数据变更审计（修改前后的值）
//! - 权限变更审计
//! - 审计日志查询与导出

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::PlatformResult;
use crate::types::NormalizationSystem;

/// 审计操作类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    /// 创建
    Create,
    /// 读取/查询
    Read,
    /// 更新
    Update,
    /// 删除
    Delete,
    /// 登录
    Login,
    /// 登出
    Logout,
    /// 权限变更
    PermissionChange,
    /// 配置变更
    ConfigChange,
    /// 流程启动
    ProcessStart,
    /// 流程完成
    ProcessComplete,
    /// 导出
    Export,
    /// 导入
    Import,
    /// 自定义操作
    Custom(String),
}

impl AuditAction {
    pub fn name(&self) -> String {
        match self {
            AuditAction::Create => "create".to_string(),
            AuditAction::Read => "read".to_string(),
            AuditAction::Update => "update".to_string(),
            AuditAction::Delete => "delete".to_string(),
            AuditAction::Login => "login".to_string(),
            AuditAction::Logout => "logout".to_string(),
            AuditAction::PermissionChange => "permission_change".to_string(),
            AuditAction::ConfigChange => "config_change".to_string(),
            AuditAction::ProcessStart => "process_start".to_string(),
            AuditAction::ProcessComplete => "process_complete".to_string(),
            AuditAction::Export => "export".to_string(),
            AuditAction::Import => "import".to_string(),
            AuditAction::Custom(s) => format!("custom_{}", s),
        }
    }

    /// 是否为敏感操作（需要额外审计）
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            AuditAction::Delete
                | AuditAction::PermissionChange
                | AuditAction::ConfigChange
                | AuditAction::Export
                | AuditAction::Login
                | AuditAction::Logout
        )
    }
}

/// 审计日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditLevel {
    /// 信息
    Info = 0,
    /// 警告
    Warning = 1,
    /// 危险操作
    Critical = 2,
}

/// 审计日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// 日志 ID
    pub id: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 用户 ID
    pub user_id: String,
    /// 用户名（冗余，便于查询）
    pub user_name: String,
    /// 所属体系
    pub system: NormalizationSystem,
    /// 操作类型
    pub action: AuditAction,
    /// 审计级别
    pub level: AuditLevel,
    /// 资源类型
    pub resource_type: String,
    /// 资源 ID
    pub resource_id: String,
    /// 操作描述
    pub description: String,
    /// 变更前的值（可选）
    pub old_value: Option<serde_json::Value>,
    /// 变更后的值（可选）
    pub new_value: Option<serde_json::Value>,
    /// IP 地址
    pub ip_address: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
    /// 请求 ID / 追踪 ID
    pub trace_id: Option<String>,
    /// 时间戳（毫秒）
    pub timestamp: u64,
    /// 操作结果
    pub success: bool,
    /// 错误信息（失败时）
    pub error_message: Option<String>,
    /// 额外属性
    pub attributes: HashMap<String, String>,
}

impl AuditLogEntry {
    /// 创建审计日志条目
    pub fn new(
        tenant_id: &str,
        user_id: &str,
        system: NormalizationSystem,
        action: AuditAction,
        resource_type: &str,
        resource_id: &str,
        description: &str,
    ) -> Self {
        let level = if action.is_sensitive() {
            AuditLevel::Critical
        } else {
            AuditLevel::Info
        };

        Self {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant_id.to_string(),
            user_id: user_id.to_string(),
            user_name: String::new(),
            system,
            action,
            level,
            resource_type: resource_type.to_string(),
            resource_id: resource_id.to_string(),
            description: description.to_string(),
            old_value: None,
            new_value: None,
            ip_address: None,
            user_agent: None,
            trace_id: None,
            timestamp: now_ms(),
            success: true,
            error_message: None,
            attributes: HashMap::new(),
        }
    }

    /// 设置变更前后的值
    pub fn with_values(mut self, old: serde_json::Value, new: serde_json::Value) -> Self {
        self.old_value = Some(old);
        self.new_value = Some(new);
        self
    }

    /// 设置为失败
    pub fn with_failure(mut self, error: &str) -> Self {
        self.success = false;
        self.error_message = Some(error.to_string());
        self
    }

    /// 设置追踪信息
    pub fn with_trace(mut self, trace_id: &str, ip: &str, user_agent: &str) -> Self {
        self.trace_id = Some(trace_id.to_string());
        self.ip_address = Some(ip.to_string());
        self.user_agent = Some(user_agent.to_string());
        self
    }

    /// 添加额外属性
    pub fn with_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
}

/// 审计日志查询条件
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// 租户 ID
    pub tenant_id: Option<String>,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 所属体系
    pub system: Option<NormalizationSystem>,
    /// 操作类型
    pub action: Option<AuditAction>,
    /// 资源类型
    pub resource_type: Option<String>,
    /// 开始时间
    pub start_time: Option<u64>,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 最小级别
    pub min_level: Option<AuditLevel>,
    /// 是否成功
    pub success: Option<bool>,
    /// 最大返回数
    pub limit: usize,
}

/// 审计日志管理器
pub struct AuditLogger {
    /// 审计日志存储（内存环形缓冲区）
    logs: RwLock<Vec<AuditLogEntry>>,
    /// 最大日志条数（超过后淘汰最老的）
    max_entries: usize,
    /// 审计统计
    stats: RwLock<AuditStats>,
    /// 是否启用
    enabled: RwLock<bool>,
}

/// 审计统计
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditStats {
    /// 总审计日志数
    pub total_entries: u64,
    /// 敏感操作数
    pub sensitive_ops: u64,
    /// 失败操作数
    pub failed_ops: u64,
    /// 按体系统计
    pub by_system: HashMap<String, u64>,
    /// 按操作类型统计
    pub by_action: HashMap<String, u64>,
}

impl AuditLogger {
    /// 创建审计日志管理器
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(Vec::new()),
            max_entries: 100000,
            stats: RwLock::new(AuditStats::default()),
            enabled: RwLock::new(true),
        }
    }

    /// 设置最大日志条数
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// 启用/禁用审计
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// 是否启用
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// 记录审计日志
    pub fn log(&self, entry: AuditLogEntry) {
        if !*self.enabled.read() {
            return;
        }

        // 更新统计
        {
            let mut stats = self.stats.write();
            stats.total_entries += 1;
            if entry.action.is_sensitive() {
                stats.sensitive_ops += 1;
            }
            if !entry.success {
                stats.failed_ops += 1;
            }
            *stats
                .by_system
                .entry(entry.system.name().to_string())
                .or_insert(0) += 1;
            *stats
                .by_action
                .entry(entry.action.name())
                .or_insert(0) += 1;
        }

        // 存储日志
        let mut logs = self.logs.write();
        logs.push(entry);

        // 超过上限时淘汰最老的
        if logs.len() > self.max_entries {
            let overflow = logs.len() - self.max_entries;
            logs.drain(0..overflow);
        }
    }

    /// 便捷方法：记录创建操作
    pub fn log_create(
        &self,
        tenant_id: &str,
        user_id: &str,
        system: NormalizationSystem,
        resource_type: &str,
        resource_id: &str,
        description: &str,
        new_value: serde_json::Value,
    ) {
        let entry = AuditLogEntry::new(
            tenant_id,
            user_id,
            system,
            AuditAction::Create,
            resource_type,
            resource_id,
            description,
        )
        .with_values(serde_json::Value::Null, new_value);
        self.log(entry);
    }

    /// 便捷方法：记录更新操作
    pub fn log_update(
        &self,
        tenant_id: &str,
        user_id: &str,
        system: NormalizationSystem,
        resource_type: &str,
        resource_id: &str,
        description: &str,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
    ) {
        let entry = AuditLogEntry::new(
            tenant_id,
            user_id,
            system,
            AuditAction::Update,
            resource_type,
            resource_id,
            description,
        )
        .with_values(old_value, new_value);
        self.log(entry);
    }

    /// 便捷方法：记录删除操作
    pub fn log_delete(
        &self,
        tenant_id: &str,
        user_id: &str,
        system: NormalizationSystem,
        resource_type: &str,
        resource_id: &str,
        description: &str,
        old_value: serde_json::Value,
    ) {
        let entry = AuditLogEntry::new(
            tenant_id,
            user_id,
            system,
            AuditAction::Delete,
            resource_type,
            resource_id,
            description,
        )
        .with_values(old_value, serde_json::Value::Null);
        self.log(entry);
    }

    /// 便捷方法：记录权限变更
    pub fn log_permission_change(
        &self,
        tenant_id: &str,
        user_id: &str,
        system: NormalizationSystem,
        target_user: &str,
        description: &str,
        old_perms: serde_json::Value,
        new_perms: serde_json::Value,
    ) {
        let entry = AuditLogEntry::new(
            tenant_id,
            user_id,
            system,
            AuditAction::PermissionChange,
            "permission",
            target_user,
            description,
        )
        .with_values(old_perms, new_perms);
        self.log(entry);
    }

    /// 便捷方法：记录登录
    pub fn log_login(
        &self,
        tenant_id: &str,
        user_id: &str,
        ip: &str,
        user_agent: &str,
        success: bool,
        error: Option<&str>,
    ) {
        let mut entry = AuditLogEntry::new(
            tenant_id,
            user_id,
            NormalizationSystem::Permission,
            AuditAction::Login,
            "session",
            user_id,
            if success { "登录成功" } else { "登录失败" },
        );
        entry.ip_address = Some(ip.to_string());
        entry.user_agent = Some(user_agent.to_string());
        entry.success = success;
        if let Some(e) = error {
            entry.error_message = Some(e.to_string());
        }
        self.log(entry);
    }

    /// 查询审计日志
    pub fn query(&self, query: &AuditQuery) -> Vec<AuditLogEntry> {
        let logs = self.logs.read();
        let mut results: Vec<AuditLogEntry> = Vec::new();

        // 从新到旧遍历
        for entry in logs.iter().rev() {
            if results.len() >= query.limit && query.limit > 0 {
                break;
            }

            if let Some(ref tenant) = query.tenant_id {
                if &entry.tenant_id != tenant {
                    continue;
                }
            }
            if let Some(ref user) = query.user_id {
                if &entry.user_id != user {
                    continue;
                }
            }
            if let Some(ref sys) = query.system {
                if &entry.system != sys {
                    continue;
                }
            }
            if let Some(ref action) = query.action {
                if entry.action.name() != action.name() {
                    continue;
                }
            }
            if let Some(ref rt) = query.resource_type {
                if &entry.resource_type != rt {
                    continue;
                }
            }
            if let Some(start) = query.start_time {
                if entry.timestamp < start {
                    continue;
                }
            }
            if let Some(end) = query.end_time {
                if entry.timestamp > end {
                    continue;
                }
            }
            if let Some(min_level) = query.min_level {
                if entry.level < min_level {
                    continue;
                }
            }
            if let Some(success) = query.success {
                if entry.success != success {
                    continue;
                }
            }

            results.push(entry.clone());
        }

        results
    }

    /// 获取统计信息
    pub fn stats(&self) -> AuditStats {
        self.stats.read().clone()
    }

    /// 获取日志总数
    pub fn count(&self) -> usize {
        self.logs.read().len()
    }

    /// 清空日志
    pub fn clear(&self) {
        self.logs.write().clear();
        *self.stats.write() = AuditStats::default();
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_basic() {
        let logger = AuditLogger::new();
        assert_eq!(logger.count(), 0);

        let entry = AuditLogEntry::new(
            "t1",
            "u1",
            NormalizationSystem::Lowcode,
            AuditAction::Create,
            "entity",
            "e1",
            "创建实体",
        );
        logger.log(entry);

        assert_eq!(logger.count(), 1);
        let stats = logger.stats();
        assert_eq!(stats.total_entries, 1);
    }

    #[test]
    fn test_log_create_update_delete() {
        let logger = AuditLogger::new();

        logger.log_create(
            "t1", "u1", NormalizationSystem::Lowcode,
            "entity", "e1", "创建用户实体",
            serde_json::json!({"name": "test"}),
        );

        logger.log_update(
            "t1", "u1", NormalizationSystem::Lowcode,
            "entity", "e1", "更新用户实体",
            serde_json::json!({"name": "old"}),
            serde_json::json!({"name": "new"}),
        );

        logger.log_delete(
            "t1", "u1", NormalizationSystem::Lowcode,
            "entity", "e1", "删除用户实体",
            serde_json::json!({"name": "test"}),
        );

        assert_eq!(logger.count(), 3);
        let stats = logger.stats();
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.failed_ops, 0);
    }

    #[test]
    fn test_sensitive_operations() {
        let logger = AuditLogger::new();

        logger.log_permission_change(
            "t1", "admin", NormalizationSystem::Permission,
            "u1", "修改用户角色",
            serde_json::json!(["user"]),
            serde_json::json!(["user", "admin"]),
        );

        logger.log_login("t1", "u1", "192.168.1.1", "Mozilla/5.0", true, None);

        let stats = logger.stats();
        assert_eq!(stats.sensitive_ops, 2); // 权限变更 + 登录
    }

    #[test]
    fn test_query_by_tenant() {
        let logger = AuditLogger::new();

        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        logger.log_create("t2", "u2", NormalizationSystem::Lowcode, "e", "2", "test", serde_json::json!({}));
        logger.log_create("t1", "u3", NormalizationSystem::Lowcode, "e", "3", "test", serde_json::json!({}));

        let query = AuditQuery {
            tenant_id: Some("t1".to_string()),
            limit: 10,
            ..Default::default()
        };

        let results = logger.query(&query);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_action() {
        let logger = AuditLogger::new();

        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        logger.log_delete("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));

        let query = AuditQuery {
            action: Some(AuditAction::Delete),
            limit: 10,
            ..Default::default()
        };

        let results = logger.query(&query);
        assert_eq!(results.len(), 1);
        assert!(matches!(results[0].action, AuditAction::Delete));
    }

    #[test]
    fn test_query_by_level() {
        let logger = AuditLogger::new();

        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "info", serde_json::json!({}));
        logger.log_permission_change(
            "t1", "admin", NormalizationSystem::Permission,
            "u1", "critical",
            serde_json::json!([]), serde_json::json!(["admin"]),
        );

        let query = AuditQuery {
            min_level: Some(AuditLevel::Critical),
            limit: 10,
            ..Default::default()
        };

        let results = logger.query(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_failed_login() {
        let logger = AuditLogger::new();

        logger.log_login("t1", "u1", "1.2.3.4", "ua", false, Some("wrong password"));

        let stats = logger.stats();
        assert_eq!(stats.failed_ops, 1);

        let query = AuditQuery {
            success: Some(false),
            limit: 10,
            ..Default::default()
        };
        let results = logger.query(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].error_message.as_deref(), Some("wrong password"));
    }

    #[test]
    fn test_disable_audit() {
        let logger = AuditLogger::new();
        logger.set_enabled(false);

        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        assert_eq!(logger.count(), 0);

        logger.set_enabled(true);
        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        assert_eq!(logger.count(), 1);
    }

    #[test]
    fn test_clear() {
        let logger = AuditLogger::new();
        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        assert_eq!(logger.count(), 1);

        logger.clear();
        assert_eq!(logger.count(), 0);
        assert_eq!(logger.stats().total_entries, 0);
    }

    #[test]
    fn test_max_entries_eviction() {
        let logger = AuditLogger::new().with_max_entries(10);

        for i in 0..15 {
            logger.log_create(
                "t1", "u1", NormalizationSystem::Lowcode,
                "e", &i.to_string(), "test",
                serde_json::json!({"i": i}),
            );
        }

        assert_eq!(logger.count(), 10);
        // 保留的应该是最新的 10 条（5-14）
        let stats = logger.stats();
        assert_eq!(stats.total_entries, 15); // 统计数不变
    }

    #[test]
    fn test_query_limit() {
        let logger = AuditLogger::new();

        for i in 0..20 {
            logger.log_create(
                "t1", "u1", NormalizationSystem::Lowcode,
                "e", &i.to_string(), "test",
                serde_json::json!({}),
            );
        }

        let query = AuditQuery {
            tenant_id: Some("t1".to_string()),
            limit: 5,
            ..Default::default()
        };

        let results = logger.query(&query);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_by_system_stats() {
        let logger = AuditLogger::new();

        logger.log_create("t1", "u1", NormalizationSystem::Lowcode, "e", "1", "test", serde_json::json!({}));
        logger.log_create("t1", "u1", NormalizationSystem::Permission, "e", "2", "test", serde_json::json!({}));
        logger.log_create("t1", "u1", NormalizationSystem::AiAssistant, "e", "3", "test", serde_json::json!({}));

        let stats = logger.stats();
        assert_eq!(stats.by_system.len(), 3);
    }

    #[test]
    fn test_audit_entry_builder() {
        let entry = AuditLogEntry::new(
            "t1", "u1", NormalizationSystem::Lowcode,
            AuditAction::Update, "entity", "e1", "test",
        )
        .with_values(serde_json::json!({"a": 1}), serde_json::json!({"a": 2}))
        .with_trace("trace-1", "10.0.0.1", "Chrome")
        .with_attr("app", "test-app")
        .with_failure("something went wrong");

        assert!(!entry.success);
        assert_eq!(entry.trace_id.as_deref(), Some("trace-1"));
        assert_eq!(entry.ip_address.as_deref(), Some("10.0.0.1"));
        assert_eq!(entry.attributes.get("app").unwrap(), "test-app");
        assert!(entry.old_value.is_some());
        assert!(entry.new_value.is_some());
    }
}
