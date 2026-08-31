// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RBAC 权限检查 — 资源级粒度，拒绝含具体缺失路径
//!
//! 平台级基础设施，不依赖任何领域类型。
//! 支持：资源级权限检查、跨租户隔离、审计集成（feature flag 可选）。

use crate::policy::BuiltinRoles;
use std::sync::Arc;

// ── 资源描述 ────────────────────────────────────────────────────────────────

/// 资源描述（带可选租户隔离）
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Resource {
    pub path: String,
    pub tenant: Option<String>,
}

impl Resource {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.into(),
            tenant: None,
        }
    }
    pub fn with_tenant(path: &str, tenant: &str) -> Self {
        Self {
            path: path.into(),
            tenant: Some(tenant.into()),
        }
    }
}

// ── 权限检查上下文 ──────────────────────────────────────────────────────────

/// 权限检查上下文
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        Self {
            principal: principal.into(),
            roles,
            resource,
            action: action.into(),
            session_id: None,
            client_ip: None,
        }
    }
    pub fn with_session(mut self, sid: &str) -> Self {
        self.session_id = Some(sid.into());
        self
    }
    pub fn with_client_ip(mut self, ip: &str) -> Self {
        self.client_ip = Some(ip.into());
        self
    }
}

// ── 权限检查结果 ────────────────────────────────────────────────────────────

/// 权限检查结果
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PermissionResult {
    Granted,
    Denied(String),
}

impl PermissionResult {
    pub fn is_granted(&self) -> bool {
        matches!(self, Self::Granted)
    }
    pub fn denied_reason(&self) -> Option<&str> {
        match self {
            Self::Granted => None,
            Self::Denied(r) => Some(r),
        }
    }
}

// ── 检查核心 ────────────────────────────────────────────────────────────────

/// 执行权限检查（无审计）
///
/// 检查逻辑：
/// 1. 无角色 → 直接拒绝
/// 2. 展开所有角色的继承链，收集全部权限
/// 3. 跨租户隔离检查（principal 带 tenant: 前缀时生效）
/// 4. 逐一匹配权限，admin/* 自动通过
pub fn check(ctx: &PermissionCheck) -> PermissionResult {
    if ctx.roles.is_empty() {
        return PermissionResult::Denied("no roles assigned".into());
    }

    let policy = crate::policy::POLICY.read().unwrap();
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
            ctx.roles.join(", "),
            ctx.action,
            ctx.resource.path
        )),
    }
}

// ── 审计集成（可选 feature） ────────────────────────────────────────────────

/// 执行权限检查并集成审计（权限拒绝时自动产生审计事件）
///
/// 仅在启用 `audit` feature 时可用。
/// 审计事件不阻断权限检查流程，审计失败仅记录警告。
#[cfg(feature = "audit")]
pub fn check_with_audit(
    ctx: &PermissionCheck,
    audit_ctx: Option<&Arc<mox_audit::AuditContext>>,
) -> PermissionResult {
    let result = check(ctx);

    if !result.is_granted() {
        if let Some(audit) = audit_ctx {
            let reason = result.denied_reason().unwrap_or("unknown");
            let tenant = ctx.resource.tenant.as_deref().unwrap_or("default");
            let resource = mox_audit::AuditResource::flow(&ctx.resource.path, tenant);
            if let Err(e) = audit.log_rbac_denied(
                resource,
                tenant,
                &ctx.principal,
                &format!("{}:{}", ctx.action, ctx.resource.path),
            ) {
                tracing::warn!(target: "rbac", "audit write failed: {}", e);
            }
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
            "user:alice",
            vec!["viewer".into()],
            "write",
            Resource::new("db:prod/citizen_info"),
        );
        let r = check(&ctx);
        assert!(!r.is_granted());
        assert!(r.denied_reason().unwrap().contains("viewer"));
    }

    #[test]
    fn editor_can_write_test() {
        let ctx = PermissionCheck::new(
            "user:bob",
            vec!["editor".into()],
            "write",
            Resource::new("db:test/citizen_info"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn admin_can_do_anything() {
        let ctx = PermissionCheck::new(
            "user:charlie",
            vec!["admin".into()],
            "write",
            Resource::new("db:prod/anything"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn safety_approver_prod_write() {
        let ctx = PermissionCheck::new(
            "user:david",
            vec!["safety_approver".into()],
            "write",
            Resource::new("db:prod/citizen_info"),
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
    fn cross_tenant_admin_bypasses() {
        let ctx = PermissionCheck::new(
            "tenant:A:user:alice",
            vec!["admin".into()],
            "write",
            Resource::with_tenant("db:prod/data", "tenant:B"),
        );
        // admin 角色有 admin 权限，应绕过租户隔离
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn wildcard_everything() {
        let ctx = PermissionCheck::new(
            "user:eve",
            vec!["admin".into()],
            "execute",
            Resource::new("flow:anything"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn operator_can_read_all() {
        let ctx = PermissionCheck::new(
            "user:op",
            vec!["operator".into()],
            "read",
            Resource::new("db:prod/secret"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn auditor_can_read_audit() {
        let ctx = PermissionCheck::new(
            "user:aud",
            vec!["auditor".into()],
            "read",
            Resource::new("audit:logs/2024"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn multiple_roles_combine_permissions() {
        let ctx = PermissionCheck::new(
            "user:multi",
            vec!["viewer".into(), "auditor".into()],
            "read",
            Resource::new("audit:logs"),
        );
        // viewer 没有 audit 权限，但 auditor 有
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn permission_result_eq() {
        assert_eq!(PermissionResult::Granted, PermissionResult::Granted);
        assert_eq!(
            PermissionResult::Denied("reason".into()),
            PermissionResult::Denied("reason".into())
        );
        assert_ne!(
            PermissionResult::Granted,
            PermissionResult::Denied("reason".into())
        );
    }

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
    fn permission_check_builder() {
        let ctx = PermissionCheck::new(
            "user:test",
            vec!["viewer".into()],
            "read",
            Resource::new("db:test"),
        )
        .with_session("sess-123")
        .with_client_ip("10.0.0.1");

        assert_eq!(ctx.session_id.as_deref(), Some("sess-123"));
        assert_eq!(ctx.client_ip.as_deref(), Some("10.0.0.1"));
    }

    #[cfg(feature = "audit")]
    #[test]
    fn check_with_audit_denied_produces_event() {
        use mox_audit::{MultiSink, NoopSink};

        let sink = MultiSink::new().with_sink(Box::new(NoopSink));
        let audit_ctx = Arc::new(
            mox_audit::AuditContext::new(Arc::new(sink)).with_hmac_secret("test-secret".into()),
        );

        let ctx = PermissionCheck::new(
            "user:alice",
            vec!["viewer".into()],
            "write",
            Resource::new("db:prod/secret"),
        );

        let result = check_with_audit(&ctx, Some(&audit_ctx));
        assert!(!result.is_granted());
        // 审计链中应有 1 个事件（RBAC 拒绝）
        assert_eq!(audit_ctx.chain_len(), 1);
    }

    #[cfg(feature = "audit")]
    #[test]
    fn check_with_audit_granted_no_event() {
        use mox_audit::{MultiSink, NoopSink};

        let sink = MultiSink::new().with_sink(Box::new(NoopSink));
        let audit_ctx = Arc::new(
            mox_audit::AuditContext::new(Arc::new(sink)).with_hmac_secret("test-secret".into()),
        );

        let ctx = PermissionCheck::new(
            "user:alice",
            vec!["viewer".into()],
            "read",
            Resource::new("db:test/data"),
        );

        let result = check_with_audit(&ctx, Some(&audit_ctx));
        assert!(result.is_granted());
        // 审计链中不应有事件（通过时不产生审计）
        assert_eq!(audit_ctx.chain_len(), 0);
    }

    #[cfg(feature = "audit")]
    #[test]
    fn check_with_audit_none_ctx_works() {
        let ctx = PermissionCheck::new(
            "user:alice",
            vec!["viewer".into()],
            "write",
            Resource::new("db:prod/secret"),
        );

        // 传入 None 审计上下文，应正常工作且不崩溃
        let result = check_with_audit(&ctx, None);
        assert!(!result.is_granted());
    }
}
