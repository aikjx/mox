// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 核心类型定义
//!
//! 定义 RBAC/ABAC 引擎的所有基础类型：
//! - [`Action`] — 操作类型（读/写/执行/管理/通配）
//! - [`Effect`] — 策略效果（允许/拒绝）
//! - [`Resource`] — 资源描述（路径 + 租户 + 属性）
//! - [`Subject`] — 主体描述（用户/服务 + 角色 + 属性）
//! - [`Permission`] — 权限定义（操作 + 资源模式）
//! - [`Role`] — 角色定义（名称 + 继承 + 权限 + 属性）
//! - [`Policy`] — 策略定义（主体 + 资源 + 动作 + 效果 + 条件）
//! - [`EvaluationContext`] — 评估上下文（一次权限检查的完整输入）
//! - [`EvaluationResult`] — 评估结果

use std::collections::HashMap;

// ── Action ──────────────────────────────────────────────────────────────────

/// 操作类型
///
/// 标准动作集合，支持自定义动作通过 `Custom(String)` 扩展。
/// `All` 表示通配符，匹配任何动作。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Action {
    /// 读取
    Read,
    /// 写入/创建/更新
    Write,
    /// 执行/运行
    Execute,
    /// 删除
    Delete,
    /// 管理（最高权限，隐含所有其他权限）
    Admin,
    /// 通配符：匹配所有动作
    All,
    /// 自定义动作
    Custom(String),
}

impl Action {
    /// 从字符串解析动作
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "read" => Self::Read,
            "write" => Self::Write,
            "execute" => Self::Execute,
            "delete" => Self::Delete,
            "admin" => Self::Admin,
            "*" | "all" => Self::All,
            other => Self::Custom(other.to_string()),
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::Delete => "delete",
            Self::Admin => "admin",
            Self::All => "*",
            Self::Custom(s) => s.as_str(),
        }
    }

    /// 检查当前动作是否匹配目标动作
    ///
    /// 规则：
    /// - `All` 匹配任何动作
    /// - `Admin` 匹配任何非 `All` 的动作（admin 隐含所有权限）
    /// - 相同变体直接匹配
    /// - `Custom` 按字符串相等匹配
    pub fn matches(&self, target: &Action) -> bool {
        match (self, target) {
            (Self::All, _) => true,
            (_, Self::All) => true,
            (Self::Admin, _) => true,
            (a, b) => a == b,
        }
    }
}

impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ── Effect ──────────────────────────────────────────────────────────────────

/// 策略效果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Effect {
    /// 允许
    Allow,
    /// 拒绝
    Deny,
}

impl Effect {
    pub fn is_allow(self) -> bool {
        matches!(self, Self::Allow)
    }

    pub fn is_deny(self) -> bool {
        matches!(self, Self::Deny)
    }
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Allow => f.write_str("allow"),
            Self::Deny => f.write_str("deny"),
        }
    }
}

// ── Resource ────────────────────────────────────────────────────────────────

/// 属性映射（用于 ABAC）
pub type Attributes = HashMap<String, AttributeValue>;

/// 属性值类型
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AttributeValue {
    /// 字符串值
    String(String),
    /// 整数值
    Int(i64),
    /// 布尔值
    Bool(bool),
    /// 字符串列表
    List(Vec<String>),
}

impl AttributeValue {
    /// 作为字符串引用
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// 作为整数
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 作为布尔值
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 作为列表
    pub fn as_list(&self) -> Option<&[String]> {
        match self {
            Self::List(l) => Some(l.as_slice()),
            _ => None,
        }
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for AttributeValue {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<String>> for AttributeValue {
    fn from(v: Vec<String>) -> Self {
        Self::List(v)
    }
}

/// 资源描述
///
/// 包含资源路径、所属租户和动态属性（用于 ABAC）。
/// 资源路径采用层级命名，如 `db:prod/citizen_info`、`flow:gov-pii/report`。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resource {
    /// 资源路径（支持 : 和 / 分隔层级）
    pub path: String,
    /// 租户标识（用于跨租户隔离）
    pub tenant: Option<String>,
    /// 资源属性（用于 ABAC 条件评估）
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl Resource {
    /// 创建无租户的资源
    pub fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            tenant: None,
            attributes: Attributes::new(),
        }
    }

    /// 创建带租户的资源
    pub fn with_tenant(path: &str, tenant: &str) -> Self {
        Self {
            path: path.into(),
            tenant: Some(tenant.into()),
            attributes: Attributes::new(),
        }
    }

    /// 添加属性
    pub fn with_attr(mut self, key: &str, value: impl Into<AttributeValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

// ── Subject ─────────────────────────────────────────────────────────────────

/// 主体（谁在发起请求）
///
/// 包含主体标识、角色列表和动态属性（用于 ABAC）。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Subject {
    /// 主体 ID，如 `user:alice`、`service:flow-engine`
    pub id: String,
    /// 角色列表
    pub roles: Vec<String>,
    /// 主体属性（用于 ABAC 条件评估）
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl Subject {
    /// 创建主体
    pub fn new(id: &str, roles: Vec<String>) -> Self {
        Self {
            id: id.into(),
            roles,
            attributes: Attributes::new(),
        }
    }

    /// 添加属性
    pub fn with_attr(mut self, key: &str, value: impl Into<AttributeValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// 获取租户（从 ID 前缀解析，如 `tenant:A:user:alice`）
    pub fn tenant(&self) -> Option<&str> {
        if self.id.starts_with("tenant:") {
            let rest = &self.id["tenant:".len()..];
            rest.find(':').map(|idx| &rest[..idx])
        } else {
            None
        }
    }
}

// ── Permission ──────────────────────────────────────────────────────────────

/// 权限定义：操作 + 资源路径模式
///
/// 资源路径支持通配符：
/// - `*` 匹配所有资源
/// - `db:prod/*` 匹配 `db:prod/` 下的所有子资源
/// - 精确匹配：`db:prod/citizen_info`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Permission {
    /// 操作
    pub action: Action,
    /// 资源路径模式（支持通配符）
    pub resource_pattern: String,
}

impl Permission {
    /// 创建权限
    pub fn new(action: Action, resource_pattern: &str) -> Self {
        Self {
            action,
            resource_pattern: resource_pattern.into(),
        }
    }

    /// 从字符串动作创建
    pub fn from_str(action: &str, resource_pattern: &str) -> Self {
        Self {
            action: Action::from_str(action),
            resource_pattern: resource_pattern.into(),
        }
    }

    /// 检查此权限是否覆盖指定的动作和资源
    pub fn matches(&self, action: &Action, resource: &str) -> bool {
        self.action.matches(action) && self.wildcard_match(resource)
    }

    /// 通配符资源路径匹配
    fn wildcard_match(&self, resource: &str) -> bool {
        let pattern = &self.resource_pattern;
        if pattern == "*" {
            return true;
        }
        // 尾缀通配：db:prod/* 匹配 db:prod/a、db:prod/a/b
        if let Some(stripped) = pattern.strip_suffix("/*") {
            return resource.starts_with(stripped)
                && resource.as_bytes().get(stripped.len()) == Some(&b'/');
        }
        // 前缀通配（路径段级）：db:* 匹配 db:anything
        if let Some(stripped) = pattern.strip_suffix('*') {
            return resource.starts_with(stripped);
        }
        // 精确匹配
        pattern == resource
    }
}

// ── Role ────────────────────────────────────────────────────────────────────

/// 角色定义
///
/// 角色是权限的集合，支持多继承。角色也可以有属性（用于 ABAC）。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Role {
    /// 角色名称（唯一标识）
    pub name: String,
    /// 继承的父角色名列表
    pub extends: Vec<String>,
    /// 直接授予的权限列表
    pub permissions: Vec<Permission>,
    /// 角色描述
    pub description: Option<String>,
    /// 角色属性（用于 ABAC 条件评估）
    #[cfg_attr(feature = "serde", serde(default))]
    pub attributes: Attributes,
}

impl Role {
    /// 创建空角色
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            extends: Vec::new(),
            permissions: Vec::new(),
            description: None,
            attributes: Attributes::new(),
        }
    }

    /// 添加父角色（链式调用）
    pub fn extends(mut self, parent: &str) -> Self {
        self.extends.push(parent.into());
        self
    }

    /// 授予权限（链式调用）
    pub fn grant(mut self, action: Action, resource: &str) -> Self {
        self.permissions.push(Permission::new(action, resource));
        self
    }

    /// 授予权限（字符串动作，链式调用）
    pub fn grant_str(mut self, action: &str, resource: &str) -> Self {
        self.permissions.push(Permission::from_str(action, resource));
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 添加属性
    pub fn with_attr(mut self, key: &str, value: impl Into<AttributeValue>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

// ── Policy ──────────────────────────────────────────────────────────────────

/// 策略定义
///
/// 策略是访问控制的基本规则单元。一条策略包含：
/// - 主体模式（哪些角色/用户适用）
/// - 资源模式（哪些资源适用）
/// - 动作
/// - 效果（允许/拒绝）
/// - 条件表达式（ABAC，可选）
///
/// 策略评估遵循"拒绝优先"原则：任何匹配的 Deny 策略都会拒绝访问。
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Policy {
    /// 策略 ID（唯一标识）
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 适用的角色模式列表（支持通配符）
    pub role_patterns: Vec<String>,
    /// 适用的资源模式列表（支持通配符）
    pub resource_patterns: Vec<String>,
    /// 适用的动作
    pub actions: Vec<Action>,
    /// 策略效果
    pub effect: Effect,
    /// ABAC 条件表达式（可选，为空则无条件）
    /// 语法：`subject.department == resource.owner_department`
    pub condition: Option<String>,
    /// 策略描述
    pub description: Option<String>,
    /// 优先级（数字越小优先级越高，默认 100）
    pub priority: u32,
}

impl Policy {
    /// 创建策略
    pub fn new(id: &str, name: &str, effect: Effect) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            role_patterns: Vec::new(),
            resource_patterns: Vec::new(),
            actions: Vec::new(),
            effect,
            condition: None,
            description: None,
            priority: 100,
        }
    }

    /// 添加角色模式
    pub fn for_role(mut self, role: &str) -> Self {
        self.role_patterns.push(role.into());
        self
    }

    /// 添加资源模式
    pub fn on_resource(mut self, resource: &str) -> Self {
        self.resource_patterns.push(resource.into());
        self
    }

    /// 添加动作
    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// 添加字符串动作
    pub fn with_action_str(mut self, action: &str) -> Self {
        self.actions.push(Action::from_str(action));
        self
    }

    /// 设置条件表达式
    pub fn with_condition(mut self, condition: &str) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// 设置描述
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// 设置优先级
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

// ── EvaluationContext ───────────────────────────────────────────────────────

/// 评估上下文：一次权限检查的完整输入
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// 主体
    pub subject: Subject,
    /// 资源
    pub resource: Resource,
    /// 动作
    pub action: Action,
    /// 环境属性（时间、IP 等，用于 ABAC）
    pub environment: Attributes,
}

impl EvaluationContext {
    /// 创建评估上下文
    pub fn new(subject: Subject, resource: Resource, action: Action) -> Self {
        Self {
            subject,
            resource,
            action,
            environment: Attributes::new(),
        }
    }

    /// 添加环境属性
    pub fn with_env(mut self, key: &str, value: impl Into<AttributeValue>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }
}

// ── EvaluationResult ────────────────────────────────────────────────────────

/// 评估结果
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EvaluationResult {
    /// 允许访问
    Granted {
        /// 匹配到的策略 ID 列表（按优先级排序）
        matched_policies: Vec<String>,
    },
    /// 拒绝访问
    Denied {
        /// 拒绝原因
        reason: String,
        /// 匹配到的拒绝策略 ID（如果是被策略拒绝的）
        denied_by_policy: Option<String>,
    },
}

impl EvaluationResult {
    /// 是否允许
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted { .. })
    }

    /// 是否拒绝
    pub fn is_denied(&self) -> bool {
        matches!(self, Self::Denied { .. })
    }

    /// 获取拒绝原因
    pub fn denied_reason(&self) -> Option<&str> {
        match self {
            Self::Denied { reason, .. } => Some(reason.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for EvaluationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Granted { matched_policies } => {
                write!(f, "Granted ({} policies matched)", matched_policies.len())
            }
            Self::Denied {
                reason,
                denied_by_policy,
            } => {
                if let Some(policy) = denied_by_policy {
                    write!(f, "Denied by policy '{}': {}", policy, reason)
                } else {
                    write!(f, "Denied: {}", reason)
                }
            }
        }
    }
}

// ── 内置角色定义（便捷构造） ─────────────────────────────────────────────────

/// 内置角色集合（标准 RBAC 模型）
pub struct BuiltinRoles;

impl BuiltinRoles {
    /// 获取标准内置角色列表
    ///
    /// 角色层级：admin > editor > viewer
    /// 附加角色：safety_approver（生产审批）、operator（运维）、auditor（审计）
    pub fn all() -> Vec<Role> {
        vec![
            Role::new("admin")
                .extends("editor")
                .grant_str("admin", "*")
                .with_description("系统管理员，拥有所有权限"),
            Role::new("editor")
                .extends("viewer")
                .grant_str("write", "db:test/*")
                .grant_str("write", "db:staging/*")
                .grant_str("write", "flow:*")
                .grant_str("execute", "flow:*")
                .grant_str("delete", "db:test/*")
                .with_description("编辑者，可读写测试/预发布数据和流程"),
            Role::new("viewer")
                .grant_str("read", "db:*")
                .grant_str("read", "flow:*")
                .grant_str("read", "mem:*")
                .grant_str("execute", "flow:readonly/*")
                .with_description("只读用户，可查看所有数据但不能修改"),
            Role::new("safety_approver")
                .grant_str("admin", "db:prod/*")
                .grant_str("write", "flow:gov-pii/*")
                .grant_str("execute", "flow:gov-pii/*")
                .with_description("安全审批员，可审批生产环境写操作"),
            Role::new("operator")
                .grant_str("read", "*")
                .grant_str("execute", "flow:*")
                .with_description("运维人员，可查看所有资源和执行流程"),
            Role::new("auditor")
                .grant_str("read", "db:*")
                .grant_str("read", "flow:*")
                .grant_str("read", "mem:*")
                .grant_str("read", "audit:*")
                .with_description("审计员，可查看审计日志和所有数据"),
        ]
    }
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Action 测试 ──

    #[test]
    fn action_from_str_variants() {
        assert_eq!(Action::from_str("read"), Action::Read);
        assert_eq!(Action::from_str("WRITE"), Action::Write);
        assert_eq!(Action::from_str("Execute"), Action::Execute);
        assert_eq!(Action::from_str("delete"), Action::Delete);
        assert_eq!(Action::from_str("admin"), Action::Admin);
        assert_eq!(Action::from_str("*"), Action::All);
        assert_eq!(Action::from_str("all"), Action::All);
        assert_eq!(
            Action::from_str("custom_action"),
            Action::Custom("custom_action".into())
        );
    }

    #[test]
    fn action_matches_rules() {
        // All 匹配任何
        assert!(Action::All.matches(&Action::Read));
        assert!(Action::All.matches(&Action::Custom("x".into())));
        // Admin 匹配任何非 All
        assert!(Action::Admin.matches(&Action::Read));
        assert!(Action::Admin.matches(&Action::Write));
        assert!(Action::Admin.matches(&Action::Delete));
        // 相同匹配
        assert!(Action::Read.matches(&Action::Read));
        assert!(!Action::Read.matches(&Action::Write));
        // Custom 匹配
        assert!(Action::Custom("approve".into()).matches(&Action::Custom("approve".into())));
        assert!(!Action::Custom("approve".into()).matches(&Action::Custom("reject".into())));
    }

    // ── Permission 测试 ──

    #[test]
    fn permission_wildcard_match_exact() {
        let p = Permission::from_str("read", "db:prod/citizen");
        assert!(p.matches(&Action::Read, "db:prod/citizen"));
        assert!(!p.matches(&Action::Write, "db:prod/citizen"));
        assert!(!p.matches(&Action::Read, "db:prod/other"));
    }

    #[test]
    fn permission_wildcard_match_prefix() {
        let p = Permission::from_str("write", "db:prod/*");
        assert!(p.matches(&Action::Write, "db:prod/citizen_info"));
        assert!(p.matches(&Action::Write, "db:prod/citizen"));
        assert!(!p.matches(&Action::Write, "db:prodx/citizen"));
        assert!(!p.matches(&Action::Write, "db:prod"));
    }

    #[test]
    fn permission_wildcard_match_everything() {
        let p = Permission::from_str("admin", "*");
        assert!(p.matches(&Action::Read, "anything/at/all"));
        assert!(p.matches(&Action::Write, "db:prod/secret"));
    }

    #[test]
    fn permission_admin_action_matches_all() {
        let p = Permission::new(Action::Admin, "db:*");
        assert!(p.matches(&Action::Read, "db:test"));
        assert!(p.matches(&Action::Write, "db:prod"));
        assert!(p.matches(&Action::Delete, "db:anything"));
    }

    // ── Resource 测试 ──

    #[test]
    fn resource_constructors() {
        let r = Resource::new("db:test");
        assert_eq!(r.path, "db:test");
        assert!(r.tenant.is_none());

        let r = Resource::with_tenant("db:prod", "tenant-A");
        assert_eq!(r.path, "db:prod");
        assert_eq!(r.tenant.as_deref(), Some("tenant-A"));
    }

    #[test]
    fn resource_attributes() {
        let r = Resource::new("doc:report")
            .with_attr("owner", "alice")
            .with_attr("confidential", true)
            .with_attr("level", 3i64);

        assert_eq!(r.attributes.get("owner").unwrap().as_str(), Some("alice"));
        assert_eq!(r.attributes.get("confidential").unwrap().as_bool(), Some(true));
        assert_eq!(r.attributes.get("level").unwrap().as_int(), Some(3));
    }

    // ── Subject 测试 ──

    #[test]
    fn subject_tenant_parsing() {
        let s = Subject::new("tenant:A:user:alice", vec!["admin".into()]);
        assert_eq!(s.tenant(), Some("A"));

        let s = Subject::new("user:bob", vec![]);
        assert_eq!(s.tenant(), None);

        let s = Subject::new("tenant:org-123:service:flow", vec![]);
        assert_eq!(s.tenant(), Some("org-123"));
    }

    // ── Role 测试 ──

    #[test]
    fn role_builder_pattern() {
        let r = Role::new("test_role")
            .extends("viewer")
            .grant_str("write", "custom/*")
            .with_description("Test role");

        assert_eq!(r.name, "test_role");
        assert_eq!(r.extends, vec!["viewer"]);
        assert_eq!(r.permissions.len(), 1);
        assert_eq!(r.description.as_deref(), Some("Test role"));
    }

    // ── Policy 测试 ──

    #[test]
    fn policy_builder_pattern() {
        let p = Policy::new("p001", "allow-editor-write", Effect::Allow)
            .for_role("editor")
            .on_resource("db:test/*")
            .with_action(Action::Write)
            .with_condition("resource.owner == subject.id")
            .with_priority(50);

        assert_eq!(p.id, "p001");
        assert_eq!(p.effect, Effect::Allow);
        assert_eq!(p.role_patterns, vec!["editor"]);
        assert_eq!(p.resource_patterns, vec!["db:test/*"]);
        assert_eq!(p.actions, vec![Action::Write]);
        assert_eq!(p.condition.as_deref(), Some("resource.owner == subject.id"));
        assert_eq!(p.priority, 50);
    }

    // ── EvaluationResult 测试 ──

    #[test]
    fn evaluation_result_display() {
        let granted = EvaluationResult::Granted {
            matched_policies: vec!["p1".into()],
        };
        assert!(granted.is_granted());
        assert!(format!("{}", granted).contains("Granted"));

        let denied = EvaluationResult::Denied {
            reason: "no permission".into(),
            denied_by_policy: Some("p-deny".into()),
        };
        assert!(denied.is_denied());
        assert_eq!(denied.denied_reason(), Some("no permission"));
        assert!(format!("{}", denied).contains("p-deny"));
    }

    // ── BuiltinRoles 测试 ──

    #[test]
    fn builtin_roles_count() {
        let roles = BuiltinRoles::all();
        assert_eq!(roles.len(), 6);
        let names: Vec<&str> = roles.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"admin"));
        assert!(names.contains(&"editor"));
        assert!(names.contains(&"viewer"));
        assert!(names.contains(&"safety_approver"));
        assert!(names.contains(&"operator"));
        assert!(names.contains(&"auditor"));
    }

    // ── AttributeValue 测试 ──

    #[test]
    fn attribute_value_conversions() {
        let s: AttributeValue = "hello".into();
        assert_eq!(s.as_str(), Some("hello"));

        let i: AttributeValue = 42i64.into();
        assert_eq!(i.as_int(), Some(42));

        let b: AttributeValue = true.into();
        assert_eq!(b.as_bool(), Some(true));

        let l: AttributeValue = vec!["a".into(), "b".into()].into();
        assert_eq!(l.as_list(), Some(&["a".to_string(), "b".to_string()][..]));
    }
}
