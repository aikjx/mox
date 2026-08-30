// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 数据权限管理
//!
//! 支持行级数据权限控制，通过数据范围和过滤规则限制用户可见的数据。

use parking_lot::RwLock;
use std::collections::HashMap;

use crate::error::{PermError, PermResult};
use crate::types::{Subject, now_ms};
use uuid::Uuid;

/// 数据范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataScope {
    /// 全部数据
    All,
    /// 本部门及以下
    DeptAndSub,
    /// 仅本部门
    DeptOnly,
    /// 本人创建
    SelfOnly,
    /// 自定义（通过过滤规则）
    Custom,
}

impl DataScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            DataScope::All => "all",
            DataScope::DeptAndSub => "dept_and_sub",
            DataScope::DeptOnly => "dept_only",
            DataScope::SelfOnly => "self_only",
            DataScope::Custom => "custom",
        }
    }

    /// 从字符串解析
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(DataScope::All),
            "dept_and_sub" => Some(DataScope::DeptAndSub),
            "dept_only" => Some(DataScope::DeptOnly),
            "self_only" => Some(DataScope::SelfOnly),
            "custom" => Some(DataScope::Custom),
            _ => None,
        }
    }

    /// 检查是否包含更严格的范围
    pub fn includes(&self, other: &DataScope) -> bool {
        // 范围从大到小：All > DeptAndSub > DeptOnly > SelfOnly > Custom
        let order = |scope: &DataScope| match scope {
            DataScope::All => 4,
            DataScope::DeptAndSub => 3,
            DataScope::DeptOnly => 2,
            DataScope::SelfOnly => 1,
            DataScope::Custom => 0,
        };
        order(self) >= order(other)
    }
}

/// 过滤操作符
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 大于
    Gt,
    /// 大于等于
    Gte,
    /// 小于
    Lt,
    /// 小于等于
    Lte,
    /// 包含
    In,
    /// 不包含
    NotIn,
    /// 模糊匹配
    Like,
    /// 区间
    Between,
    /// 为空
    IsNull,
    /// 不为空
    IsNotNull,
}

/// 过滤值类型
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FilterValue {
    /// 字符串值
    String(String),
    /// 数值
    Number(f64),
    /// 布尔
    Bool(bool),
    /// 列表
    List(Vec<String>),
    /// 动态值（运行时解析）
    Dynamic(String), // 如 "user.id", "user.dept_id", "now()"
}

/// 数据过滤规则
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataFilterRule {
    /// 规则 ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 资源类型
    pub resource_type: String,
    /// 字段名
    pub field: String,
    /// 操作符
    pub operator: FilterOperator,
    /// 过滤值
    pub value: FilterValue,
    /// 逻辑连接（与上一条规则的关系）
    pub logic: FilterLogic,
    /// 租户 ID
    pub tenant_id: String,
    /// 描述
    pub description: Option<String>,
    /// 创建时间
    pub created_at: u64,
}

/// 过滤逻辑连接
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterLogic {
    /// 逻辑与
    And,
    /// 逻辑或
    Or,
}

impl DataFilterRule {
    /// 创建新规则
    pub fn new(
        name: &str,
        resource_type: &str,
        field: &str,
        operator: FilterOperator,
        value: FilterValue,
        tenant_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            resource_type: resource_type.to_string(),
            field: field.to_string(),
            operator,
            value,
            logic: FilterLogic::And,
            tenant_id: tenant_id.to_string(),
            description: None,
            created_at: now_ms(),
        }
    }

    /// 设置逻辑连接
    pub fn with_logic(mut self, logic: FilterLogic) -> Self {
        self.logic = logic;
        self
    }
}

/// 角色数据权限配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RoleDataPermission {
    /// 角色 ID
    pub role_id: String,
    /// 资源类型
    pub resource_type: String,
    /// 数据范围
    pub scope: DataScope,
    /// 自定义过滤规则 ID 列表
    pub filter_rule_ids: Vec<String>,
    /// 租户 ID
    pub tenant_id: String,
}

impl RoleDataPermission {
    pub fn new(role_id: &str, resource_type: &str, scope: DataScope, tenant_id: &str) -> Self {
        Self {
            role_id: role_id.to_string(),
            resource_type: resource_type.to_string(),
            scope,
            filter_rule_ids: Vec::new(),
            tenant_id: tenant_id.to_string(),
        }
    }
}

/// 数据权限管理器
pub struct DataPermissionManager {
    /// 过滤规则表
    rules: RwLock<HashMap<String, DataFilterRule>>,
    /// 角色数据权限："role_id:resource_type" -> RoleDataPermission
    role_permissions: RwLock<HashMap<String, RoleDataPermission>>,
}

impl DataPermissionManager {
    /// 创建数据权限管理器
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(HashMap::new()),
            role_permissions: RwLock::new(HashMap::new()),
        }
    }

    // ---------- 过滤规则 ----------

    /// 创建过滤规则
    pub fn create_rule(&self, rule: DataFilterRule) -> PermResult<DataFilterRule> {
        self.rules
            .write()
            .insert(rule.id.clone(), rule.clone());
        Ok(rule)
    }

    /// 获取过滤规则
    pub fn get_rule(&self, rule_id: &str) -> PermResult<DataFilterRule> {
        self.rules
            .read()
            .get(rule_id)
            .cloned()
            .ok_or_else(|| PermError::NotFound(format!("filter rule '{}' not found", rule_id)))
    }

    /// 列出资源类型的所有规则
    pub fn list_rules(&self, tenant_id: &str, resource_type: &str) -> Vec<DataFilterRule> {
        self.rules
            .read()
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.resource_type == resource_type)
            .cloned()
            .collect()
    }

    /// 删除过滤规则
    pub fn delete_rule(&self, rule_id: &str) -> PermResult<bool> {
        // 检查是否被角色引用
        for rp in self.role_permissions.read().values() {
            if rp.filter_rule_ids.iter().any(|id| id == rule_id) {
                return Err(PermError::InvalidArgument(format!(
                    "rule '{}' is used by role data permission",
                    rule_id
                )));
            }
        }
        Ok(self.rules.write().remove(rule_id).is_some())
    }

    // ---------- 角色数据权限 ----------

    /// 设置角色数据权限
    pub fn set_role_permission(&self, perm: RoleDataPermission) {
        let key = role_perm_key(&perm.role_id, &perm.resource_type);
        self.role_permissions.write().insert(key, perm);
    }

    /// 获取角色数据权限
    pub fn get_role_permission(
        &self,
        role_id: &str,
        resource_type: &str,
    ) -> Option<RoleDataPermission> {
        let key = role_perm_key(role_id, resource_type);
        self.role_permissions.read().get(&key).cloned()
    }

    /// 获取主体在某资源上的数据范围（取最宽松的范围）
    pub fn get_data_scope(
        &self,
        role_ids: &[String],
        resource_type: &str,
    ) -> DataScope {
        let mut max_scope = DataScope::SelfOnly;

        for role_id in role_ids {
            if let Some(perm) = self.get_role_permission(role_id, resource_type) {
                if perm.scope.includes(&max_scope) {
                    max_scope = perm.scope;
                }
            }
        }

        max_scope
    }

    /// 获取主体的所有有效过滤规则
    pub fn get_effective_filters(
        &self,
        role_ids: &[String],
        resource_type: &str,
    ) -> Vec<DataFilterRule> {
        let mut rule_ids = Vec::new();
        let perms = self.role_permissions.read();

        for role_id in role_ids {
            let key = role_perm_key(role_id, resource_type);
            if let Some(perm) = perms.get(&key) {
                rule_ids.extend(perm.filter_rule_ids.clone());
            }
        }

        // 去重并获取规则详情
        let rules = self.rules.read();
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        for rule_id in rule_ids {
            if seen.insert(rule_id.clone()) {
                if let Some(rule) = rules.get(&rule_id) {
                    result.push(rule.clone());
                }
            }
        }

        result
    }

    /// 生成数据过滤的 SQL 条件片段（简化版）
    pub fn build_filter_sql(
        &self,
        role_ids: &[String],
        resource_type: &str,
        subject: &Subject,
    ) -> Option<String> {
        let scope = self.get_data_scope(role_ids, resource_type);
        let rules = self.get_effective_filters(role_ids, resource_type);

        let mut conditions = Vec::new();

        // 基于 scope 生成条件
        match scope {
            DataScope::All => {
                // 全部可见，不加条件
                return None;
            }
            DataScope::SelfOnly => {
                conditions.push(format!("created_by = '{}'", subject.id));
            }
            DataScope::DeptOnly => {
                if let Some(dept_id) = subject.get_attr("dept_id") {
                    conditions.push(format!("dept_id = '{}'", dept_id));
                }
            }
            DataScope::DeptAndSub => {
                if let Some(dept_path) = subject.get_attr("dept_path") {
                    conditions.push(format!("dept_path LIKE '{}%'", dept_path));
                }
            }
            DataScope::Custom => {
                // 仅使用自定义规则
            }
        }

        // 添加自定义过滤规则
        for rule in &rules {
            let cond = format_rule_condition(rule, subject);
            conditions.push(cond);
        }

        if conditions.is_empty() {
            None
        } else {
            Some(conditions.join(" AND "))
        }
    }

    /// 规则总数
    pub fn rule_count(&self) -> usize {
        self.rules.read().len()
    }
}

impl Default for DataPermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成角色权限键
fn role_perm_key(role_id: &str, resource_type: &str) -> String {
    format!("{}:{}", role_id, resource_type)
}

/// 格式化规则为 SQL 条件
fn format_rule_condition(rule: &DataFilterRule, subject: &Subject) -> String {
    use FilterOperator::*;

    let value_str = match &rule.value {
        FilterValue::String(s) => format!("'{}'", s),
        FilterValue::Number(n) => n.to_string(),
        FilterValue::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        FilterValue::List(list) => format!(
            "({})",
            list.iter()
                .map(|s| format!("'{}'", s))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        FilterValue::Dynamic(expr) => resolve_dynamic_value(expr, subject),
    };

    match rule.operator {
        Eq => format!("{} = {}", rule.field, value_str),
        Ne => format!("{} != {}", rule.field, value_str),
        Gt => format!("{} > {}", rule.field, value_str),
        Gte => format!("{} >= {}", rule.field, value_str),
        Lt => format!("{} < {}", rule.field, value_str),
        Lte => format!("{} <= {}", rule.field, value_str),
        In => format!("{} IN {}", rule.field, value_str),
        NotIn => format!("{} NOT IN {}", rule.field, value_str),
        Like => format!("{} LIKE {}", rule.field, value_str),
        Between => format!("{} BETWEEN ...", rule.field), // 简化
        IsNull => format!("{} IS NULL", rule.field),
        IsNotNull => format!("{} IS NOT NULL", rule.field),
    }
}

/// 解析动态值
fn resolve_dynamic_value(expr: &str, subject: &Subject) -> String {
    match expr {
        "user.id" => format!("'{}'", subject.id),
        "user.tenant_id" => format!("'{}'", subject.tenant_id),
        other => {
            // 尝试从属性中查找
            if let Some(val) = subject.get_attr(other) {
                format!("'{}'", val)
            } else {
                format!("'{}'", other)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_scope_includes() {
        assert!(DataScope::All.includes(&DataScope::DeptAndSub));
        assert!(DataScope::DeptAndSub.includes(&DataScope::DeptOnly));
        assert!(DataScope::DeptOnly.includes(&DataScope::SelfOnly));
        assert!(!DataScope::SelfOnly.includes(&DataScope::All));
    }

    #[test]
    fn test_data_scope_parse() {
        assert_eq!(DataScope::parse("all"), Some(DataScope::All));
        assert_eq!(DataScope::parse("self_only"), Some(DataScope::SelfOnly));
        assert_eq!(DataScope::parse("invalid"), None);
    }

    #[test]
    fn test_create_and_get_rule() {
        let mgr = DataPermissionManager::new();
        let rule = DataFilterRule::new(
            "test_rule",
            "document",
            "status",
            FilterOperator::Eq,
            FilterValue::String("published".to_string()),
            "t-1",
        );

        let created = mgr.create_rule(rule.clone()).unwrap();
        assert_eq!(created.name, "test_rule");
        assert_eq!(mgr.rule_count(), 1);

        let got = mgr.get_rule(&created.id).unwrap();
        assert_eq!(got.field, "status");
    }

    #[test]
    fn test_role_data_permission() {
        let mgr = DataPermissionManager::new();
        let perm = RoleDataPermission::new("role-1", "document", DataScope::DeptOnly, "t-1");
        mgr.set_role_permission(perm);

        let got = mgr.get_role_permission("role-1", "document").unwrap();
        assert_eq!(got.scope, DataScope::DeptOnly);
    }

    #[test]
    fn test_get_data_scope_takes_max() {
        let mgr = DataPermissionManager::new();

        mgr.set_role_permission(RoleDataPermission::new(
            "role-1", "doc", DataScope::SelfOnly, "t-1",
        ));
        mgr.set_role_permission(RoleDataPermission::new(
            "role-2", "doc", DataScope::DeptAndSub, "t-1",
        ));

        let scope = mgr.get_data_scope(&["role-1".to_string(), "role-2".to_string()], "doc");
        assert_eq!(scope, DataScope::DeptAndSub);
    }

    #[test]
    fn test_build_filter_sql_self_only() {
        let mgr = DataPermissionManager::new();
        let subject = Subject::user("user-1", "t-1");

        mgr.set_role_permission(RoleDataPermission::new(
            "role-1", "document", DataScope::SelfOnly, "t-1",
        ));

        let sql = mgr.build_filter_sql(&["role-1".to_string()], "document", &subject);
        assert!(sql.is_some());
        assert!(sql.unwrap().contains("created_by = 'user-1'"));
    }

    #[test]
    fn test_build_filter_sql_all_returns_none() {
        let mgr = DataPermissionManager::new();
        let subject = Subject::user("user-1", "t-1");

        mgr.set_role_permission(RoleDataPermission::new(
            "role-1", "document", DataScope::All, "t-1",
        ));

        let sql = mgr.build_filter_sql(&["role-1".to_string()], "document", &subject);
        assert!(sql.is_none());
    }

    #[test]
    fn test_custom_filter_rules() {
        let mgr = DataPermissionManager::new();
        let subject = Subject::user("user-1", "t-1");

        let rule = DataFilterRule::new(
            "only_active",
            "document",
            "status",
            FilterOperator::Eq,
            FilterValue::String("active".to_string()),
            "t-1",
        );
        let rule = mgr.create_rule(rule).unwrap();

        let mut perm = RoleDataPermission::new("role-1", "document", DataScope::Custom, "t-1");
        perm.filter_rule_ids.push(rule.id.clone());
        mgr.set_role_permission(perm);

        let sql = mgr.build_filter_sql(&["role-1".to_string()], "document", &subject);
        assert!(sql.is_some());
        assert!(sql.unwrap().contains("status = 'active'"));
    }

    #[test]
    fn test_dynamic_filter_value() {
        let mgr = DataPermissionManager::new();
        let mut subject = Subject::user("user-1", "t-1");
        subject
            .attributes
            .insert("dept_id".to_string(), "dept-123".to_string());

        let rule = DataFilterRule::new(
            "dept_filter",
            "document",
            "department",
            FilterOperator::Eq,
            FilterValue::Dynamic("dept_id".to_string()),
            "t-1",
        );
        let rule = mgr.create_rule(rule).unwrap();

        let mut perm = RoleDataPermission::new("role-1", "document", DataScope::Custom, "t-1");
        perm.filter_rule_ids.push(rule.id.clone());
        mgr.set_role_permission(perm);

        let sql = mgr.build_filter_sql(&["role-1".to_string()], "document", &subject);
        assert!(sql.unwrap().contains("'dept-123'"));
    }
}
