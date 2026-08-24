//! RBAC 策略定义 — 角色 + 资源权限矩阵

use std::collections::HashMap;

/// 权限：操作 + 资源路径（支持通配符）
/// 示例：`write:db:prod/*`、`read:db:*`、`execute:flow:*`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Permission {
    pub action: String,   // read | write | execute | admin
    pub resource: String, // 支持 * 通配符，如 db:prod/*, flow:*
}

impl Permission {
    pub fn new(action: &str, resource: &str) -> Self {
        Self {
            action: action.into(),
            resource: resource.into(),
        }
    }

    /// 资源路径通配符匹配：db:prod/* 匹配 db:prod/citizen_info
    pub fn matches(&self, action: &str, resource: &str) -> bool {
        (self.action == action || self.action == "*" || self.action == "admin")
            && self.wildcard_match(resource)
    }

    fn wildcard_match(&self, resource: &str) -> bool {
        let pattern = &self.resource;
        if pattern == "*" {
            return true;
        }
        // 尾缀通配：db:prod/* 匹配 db:prod/a、db:prod/a/b
        if let Some(stripped) = pattern.strip_suffix("/*") {
            return resource.starts_with(stripped)
                && resource.as_bytes().get(stripped.len()) == Some(&b'/');
        }
        // 精确匹配
        pattern == resource
    }
}

/// 角色定义
#[derive(Debug, Clone)]
pub struct RoleDef {
    pub name: String,
    /// 继承的父角色名
    pub extends: Vec<String>,
    /// 直接授予的权限
    pub grants: Vec<Permission>,
}

impl RoleDef {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            extends: Vec::new(),
            grants: Vec::new(),
        }
    }
    pub fn extends(mut self, parent: &str) -> Self {
        self.extends.push(parent.into());
        self
    }
    pub fn grant(mut self, action: &str, resource: &str) -> Self {
        self.grants.push(Permission::new(action, resource));
        self
    }
}

/// 内置角色定义
pub struct BuiltinRoles;

impl BuiltinRoles {
    /// 构建完整继承链：返回角色名 → 展开后所有权限
    pub fn resolve(policy: &RbacPolicy, role: &str) -> Vec<Permission> {
        let mut visited = std::collections::HashSet::new();
        Self::resolve_impl(policy, role, &mut visited)
    }

    fn resolve_impl(
        policy: &RbacPolicy,
        role: &str,
        visited: &mut std::collections::HashSet<String>,
    ) -> Vec<Permission> {
        if visited.contains(role) {
            return Vec::new();
        } // 防止循环继承
        visited.insert(role.into());

        let mut perms = Vec::new();
        if let Some(def) = policy.roles.get(role) {
            // 父角色权限先
            for parent in &def.extends {
                perms.extend(Self::resolve_impl(policy, parent, visited));
            }
            // 自身权限
            perms.extend(def.grants.clone());
        }
        perms
    }
}

/// RBAC 策略容器
#[derive(Debug, Clone)]
pub struct RbacPolicy {
    /// 角色名 → 角色定义
    pub roles: HashMap<String, RoleDef>,
}

impl RbacPolicy {
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }
    pub fn with_roles(mut self, roles: Vec<RoleDef>) -> Self {
        for r in roles {
            self.roles.insert(r.name.clone(), r);
        }
        self
    }
}

impl Default for RbacPolicy {
    fn default() -> Self {
        Self::new().with_roles(vec![
            // admin 继承 editor 继承 viewer
            RoleDef::new("admin").extends("editor").grant("admin", "*"),
            RoleDef::new("editor")
                .extends("viewer")
                .grant("write", "db:test/*")
                .grant("write", "db:staging/*")
                .grant("write", "flow:*")
                .grant("execute", "flow:*"),
            RoleDef::new("viewer")
                .grant("read", "db:*")
                .grant("read", "flow:*")
                .grant("read", "mem:*")
                .grant("execute", "flow:readonly/*"),
            RoleDef::new("safety_approver")
                .grant("admin", "db:prod/*") // 仅审批写生产
                .grant("write", "flow:gov-pii/*")
                .grant("execute", "flow:gov-pii/*"),
            RoleDef::new("operator")
                .grant("read", "*")
                .grant("execute", "flow:*"),
            RoleDef::new("auditor")
                .grant("read", "db:*")
                .grant("read", "flow:*")
                .grant("read", "mem:*")
                .grant("read", "audit:*"),
        ])
    }
}

/// 全局默认策略（LazyLock 保证只初始化一次）
pub static POLICY: std::sync::LazyLock<std::sync::RwLock<RbacPolicy>> =
    std::sync::LazyLock::new(|| std::sync::RwLock::new(RbacPolicy::default()));

#[cfg(test)]
mod tests {
    use super::*;

    fn make_policy() -> RbacPolicy {
        RbacPolicy::default()
    }

    #[test]
    fn wildcard_match_exact() {
        let p = Permission::new("read", "db:prod/citizen");
        assert!(p.matches("read", "db:prod/citizen"));
        assert!(!p.matches("write", "db:prod/citizen"));
        assert!(!p.matches("read", "db:prod/other"));
    }

    #[test]
    fn wildcard_match_prefix() {
        let p = Permission::new("write", "db:prod/*");
        assert!(p.matches("write", "db:prod/citizen_info"));
        assert!(p.matches("write", "db:prod/citizen"));
        assert!(!p.matches("write", "db:prodx/citizen")); // 不同顶级
        assert!(!p.matches("write", "db:prod")); // 精确路径不匹配通配
    }

    #[test]
    fn role_inheritance() {
        let pol = make_policy();
        let viewer_perms = BuiltinRoles::resolve(&pol, "viewer");
        let editor_perms = BuiltinRoles::resolve(&pol, "editor");
        assert!(editor_perms.len() >= viewer_perms.len());
        assert!(BuiltinRoles::resolve(&pol, "admin").len() >= editor_perms.len());
    }

    #[test]
    fn safety_approver_prod_write() {
        let pol = make_policy();
        let perms = BuiltinRoles::resolve(&pol, "safety_approver");
        assert!(perms.iter().any(|p| p.matches("write", "db:prod/citizen")));
        assert!(perms.iter().any(|p| p.matches("admin", "db:prod/*")));
    }
}
