// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! ABAC（基于属性的访问控制）引擎
//!
//! 支持基于主体属性、资源属性、环境属性的动态策略评估。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::PermResult;
use crate::types::{Action, PermissionEffect, ResourceScope, Subject};

/// 属性值
#[derive(Debug, Clone, PartialEq)]
pub enum AttributeValue {
    /// 字符串
    String(String),
    /// 整数
    Int(i64),
    /// 布尔
    Bool(bool),
    /// 字符串列表
    StringList(Vec<String>),
    /// 空
    Null,
}

impl AttributeValue {
    /// 转换为字符串
    pub fn as_str(&self) -> Option<&str> {
        match self {
            AttributeValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// 转换为整数
    pub fn as_int(&self) -> Option<i64> {
        match self {
            AttributeValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// 转换为布尔
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttributeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Self {
        if let Ok(i) = s.parse::<i64>() {
            return AttributeValue::Int(i);
        }
        if s == "true" {
            return AttributeValue::Bool(true);
        }
        if s == "false" {
            return AttributeValue::Bool(false);
        }
        if s == "null" {
            return AttributeValue::Null;
        }
        AttributeValue::String(s.to_string())
    }
}

impl From<&str> for AttributeValue {
    fn from(s: &str) -> Self {
        AttributeValue::String(s.to_string())
    }
}

impl From<String> for AttributeValue {
    fn from(s: String) -> Self {
        AttributeValue::String(s)
    }
}

impl From<i64> for AttributeValue {
    fn from(i: i64) -> Self {
        AttributeValue::Int(i)
    }
}

impl From<bool> for AttributeValue {
    fn from(b: bool) -> Self {
        AttributeValue::Bool(b)
    }
}

/// 属性上下文（用于策略评估）
#[derive(Debug, Clone, Default)]
pub struct AttributeContext {
    /// 主体属性
    pub subject: HashMap<String, AttributeValue>,
    /// 资源属性
    pub resource: HashMap<String, AttributeValue>,
    /// 操作属性
    pub action: HashMap<String, AttributeValue>,
    /// 环境属性（时间、IP、设备等）
    pub environment: HashMap<String, AttributeValue>,
}

impl AttributeContext {
    /// 创建空上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 Subject 构建
    pub fn from_subject(subject: &Subject) -> Self {
        let mut ctx = Self::default();
        ctx.subject
            .insert("id".to_string(), AttributeValue::String(subject.id.clone()));
        ctx.subject.insert(
            "tenant_id".to_string(),
            AttributeValue::String(subject.tenant_id.clone()),
        );
        for (k, v) in &subject.attributes {
            ctx.subject
                .insert(k.clone(), AttributeValue::String(v.clone()));
        }
        ctx
    }

    /// 从 ResourceScope 构建
    pub fn with_resource(mut self, resource: &ResourceScope) -> Self {
        self.resource.insert(
            "type".to_string(),
            AttributeValue::String(resource.resource_type.clone()),
        );
        if let Some(id) = &resource.resource_id {
            self.resource
                .insert("id".to_string(), AttributeValue::String(id.clone()));
        }
        for (k, v) in &resource.attributes {
            self.resource
                .insert(k.clone(), AttributeValue::String(v.clone()));
        }
        self
    }

    /// 设置环境属性
    pub fn with_env(mut self, key: &str, value: AttributeValue) -> Self {
        self.environment.insert(key.to_string(), value);
        self
    }

    /// 获取属性值（支持点路径：subject.department）
    pub fn get(&self, path: &str) -> Option<&AttributeValue> {
        let parts: Vec<&str> = path.splitn(2, '.').collect();
        if parts.len() != 2 {
            return None;
        }

        let map = match parts[0] {
            "subject" => &self.subject,
            "resource" => &self.resource,
            "action" => &self.action,
            "environment" | "env" => &self.environment,
            _ => return None,
        };

        map.get(parts[1])
    }
}

/// 比较操作符
#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Ge,
    /// 小于
    Lt,
    /// 小于等于
    Le,
    /// 包含
    In,
    /// 不包含
    NotIn,
    /// 匹配（字符串前缀/后缀）
    StartsWith,
    EndsWith,
    ContainsStr,
    /// 正则匹配
    Regex,
}

impl CompareOp {
    /// 从字符串解析
    pub fn parse(op: &str) -> Option<Self> {
        match op {
            "eq" | "==" => Some(CompareOp::Eq),
            "ne" | "!=" => Some(CompareOp::Ne),
            "gt" | ">" => Some(CompareOp::Gt),
            "ge" | ">=" => Some(CompareOp::Ge),
            "lt" | "<" => Some(CompareOp::Lt),
            "le" | "<=" => Some(CompareOp::Le),
            "in" => Some(CompareOp::In),
            "not_in" => Some(CompareOp::NotIn),
            "starts_with" => Some(CompareOp::StartsWith),
            "ends_with" => Some(CompareOp::EndsWith),
            "contains" => Some(CompareOp::ContainsStr),
            "regex" => Some(CompareOp::Regex),
            _ => None,
        }
    }
}

/// 条件表达式（简单的表达式树）
#[derive(Debug, Clone)]
pub enum ConditionExpr {
    /// 比较：属性 op 值
    Compare {
        attr_path: String,
        op: CompareOp,
        value: AttributeValue,
    },
    /// 逻辑与
    And(Vec<ConditionExpr>),
    /// 逻辑或
    Or(Vec<ConditionExpr>),
    /// 逻辑非
    Not(Box<ConditionExpr>),
    /// 常量
    Constant(bool),
}

impl ConditionExpr {
    /// 评估表达式
    pub fn evaluate(&self, ctx: &AttributeContext) -> PermResult<bool> {
        match self {
            ConditionExpr::Constant(val) => Ok(*val),
            ConditionExpr::Not(expr) => Ok(!expr.evaluate(ctx)?),
            ConditionExpr::And(exprs) => {
                for e in exprs {
                    if !e.evaluate(ctx)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ConditionExpr::Or(exprs) => {
                for e in exprs {
                    if e.evaluate(ctx)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ConditionExpr::Compare { attr_path, op, value } => {
                let attr_val = ctx.get(attr_path);
                Self::compare(attr_val, op, value)
            }
        }
    }

    fn compare(
        attr_val: Option<&AttributeValue>,
        op: &CompareOp,
        target: &AttributeValue,
    ) -> PermResult<bool> {
        use CompareOp::*;

        let attr_val = match attr_val {
            Some(v) => v,
            None => {
                // 属性不存在时，只有 Ne 和 NotIn 返回 true
                return Ok(matches!(op, Ne | NotIn));
            }
        };

        match op {
            Eq => Ok(attr_val == target),
            Ne => Ok(attr_val != target),
            Gt => {
                if let (Some(a), Some(b)) = (attr_val.as_int(), target.as_int()) {
                    Ok(a > b)
                } else {
                    Ok(false)
                }
            }
            Ge => {
                if let (Some(a), Some(b)) = (attr_val.as_int(), target.as_int()) {
                    Ok(a >= b)
                } else {
                    Ok(false)
                }
            }
            Lt => {
                if let (Some(a), Some(b)) = (attr_val.as_int(), target.as_int()) {
                    Ok(a < b)
                } else {
                    Ok(false)
                }
            }
            Le => {
                if let (Some(a), Some(b)) = (attr_val.as_int(), target.as_int()) {
                    Ok(a <= b)
                } else {
                    Ok(false)
                }
            }
            In => {
                if let AttributeValue::StringList(list) = target {
                    if let Some(s) = attr_val.as_str() {
                        Ok(list.iter().any(|item| item == s))
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(false)
                }
            }
            NotIn => {
                if let AttributeValue::StringList(list) = target {
                    if let Some(s) = attr_val.as_str() {
                        Ok(!list.iter().any(|item| item == s))
                    } else {
                        Ok(true)
                    }
                } else {
                    Ok(true)
                }
            }
            StartsWith => {
                if let (Some(a), Some(b)) = (attr_val.as_str(), target.as_str()) {
                    Ok(a.starts_with(b))
                } else {
                    Ok(false)
                }
            }
            EndsWith => {
                if let (Some(a), Some(b)) = (attr_val.as_str(), target.as_str()) {
                    Ok(a.ends_with(b))
                } else {
                    Ok(false)
                }
            }
            ContainsStr => {
                if let (Some(a), Some(b)) = (attr_val.as_str(), target.as_str()) {
                    Ok(a.contains(b))
                } else {
                    Ok(false)
                }
            }
            Regex => {
                if let (Some(a), Some(b)) = (attr_val.as_str(), target.as_str()) {
                    // 简单正则：只支持 * 通配符
                    let pattern = format!("^{}$", regex::escape(b).replace(r"\*", ".*"));
                    Ok(regex::Regex::new(&pattern)
                        .map(|re| re.is_match(a))
                        .unwrap_or(false))
                } else {
                    Ok(false)
                }
            }
        }
    }
}

/// ABAC 策略
#[derive(Debug, Clone)]
pub struct AbacPolicy {
    /// 策略 ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 租户 ID
    pub tenant_id: String,
    /// 效果（允许/拒绝）
    pub effect: PermissionEffect,
    /// 适用的操作
    pub action: Action,
    /// 适用的资源类型
    pub resource_type: String,
    /// 条件表达式
    pub condition: ConditionExpr,
    /// 描述
    pub description: Option<String>,
    /// 优先级（数值越小优先级越高）
    pub priority: u32,
    /// 是否启用
    pub enabled: bool,
}

impl AbacPolicy {
    /// 创建策略
    pub fn new(
        name: &str,
        tenant_id: &str,
        effect: PermissionEffect,
        action: Action,
        resource_type: &str,
        condition: ConditionExpr,
    ) -> Self {
        use uuid::Uuid;
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            tenant_id: tenant_id.to_string(),
            effect,
            action,
            resource_type: resource_type.to_string(),
            condition,
            description: None,
            priority: 100,
            enabled: true,
        }
    }

    /// 检查是否适用
    pub fn applies(&self, action: &Action, resource_type: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if self.resource_type != "*" && self.resource_type != resource_type {
            return false;
        }
        self.action.matches(action)
    }
}

/// ABAC 引擎
pub struct AbacEngine {
    /// 策略表
    policies: RwLock<Vec<AbacPolicy>>,
}

impl AbacEngine {
    /// 创建 ABAC 引擎
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(Vec::new()),
        }
    }

    /// 添加策略
    pub fn add_policy(&self, policy: AbacPolicy) {
        let mut policies = self.policies.write();
        policies.push(policy);
        // 按优先级排序（数字越小越优先）
        policies.sort_by_key(|p| p.priority);
    }

    /// 移除策略
    pub fn remove_policy(&self, policy_id: &str) -> bool {
        let mut policies = self.policies.write();
        let len_before = policies.len();
        policies.retain(|p| p.id != policy_id);
        policies.len() != len_before
    }

    /// 获取策略
    pub fn get_policy(&self, policy_id: &str) -> Option<AbacPolicy> {
        self.policies
            .read()
            .iter()
            .find(|p| p.id == policy_id)
            .cloned()
    }

    /// 列出租户策略
    pub fn list_policies(&self, tenant_id: &str) -> Vec<AbacPolicy> {
        self.policies
            .read()
            .iter()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    /// 评估所有适用策略，返回最终决策
    ///
    /// 评估规则：
    /// 1. 任何明确拒绝（Deny）立即返回拒绝
    /// 2. 至少一个允许（Allow）且无拒绝，则允许
    /// 3. 无适用策略则返回 None（交由上层决策，如 RBAC）
    pub fn evaluate(
        &self,
        action: &Action,
        resource: &ResourceScope,
        ctx: &AttributeContext,
        tenant_id: &str,
    ) -> PermResult<Option<PermissionEffect>> {
        let policies = self.policies.read();
        let mut has_allow = false;

        for policy in policies.iter() {
            if policy.tenant_id != tenant_id {
                continue;
            }
            if !policy.applies(action, &resource.resource_type) {
                continue;
            }

            // 评估条件
            let condition_met = policy.condition.evaluate(ctx)?;

            if condition_met {
                match policy.effect {
                    PermissionEffect::Deny => {
                        // Deny 优先，立即返回
                        return Ok(Some(PermissionEffect::Deny));
                    }
                    PermissionEffect::Allow => {
                        has_allow = true;
                    }
                }
            }
        }

        if has_allow {
            Ok(Some(PermissionEffect::Allow))
        } else {
            Ok(None) // 无适用策略，交由上层
        }
    }

    /// 策略总数
    pub fn policy_count(&self) -> usize {
        self.policies.read().len()
    }
}

impl Default for AbacEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_value_parse() {
        assert_eq!(AttributeValue::parse("42"), AttributeValue::Int(42));
        assert_eq!(AttributeValue::parse("true"), AttributeValue::Bool(true));
        assert_eq!(
            AttributeValue::parse("hello"),
            AttributeValue::String("hello".to_string())
        );
        assert_eq!(AttributeValue::parse("null"), AttributeValue::Null);
    }

    #[test]
    fn test_attribute_context_get() {
        let mut ctx = AttributeContext::new();
        ctx.subject
            .insert("department".to_string(), AttributeValue::String("eng".to_string()));
        ctx.resource
            .insert("sensitivity".to_string(), AttributeValue::String("high".to_string()));

        assert_eq!(
            ctx.get("subject.department").unwrap().as_str(),
            Some("eng")
        );
        assert_eq!(
            ctx.get("resource.sensitivity").unwrap().as_str(),
            Some("high")
        );
        assert!(ctx.get("subject.nonexist").is_none());
        assert!(ctx.get("invalid.path").is_none());
    }

    #[test]
    fn test_condition_compare_eq() {
        let mut ctx = AttributeContext::new();
        ctx.subject
            .insert("role".to_string(), AttributeValue::String("admin".to_string()));

        let expr = ConditionExpr::Compare {
            attr_path: "subject.role".to_string(),
            op: CompareOp::Eq,
            value: AttributeValue::String("admin".to_string()),
        };

        assert!(expr.evaluate(&ctx).unwrap());
    }

    #[test]
    fn test_condition_compare_gt() {
        let mut ctx = AttributeContext::new();
        ctx.environment
            .insert("hour".to_string(), AttributeValue::Int(14));

        let expr = ConditionExpr::Compare {
            attr_path: "environment.hour".to_string(),
            op: CompareOp::Gt,
            value: AttributeValue::Int(9),
        };

        assert!(expr.evaluate(&ctx).unwrap());
    }

    #[test]
    fn test_condition_and_or() {
        let mut ctx = AttributeContext::new();
        ctx.subject
            .insert("dept".to_string(), AttributeValue::String("eng".to_string()));
        ctx.subject
            .insert("level".to_string(), AttributeValue::Int(5));

        let expr = ConditionExpr::And(vec![
            ConditionExpr::Compare {
                attr_path: "subject.dept".to_string(),
                op: CompareOp::Eq,
                value: AttributeValue::String("eng".to_string()),
            },
            ConditionExpr::Compare {
                attr_path: "subject.level".to_string(),
                op: CompareOp::Ge,
                value: AttributeValue::Int(3),
            },
        ]);

        assert!(expr.evaluate(&ctx).unwrap());
    }

    #[test]
    fn test_condition_not() {
        let ctx = AttributeContext::new();
        let expr = ConditionExpr::Not(Box::new(ConditionExpr::Constant(false)));
        assert!(expr.evaluate(&ctx).unwrap());
    }

    #[test]
    fn test_condition_in() {
        let mut ctx = AttributeContext::new();
        ctx.subject.insert(
            "role".to_string(),
            AttributeValue::String("editor".to_string()),
        );

        let expr = ConditionExpr::Compare {
            attr_path: "subject.role".to_string(),
            op: CompareOp::In,
            value: AttributeValue::StringList(vec![
                "admin".to_string(),
                "editor".to_string(),
            ]),
        };

        assert!(expr.evaluate(&ctx).unwrap());
    }

    #[test]
    fn test_abac_engine_allow() {
        let engine = AbacEngine::new();
        let tenant = "t-1";

        // 策略：工程部员工可以读取文档
        let policy = AbacPolicy::new(
            "eng_read_doc",
            tenant,
            PermissionEffect::Allow,
            Action::new("read:*"),
            "document",
            ConditionExpr::Compare {
                attr_path: "subject.department".to_string(),
                op: CompareOp::Eq,
                value: AttributeValue::String("engineering".to_string()),
            },
        );
        engine.add_policy(policy);

        // 工程部员工 -> 允许
        let mut ctx = AttributeContext::new();
        ctx.subject.insert(
            "department".to_string(),
            AttributeValue::String("engineering".to_string()),
        );
        let resource = ResourceScope::all("document");

        let result = engine
            .evaluate(&Action::new("read:doc"), &resource, &ctx, tenant)
            .unwrap();
        assert_eq!(result, Some(PermissionEffect::Allow));

        // 市场部员工 -> 无适用策略（返回 None）
        let mut ctx2 = AttributeContext::new();
        ctx2.subject.insert(
            "department".to_string(),
            AttributeValue::String("marketing".to_string()),
        );
        let result2 = engine
            .evaluate(&Action::new("read:doc"), &resource, &ctx2, tenant)
            .unwrap();
        assert_eq!(result2, None);
    }

    #[test]
    fn test_abac_engine_deny_overrides() {
        let engine = AbacEngine::new();
        let tenant = "t-1";

        // 允许策略
        let allow_policy = AbacPolicy::new(
            "allow_read",
            tenant,
            PermissionEffect::Allow,
            Action::new("read:*"),
            "document",
            ConditionExpr::Constant(true),
        );
        engine.add_policy(allow_policy);

        // 拒绝策略（高优先级）
        let mut deny_policy = AbacPolicy::new(
            "deny_external",
            tenant,
            PermissionEffect::Deny,
            Action::new("read:*"),
            "document",
            ConditionExpr::Compare {
                attr_path: "resource.sensitivity".to_string(),
                op: CompareOp::Eq,
                value: AttributeValue::String("top_secret".to_string()),
            },
        );
        deny_policy.priority = 10; // 更高优先级
        engine.add_policy(deny_policy);

        let mut ctx = AttributeContext::new();
        let mut resource = ResourceScope::all("document");
        resource
            .attributes
            .insert("sensitivity".to_string(), "top_secret".to_string());
        ctx = ctx.with_resource(&resource);

        let result = engine
            .evaluate(&Action::new("read:doc"), &resource, &ctx, tenant)
            .unwrap();
        assert_eq!(result, Some(PermissionEffect::Deny));
    }

    #[test]
    fn test_disabled_policy() {
        let engine = AbacEngine::new();
        let tenant = "t-1";

        let mut policy = AbacPolicy::new(
            "disabled_policy",
            tenant,
            PermissionEffect::Allow,
            Action::all(),
            "*",
            ConditionExpr::Constant(true),
        );
        policy.enabled = false;
        engine.add_policy(policy);

        let ctx = AttributeContext::new();
        let resource = ResourceScope::all("test");
        let result = engine
            .evaluate(&Action::all(), &resource, &ctx, tenant)
            .unwrap();
        assert_eq!(result, None);
    }
}
