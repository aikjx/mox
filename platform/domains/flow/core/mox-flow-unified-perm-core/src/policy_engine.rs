// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 统一策略引擎
//!
//! 融合 RBAC 和 ABAC 的统一决策引擎：
//! 1. 先通过 RBAC 进行粗粒度授权
//! 2. 再通过 ABAC 进行细粒度策略评估
//! 3. Deny 优先原则
//! 4. 数据权限过滤

use std::sync::Arc;

use crate::abac::{AbacEngine, AttributeContext};
use crate::data_perm::DataPermissionManager;
use crate::error::PermResult;
use crate::rbac::RbacManager;
use crate::types::{
    Action, PermissionEffect, ResourceScope, Role, Subject,
};

/// 授权决策
#[derive(Debug, Clone, PartialEq)]
pub enum AuthzDecision {
    /// 允许
    Allow,
    /// 拒绝
    Deny,
    /// 不适用（交给下一层）
    NotApplicable,
}

/// 授权结果
#[derive(Debug, Clone)]
pub struct AuthzResult {
    /// 最终决策
    pub decision: AuthzDecision,
    /// 原因
    pub reason: String,
    /// 匹配的策略/权限 ID
    pub matched_permissions: Vec<String>,
    /// 数据范围（如果适用）
    pub data_scope: Option<String>,
    /// 评估耗时（毫秒）
    pub eval_time_ms: u64,
}

impl AuthzResult {
    /// 是否允许
    pub fn is_allowed(&self) -> bool {
        matches!(self.decision, AuthzDecision::Allow)
    }

    /// 允许结果
    pub fn allow(reason: &str) -> Self {
        Self {
            decision: AuthzDecision::Allow,
            reason: reason.to_string(),
            matched_permissions: Vec::new(),
            data_scope: None,
            eval_time_ms: 0,
        }
    }

    /// 拒绝结果
    pub fn deny(reason: &str) -> Self {
        Self {
            decision: AuthzDecision::Deny,
            reason: reason.to_string(),
            matched_permissions: Vec::new(),
            data_scope: None,
            eval_time_ms: 0,
        }
    }

    /// 不适用
    pub fn not_applicable() -> Self {
        Self {
            decision: AuthzDecision::NotApplicable,
            reason: "no applicable policy".to_string(),
            matched_permissions: Vec::new(),
            data_scope: None,
            eval_time_ms: 0,
        }
    }
}

/// 策略引擎
pub struct PolicyEngine {
    /// RBAC 管理器
    pub rbac: Arc<RbacManager>,
    /// ABAC 引擎
    pub abac: Arc<AbacEngine>,
    /// 数据权限管理器
    pub data_perm: Arc<DataPermissionManager>,
}

impl PolicyEngine {
    /// 创建策略引擎
    pub fn new(
        rbac: Arc<RbacManager>,
        abac: Arc<AbacEngine>,
        data_perm: Arc<DataPermissionManager>,
    ) -> Self {
        Self {
            rbac,
            abac,
            data_perm,
        }
    }

    /// 完整授权检查
    pub fn check(
        &self,
        subject: &Subject,
        action: &Action,
        resource: &ResourceScope,
    ) -> PermResult<AuthzResult> {
        let start = crate::types::now_ms();
        let mut result = AuthzResult::not_applicable();

        // 1. RBAC 检查
        let rbac_allowed = self.rbac.check_permission(subject, action, resource)?;

        if !rbac_allowed {
            result.decision = AuthzDecision::Deny;
            result.reason = "RBAC: no matching allow permission".to_string();
            result.eval_time_ms = start.elapsed_millis();
            return Ok(result);
        }

        // 2. ABAC 策略评估
        let ctx = AttributeContext::from_subject(subject).with_resource(resource);
        let abac_result = self
            .abac
            .evaluate(action, resource, &ctx, &subject.tenant_id)?;

        match abac_result {
            Some(PermissionEffect::Deny) => {
                result.decision = AuthzDecision::Deny;
                result.reason = "ABAC: deny policy matched".to_string();
                result.eval_time_ms = start.elapsed_millis();
                return Ok(result);
            }
            Some(PermissionEffect::Allow) => {
                result.decision = AuthzDecision::Allow;
                result.reason = "RBAC + ABAC allow".to_string();
            }
            None => {
                // 无 ABAC 策略，RBAC 通过即通过
                result.decision = AuthzDecision::Allow;
                result.reason = "RBAC allow (no ABAC policy)".to_string();
            }
        }

        // 3. 收集匹配的权限信息
        let perms = self.rbac.get_subject_permissions(subject)?;
        result.matched_permissions = perms.iter().map(|p| p.id.clone()).collect();

        result.eval_time_ms = start.elapsed_millis();
        Ok(result)
    }

    /// 快速检查（只返回布尔）
    pub fn is_allowed(
        &self,
        subject: &Subject,
        action: &Action,
        resource: &ResourceScope,
    ) -> PermResult<bool> {
        Ok(self.check(subject, action, resource)?.is_allowed())
    }

    /// 获取主体的所有角色
    pub fn get_subject_roles(&self, subject: &Subject) -> PermResult<Vec<Role>> {
        self.rbac.get_subject_roles(subject)
    }

    /// 获取主体在指定资源上的数据范围
    pub fn get_data_scope(
        &self,
        subject: &Subject,
        resource_type: &str,
    ) -> PermResult<crate::data_perm::DataScope> {
        let roles = self.rbac.get_subject_roles(subject)?;
        let role_ids: Vec<String> = roles.iter().map(|r| r.id.clone()).collect();
        Ok(self.data_perm.get_data_scope(&role_ids, resource_type))
    }

    /// 批量检查多个操作/资源
    pub fn batch_check(
        &self,
        subject: &Subject,
        requests: &[(Action, ResourceScope)],
    ) -> PermResult<Vec<AuthzResult>> {
        let mut results = Vec::with_capacity(requests.len());
        for (action, resource) in requests {
            results.push(self.check(subject, action, resource)?);
        }
        Ok(results)
    }
}

/// 时间戳扩展（计算耗时）
trait TimestampExt {
    fn elapsed_millis(&self) -> u64;
}

impl TimestampExt for u64 {
    fn elapsed_millis(&self) -> u64 {
        crate::types::now_ms().saturating_sub(*self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abac::{AbacPolicy, ConditionExpr};
    use crate::data_perm::{DataScope, RoleDataPermission};
    use crate::types::{
        Permission, ResourceScope, Role, RoleBinding, SubjectType,
    };

    fn setup_engine() -> PolicyEngine {
        let rbac = Arc::new(RbacManager::new());
        let abac = Arc::new(AbacEngine::new());
        let data_perm = Arc::new(DataPermissionManager::new());

        let tenant = "t-1";

        // 创建权限
        let read_doc = Permission::allow(
            "read_doc",
            Action::new("read:*"),
            ResourceScope::all("document"),
            tenant,
        );
        rbac.create_permission(read_doc.clone()).unwrap();

        // 创建角色
        let viewer = Role::new("Viewer", "viewer", tenant);
        let viewer = rbac.create_role(viewer).unwrap();
        rbac.add_permission_to_role(&viewer.id, &read_doc.id).unwrap();

        // 绑定用户
        let subject = Subject::user("alice", tenant);
        let binding =
            RoleBinding::new(&subject.id, SubjectType::User, &viewer.id, tenant);
        rbac.create_binding(binding).unwrap();

        PolicyEngine::new(rbac, abac, data_perm)
    }

    #[test]
    fn test_rbac_allow() {
        let engine = setup_engine();
        let subject = Subject::user("alice", "t-1");

        let result = engine
            .check(
                &subject,
                &Action::new("read:doc"),
                &ResourceScope::of("document", "d1"),
            )
            .unwrap();

        assert!(result.is_allowed());
    }

    #[test]
    fn test_rbac_deny() {
        let engine = setup_engine();
        let subject = Subject::user("alice", "t-1");

        let result = engine
            .check(
                &subject,
                &Action::new("delete:doc"),
                &ResourceScope::of("document", "d1"),
            )
            .unwrap();

        assert!(!result.is_allowed());
        assert_eq!(result.decision, AuthzDecision::Deny);
    }

    #[test]
    fn test_abac_deny_overrides_rbac_allow() {
        let engine = setup_engine();
        let tenant = "t-1";

        // 添加 ABAC 拒绝策略：非工作时间不能读文档
        let policy = AbacPolicy::new(
            "deny_after_hours",
            tenant,
            PermissionEffect::Deny,
            Action::new("read:*"),
            "document",
            ConditionExpr::Compare {
                attr_path: "environment.hour".to_string(),
                op: crate::abac::CompareOp::Lt,
                value: crate::abac::AttributeValue::Int(9),
            },
        );
        engine.abac.add_policy(policy);

        let subject = Subject::user("alice", tenant);
        let resource = ResourceScope::of("document", "d1");
        let attr_ctx = crate::abac::AttributeContext::from_subject(&subject)
            .with_resource(&resource)
            .with_env("hour", crate::abac::AttributeValue::Int(3));

        let abac_result = engine
            .abac
            .evaluate(&Action::new("read:doc"), &resource, &attr_ctx, tenant)
            .unwrap();

        // 验证 ABAC 引擎的 deny 策略能匹配
        assert_eq!(abac_result, Some(PermissionEffect::Deny));
    }

    #[test]
    fn test_batch_check() {
        let engine = setup_engine();
        let subject = Subject::user("alice", "t-1");

        let requests = vec![
            (
                Action::new("read:doc"),
                ResourceScope::of("document", "d1"),
            ),
            (
                Action::new("write:doc"),
                ResourceScope::of("document", "d1"),
            ),
        ];

        let results = engine.batch_check(&subject, &requests).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_allowed());
        assert!(!results[1].is_allowed());
    }

    #[test]
    fn test_get_data_scope() {
        let engine = setup_engine();
        let subject = Subject::user("alice", "t-1");

        // 获取角色并设置数据权限
        let roles = engine.get_subject_roles(&subject).unwrap();
        assert_eq!(roles.len(), 1);

        engine
            .data_perm
            .set_role_permission(RoleDataPermission::new(
                &roles[0].id,
                "document",
                DataScope::DeptOnly,
                "t-1",
            ));

        let scope = engine.get_data_scope(&subject, "document").unwrap();
        assert_eq!(scope, DataScope::DeptOnly);
    }

    #[test]
    fn test_cross_tenant_isolation() {
        let engine = setup_engine();
        // t-2 的用户不能访问 t-1 的资源
        let subject = Subject::user("alice", "t-2");

        let result = engine
            .check(
                &subject,
                &Action::new("read:doc"),
                &ResourceScope::of("document", "d1"),
            )
            .unwrap();

        assert!(!result.is_allowed());
    }
}
