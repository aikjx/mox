//! RBAC 权限检查 — 资源级粒度，拒绝含具体缺失路径

use crate::audit::integration::AuditContext;
use crate::audit::AuditResource;
use crate::rbac::policy::BuiltinRoles;
use std::sync::Arc;

/// 资源描述（带可选租户隔离）
#[derive(Debug, Clone)]
pub struct Resource {
    pub path: String,
    pub tenant: Option<String>,
}

impl Resource {
    pub fn new(path: &str) -> Self { Self { path: path.into(), tenant: None } }
    pub fn with_tenant(path: &str, tenant: &str) -> Self {
        Self { path: path.into(), tenant: Some(tenant.into()) }
    }
}

/// 权限检查上下文
#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub principal: String,
    pub roles: Vec<String>,
    pub resource: Resource,
    pub action: String,
    pub session_id: Option<String>,
    pub client_ip: Option<String>,
}

impl PermissionCheck {
    pub fn new(principal: &str, roles: Vec<String>, action: &str, resource: Resource) -> Self {
        Self { principal: principal.into(), roles, resource, action: action.into(), session_id: None, client_ip: None }
    }
    pub fn with_session(mut self, sid: &str) -> Self { self.session_id = Some(sid.into()); self }
    pub fn with_client_ip(mut self, ip: &str) -> Self { self.client_ip = Some(ip.into()); self }
}

/// 权限检查结果
#[derive(Debug, Clone)]
pub enum PermissionResult {
    Granted,
    Denied(String),
}

impl PermissionResult {
    pub fn is_granted(&self) -> bool { matches!(self, Self::Granted) }
    pub fn denied_reason(&self) -> Option<&str> {
        match self { Self::Granted => None, Self::Denied(r) => Some(r) }
    }
}

// ── 检查核心 ────────────────────────────────────────────────────────────────

pub fn check(ctx: &PermissionCheck) -> PermissionResult {
    if ctx.roles.is_empty() {
        return PermissionResult::Denied("no roles assigned".into());
    }

    let policy = crate::rbac::policy::POLICY.read().unwrap();
    let mut all_perms = Vec::new();
    for role in &ctx.roles {
        all_perms.extend(BuiltinRoles::resolve(&policy, role));
    }

    let mut seen = std::collections::HashSet::new();
    all_perms.retain(|p| seen.insert(format!("{}:{}", p.action, p.resource)));

    // 跨租户隔离
    if let Some(resource_tenant) = &ctx.resource.tenant {
        if let Some(p_tenant) = ctx.principal.strip_prefix("tenant:") {
            let has_admin = all_perms.iter().any(|p| p.action == "admin");
            if !has_admin && p_tenant != resource_tenant {
                return PermissionResult::Denied(format!(
                    "cross-tenant access denied: principal '{}' tenant '{}' != resource tenant '{}'",
                    ctx.principal, p_tenant, resource_tenant
                ));
            }
        }
    }

    let matched = all_perms.iter().find(|p| {
        p.action == "admin" || p.action == "*" || p.matches(&ctx.action, &ctx.resource.path)
    });

    match matched {
        Some(_) => PermissionResult::Granted,
        None => PermissionResult::Denied(format!(
            "role(s) '{}' lacks permission {}:{}",
            ctx.roles.join(", "), ctx.action, ctx.resource.path
        )),
    }
}

pub fn check_with_audit(
    ctx: &PermissionCheck,
    audit_ctx: Option<&Arc<AuditContext>>,
) -> PermissionResult {
    let result = check(ctx);

    if !result.is_granted() {
        if let Some(audit) = audit_ctx {
            let reason = result.denied_reason().unwrap_or("unknown");
            let tenant = ctx.resource.tenant.as_deref().unwrap_or("default");
            let resource = AuditResource::flow(&ctx.resource.path, tenant);
            let _ = audit.log_rbac_denied(resource, tenant, reason, &format!("{}:{}", ctx.action, ctx.resource.path));
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewer_cannot_write_prod() {
        let ctx = PermissionCheck::new(
            "user:alice", vec!["viewer".into()], "write", Resource::new("db:prod/citizen_info")
        );
        let r = check(&ctx);
        assert!(!r.is_granted());
        assert!(r.denied_reason().unwrap().contains("viewer"));
    }

    #[test]
    fn editor_can_write_test() {
        let ctx = PermissionCheck::new(
            "user:bob", vec!["editor".into()], "write", Resource::new("db:test/citizen_info")
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn admin_can_do_anything() {
        let ctx = PermissionCheck::new(
            "user:charlie", vec!["admin".into()], "write", Resource::new("db:prod/anything")
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn safety_approver_prod_write() {
        let ctx = PermissionCheck::new(
            "user:david", vec!["safety_approver".into()], "write", Resource::new("db:prod/citizen_info")
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn empty_roles_denied() {
        let ctx = PermissionCheck::new("user:frank", vec![], "read", Resource::new("db:test"));
        assert!(!check(&ctx).is_granted());
    }

    #[test]
    fn cross_tenant_denied() {
        let ctx = PermissionCheck::new(
            "tenant:A:user:alice",
            vec!["editor".into()],
            "write",
            Resource::with_tenant("db:prod/data", "tenant:B"),
        );
        let r = check(&ctx);
        assert!(!r.is_granted());
        assert!(r.denied_reason().unwrap().contains("cross-tenant"));
    }

    #[test]
    fn wildcard_everything() {
        let ctx = PermissionCheck::new(
            "user:eve", vec!["admin".into()], "execute", Resource::new("flow:anything")
        );
        assert!(check(&ctx).is_granted());
    }
}
