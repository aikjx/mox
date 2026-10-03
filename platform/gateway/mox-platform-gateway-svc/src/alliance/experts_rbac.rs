// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! # 专家联盟后端 RBAC 强制层（Experts RBAC）
//!
//! 14 号企业权限模型 §五 G-2 缺口 R1 落地：把权限矩阵闭环到「后端强制」。
//!
//! ## 路径选择
//!
//! 选 **路径 b：联盟域轻量 RBAC 层**（本模块），不依赖 mox-ai-expert-svc 的
//! `check_with_audit`。理由：
//! 1. mox-ai-expert-svc 是重型 svc（reqwest blocking / rayon / rusqlite / mox-kg-sdk
//!    等），网关轻量服务拉入整套依赖树成本高；
//! 2. `check_with_audit` 依赖其私有 audit 模块（`crate::audit::integration::AuditContext`、
//!    `crate::audit::AuditResource`），跨 crate 复用需把整套审计设施 pub 化；
//! 3. 角色语义不匹配——mox-ai-expert-svc 的角色是 viewer/editor/admin/safety_approver
//!    （39 号链），联盟现行是前端 5 角色码
//!    （super_admin / tenant_admin / dept_manager / normal_user / readonly_auditor）；
//! 4. 其 POLICY 是全局静态策略（服务实例状态），不适合作为网关多租户策略源。
//!
//! ## 强制边界
//!
//! - **管理写面**（专家注册/CRUD、图谱重建、调度配置写、负载重置、专家删除）：
//!   必须持 `super_admin` 或 `tenant_admin` 角色，否则 401/403 拒绝 + 审计。
//! - **自服务写面**（consult / multi_consult / debate、会话创建/追加/归档、
//!   收藏/预约/取消）：仅要求认证（`UserInfo` 存在），不强制角色。
//! - **读面**：认证即可（由 auth_middleware 统一保证）。
//!
//! ## 审计闭环
//!
//! 拒绝事件经现有 `emit_audit` 记录（事件名 `rbac.denied`，对应
//! `AuditAction::RBACDenied`，outcome 为 `Blocked`），不引入第二套审计。

use mox_api_protocol::ApiResponse;
use mox_audit::{AuditAction, AuditOutcome};
use mox_platform_api::UserInfo;
use serde_json::Value;

use super::experts_common::{
    ExpertsSharedState, actor_from_opt_user, emit_audit, err,
};

/// 管理角色集（14 号矩阵 §五「管理写面」允许的角色码）。
///
/// - `super_admin`：平台超管，跨租户
/// - `tenant_admin`：租户管理员，本租户内管理写面
///
/// 其余角色（`dept_manager` / `normal_user` / `readonly_auditor`）一律不允许
/// 执行管理写面。
pub const ADMIN_ROLES: &[&str] = &["super_admin", "tenant_admin"];

/// 管理写面动作枚举（用于审计 resource 与拒绝消息）。
///
/// 自服务写面（consult / 会话 / 收藏 / 预约）**不**经过本枚举——它们仅要求认证。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RbacAction {
    /// 注册专家（POST /api/experts）
    RegisterExpert,
    /// 更新专家（PUT /api/experts/:id）
    UpdateExpert,
    /// 删除/软删除专家（DELETE /api/experts/:id）
    DeleteExpert,
    /// 重建图谱（POST /api/expert-graph/rebuild）
    RebuildGraph,
    /// 图谱节点/边增量增删改（POST/PUT/DELETE /api/expert-graph/nodes|edges）
    MutateGraph,
    /// 重置单专家调度负载（POST /api/experts/dispatcher/reset/:id）
    ResetDispatcher,
    /// 重置所有专家调度负载（POST /api/experts/dispatcher/reset-all）
    ResetAllDispatcher,
    /// 更新调度配置（PUT /api/experts/dispatcher/config）
    UpdateConfig,
    /// Webhook 地址与订阅管理（读写均限制管理员）
    ManageWebhooks,
}

impl RbacAction {
    /// 动作短名（写入审计 resource_type / detail）。
    pub fn code(self) -> &'static str {
        match self {
            RbacAction::RegisterExpert => "expert.register",
            RbacAction::UpdateExpert => "expert.update",
            RbacAction::DeleteExpert => "expert.disable",
            RbacAction::RebuildGraph => "graph.rebuild",
            RbacAction::MutateGraph => "graph.mutate",
            RbacAction::ResetDispatcher => "dispatcher.reset",
            RbacAction::ResetAllDispatcher => "dispatcher.reset_all",
            RbacAction::UpdateConfig => "dispatcher.config_update",
            RbacAction::ManageWebhooks => "webhook.manage",
        }
    }

    /// 面向用户的可读描述（用于 403 拒绝消息）。
    pub fn description(self) -> &'static str {
        match self {
            RbacAction::RegisterExpert => "注册专家",
            RbacAction::UpdateExpert => "更新专家信息",
            RbacAction::DeleteExpert => "删除专家",
            RbacAction::RebuildGraph => "重建专家图谱",
            RbacAction::MutateGraph => "图谱节点/边增量维护",
            RbacAction::ResetDispatcher => "重置专家调度负载",
            RbacAction::ResetAllDispatcher => "重置全部专家调度负载",
            RbacAction::UpdateConfig => "更新调度配置",
            RbacAction::ManageWebhooks => "管理事件订阅",
        }
    }
}

/// RBAC 拒绝结果。
///
/// - `status`：HTTP 语义码（401 未认证 / 403 已认证但角色不足）
/// - `reason`：面向调用方的中文原因
/// - `suggested`：建议动作（如「联系租户管理员授权」）
#[derive(Debug, Clone)]
pub struct RbacDenied {
    pub status: u16,
    pub reason: String,
    pub suggested: String,
}

impl std::fmt::Display for RbacDenied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}

/// 纯判断：当前用户是否允许执行管理写面动作。
///
/// - `None`（未认证）→ `Err(401)`
/// - `Some` 但 roles 与 [`ADMIN_ROLES`] 无交集 → `Err(403)`
/// - 匹配 → `Ok(())`
///
/// 本函数不产生副作用；审计由 [`audit_denied`] 或 [`enforce_admin_or_respond`]
/// 在 handler 层统一发射。
pub fn enforce_admin(user: &Option<UserInfo>, action: RbacAction) -> Result<(), RbacDenied> {
    let u = match user {
        Some(u) => u,
        None => {
            return Err(RbacDenied {
                status: 401,
                reason: format!(
                    "未认证：执行「{}」需要登录（动作 {}）",
                    action.description(),
                    action.code()
                ),
                suggested: "请先携带有效 JWT / API Key 登录".to_string(),
            });
        }
    };

    let has_admin = u.roles.iter().any(|r| ADMIN_ROLES.contains(&r.as_str()));
    if has_admin {
        return Ok(());
    }

    let roles_seen = if u.roles.is_empty() {
        "<无角色>".to_string()
    } else {
        u.roles.join(",")
    };
    Err(RbacDenied {
        status: 403,
        reason: format!(
            "权限不足：执行「{}」需要角色 {}，当前用户角色 [{}]",
            action.description(),
            ADMIN_ROLES.join("/"),
            roles_seen
        ),
        suggested: "请联系租户管理员（tenant_admin）或平台超管（super_admin）授权".to_string(),
    })
}

/// 拒绝时发射审计事件（`rbac.denied` / `Blocked`）。
///
/// 复用现有 [`emit_audit`] 与 [`actor_from_opt_user`]，不引入第二套审计。
/// `action` 用于 resource_type；`denied.reason` 写入 detail。
pub fn audit_denied(
    state: &ExpertsSharedState,
    user: &Option<UserInfo>,
    tenant: &str,
    action: RbacAction,
    denied: &RbacDenied,
) {
    let actor = actor_from_opt_user(user);
    let detail = format!(
        "action={}; status={}; reason={}; suggested={}",
        action.code(),
        denied.status,
        denied.reason,
        denied.suggested
    );
    emit_audit(
        state,
        &actor,
        tenant,
        AuditAction::RBACDenied,
        "rbac",
        action.code(),
        AuditOutcome::Blocked,
        Some(&detail),
    );
}

/// 一站式强制：先判角色，拒绝则审计并返回 [`ApiResponse`]（401/403）。
///
/// handler 内典型用法：
///
/// ```ignore
/// if let Err(resp) = enforce_admin_or_respond(&state, &user, &tenant.0, RbacAction::RegisterExpert) {
///     return resp;
/// }
/// ```
pub fn enforce_admin_or_respond(
    state: &ExpertsSharedState,
    user: &Option<UserInfo>,
    tenant: &str,
    action: RbacAction,
) -> Result<(), ApiResponse<Value>> {
    match enforce_admin(user, action) {
        Ok(()) => Ok(()),
        Err(d) => {
            audit_denied(state, user, tenant, action, &d);
            Err(err(d.status, d.reason))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alliance::experts_common::{
        DEFAULT_TENANT, DispatcherConfig, ExpertGraph, ExpertsSharedState,
    };
    use mox_audit::{AuditContext, MultiSink, NoopSink};
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;

    fn make_admin_user() -> UserInfo {
        UserInfo {
            id: "u-admin-001".into(),
            username: "admin".into(),
            email: "admin@example.com".into(),
            tenant_id: "t-default".into(),
            roles: vec!["tenant_admin".into()],
            enabled: true,
            created_at: "2026-09-30T00:00:00Z".into(),
        }
    }

    fn make_normal_user() -> UserInfo {
        let mut u = make_admin_user();
        u.id = "u-normal-001".into();
        u.username = "normal".into();
        u.roles = vec!["normal_user".into()];
        u
    }

    fn make_test_state() -> Arc<ExpertsSharedState> {
        let audit =
            AuditContext::new(Arc::new(MultiSink::new().with_sink(Box::new(NoopSink))));
        Arc::new(ExpertsSharedState {
            registry: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            dispatcher_config: Arc::new(Mutex::new(DispatcherConfig::default())),
            dispatch_records: Arc::new(Mutex::new(Vec::new())),
            graph: Arc::new(Mutex::new(HashMap::new())),
            plans: Arc::new(Mutex::new(HashMap::new())),
            orchestration_history: Arc::new(Mutex::new(Vec::new())),
            favorites: Arc::new(Mutex::new(HashMap::new())),
            audit: Arc::new(audit),
            events: Arc::new(crate::alliance::experts_events::EventBus::new(16)),
        })
    }

    // 用例 1：未认证 → 401
    #[test]
    fn test_enforce_admin_rejects_anonymous() {
        let r = enforce_admin(&None, RbacAction::RegisterExpert);
        let d = r.expect_err("anonymous must be denied");
        assert_eq!(d.status, 401);
        assert!(d.reason.contains("未认证"));
    }

    // 用例 2：非管理角色（normal_user）→ 403
    #[test]
    fn test_enforce_admin_rejects_non_admin() {
        let u = make_normal_user();
        let r = enforce_admin(&Some(u), RbacAction::RebuildGraph);
        let d = r.expect_err("normal_user must be denied");
        assert_eq!(d.status, 403);
        assert!(d.reason.contains("权限不足"));
        assert!(d.reason.contains("normal_user"));
    }

    // 用例 3：管理角色（tenant_admin）→ Ok
    #[test]
    fn test_enforce_admin_allows_admin() {
        let u = make_admin_user();
        enforce_admin(&Some(u), RbacAction::UpdateConfig)
            .expect("tenant_admin must pass");
    }

    // 用例 3b：super_admin 也放行
    #[test]
    fn test_enforce_admin_allows_super_admin() {
        let mut u = make_admin_user();
        u.roles = vec!["super_admin".into()];
        enforce_admin(&Some(u), RbacAction::DeleteExpert)
            .expect("super_admin must pass");
    }

    // 用例 4：一站式拒绝路径返回 401 并审计不 panic
    #[test]
    fn test_enforce_admin_or_respond_anonymous_returns_401() {
        let state = make_test_state();
        let resp = enforce_admin_or_respond(&state, &None, DEFAULT_TENANT, RbacAction::ResetAllDispatcher);
        assert!(resp.is_err());
        let r = resp.err().unwrap();
        assert_eq!(r.code, 401);
    }

    // 用例 5：一站式拒绝路径返回 403
    #[test]
    fn test_enforce_admin_or_respond_non_admin_returns_403() {
        let state = make_test_state();
        let u = make_normal_user();
        let resp = enforce_admin_or_respond(&state, &Some(u), DEFAULT_TENANT, RbacAction::ResetDispatcher);
        assert!(resp.is_err());
        let r = resp.err().unwrap();
        assert_eq!(r.code, 403);
    }

    // 用例 6：管理角色放行返回 Ok（不产生 ApiResponse）
    #[test]
    fn test_enforce_admin_or_respond_admin_ok() {
        let state = make_test_state();
        let u = make_admin_user();
        let resp = enforce_admin_or_respond(&state, &Some(u), DEFAULT_TENANT, RbacAction::RegisterExpert);
        assert!(resp.is_ok());
    }

    // 用例 7：ADMIN_ROLES 集合内容固定
    #[test]
    fn test_admin_roles_constant() {
        assert_eq!(ADMIN_ROLES, &["super_admin", "tenant_admin"]);
    }

    // 用例 8：拒绝审计不 panic（sink 为 Noop）
    #[test]
    fn test_audit_denied_does_not_panic() {
        let state = make_test_state();
        let u = make_normal_user();
        let d = enforce_admin(&Some(u.clone()), RbacAction::RebuildGraph)
            .expect_err("must be denied");
        audit_denied(&state, &Some(u), DEFAULT_TENANT, RbacAction::RebuildGraph, &d);
        let _ = AuditOutcome::Blocked;
    }
}
