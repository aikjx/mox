// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! RBAC 评估引擎
//!
//! 核心引擎，整合角色继承、策略存储、ABAC 条件、缓存层，提供统一的权限检查接口。
//!
//! # 评估流程
//!
//! 1. 检查缓存（如果启用）
//! 2. 展开角色继承链，收集所有权限
//! 3. 跨租户隔离检查
//! 4. 匹配策略（按优先级排序，拒绝优先）
//! 5. 评估 ABAC 条件（如果策略有条件）
//! 6. 返回评估结果
//! 7. 写入缓存（如果启用）
//! 8. 派发事件（如果有监听器）

use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::abac::ConditionEvaluator;
use crate::cache::{CacheKey, EvaluationCache};
use crate::error::RbacError;
use crate::events::{EventEnvelope, EventListener, RbacEvent};
use crate::hierarchy::RoleHierarchy;
use crate::store::{MemoryPolicyStore, PolicyStore};
use crate::types::{
    Action, Effect, EvaluationContext, EvaluationResult, Permission, Policy, Role,
    Subject, Resource,
};

/// RBAC 引擎配置
#[derive(Debug, Clone)]
pub struct RbacEngineConfig {
    /// 是否启用缓存
    pub cache_enabled: bool,
    /// 缓存容量（条目数）
    pub cache_capacity: usize,
    /// 是否默认拒绝（无匹配策略时的默认行为）
    pub default_deny: bool,
    /// 是否启用 ABAC 条件评估
    pub abac_enabled: bool,
    /// 是否启用事件派发
    pub events_enabled: bool,
}

impl Default for RbacEngineConfig {
    fn default() -> Self {
        Self {
            cache_enabled: true,
            cache_capacity: 1024,
            default_deny: true,
            abac_enabled: true,
            events_enabled: true,
        }
    }
}

/// RBAC 引擎
///
/// 权限评估的核心入口。整合角色继承、策略匹配、ABAC 条件、缓存和事件系统。
///
/// # 示例
///
/// ```ignore
/// use mox_rbac_engine::RbacEngine;
///
/// let engine = RbacEngine::with_builtin_roles();
/// let result = engine.check("user:alice", &["editor"], "write", "db:test/data");
/// assert!(result.is_granted());
/// ```
pub struct RbacEngine {
    config: RbacEngineConfig,
    store: Arc<dyn PolicyStore>,
    hierarchy: RwLock<RoleHierarchy>,
    cache: Option<EvaluationCache>,
    listeners: RwLock<Vec<Arc<dyn EventListener>>>,
}

impl std::fmt::Debug for RbacEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RbacEngine")
            .field("config", &self.config)
            .field("cache_enabled", &self.cache.is_some())
            .field("listeners_count", &self.listeners.read().map(|l| l.len()).unwrap_or(0))
            .finish_non_exhaustive()
    }
}

impl RbacEngine {
    // ── 构造函数 ──────────────────────────────────────────────────────────

    /// 使用内存存储和内置角色创建引擎
    pub fn with_builtin_roles() -> Self {
        let store = Arc::new(MemoryPolicyStore::with_builtin_roles());
        Self::new(store, RbacEngineConfig::default())
    }

    /// 使用指定存储和配置创建引擎
    pub fn new(store: Arc<dyn PolicyStore>, config: RbacEngineConfig) -> Self {
        let hierarchy = store.build_hierarchy().unwrap_or_default();

        let cache = if config.cache_enabled {
            Some(EvaluationCache::with_capacity(config.cache_capacity))
        } else {
            None
        };

        Self {
            config,
            store,
            hierarchy: RwLock::new(hierarchy),
            cache,
            listeners: RwLock::new(Vec::new()),
        }
    }

    /// 使用内存存储创建空引擎
    pub fn empty() -> Self {
        let store = Arc::new(MemoryPolicyStore::new());
        Self::new(store, RbacEngineConfig::default())
    }

    // ── 配置 ──────────────────────────────────────────────────────────────

    /// 获取配置引用
    pub fn config(&self) -> &RbacEngineConfig {
        &self.config
    }

    // ── 事件监听器 ────────────────────────────────────────────────────────

    /// 添加事件监听器
    pub fn add_listener(&self, listener: Arc<dyn EventListener>) {
        if let Ok(mut listeners) = self.listeners.write() {
            listeners.push(listener);
        }
    }

    /// 派发事件
    fn dispatch_event(&self, event: RbacEvent) {
        if !self.config.events_enabled {
            return;
        }

        let envelope = EventEnvelope::wrap(event);
        let category = envelope.category;

        if let Ok(listeners) = self.listeners.read() {
            for listener in listeners.iter() {
                if listener.is_interested(category) {
                    listener.on_event(&envelope);
                }
            }
        }
    }

    // ── 角色管理 ──────────────────────────────────────────────────────────

    /// 添加角色
    pub fn add_role(&self, role: Role) -> Result<(), RbacError> {
        self.store.create_role(role.clone())?;
        self.refresh_hierarchy()?;
        self.invalidate_cache("role added");

        self.dispatch_event(RbacEvent::RoleCreated {
            role: role.name,
            operator: None,
        });

        Ok(())
    }

    /// 更新角色
    pub fn update_role(&self, role: Role) -> Result<(), RbacError> {
        let name = role.name.clone();
        self.store.update_role(role)?;
        self.refresh_hierarchy()?;
        self.invalidate_cache("role updated");

        self.dispatch_event(RbacEvent::RoleUpdated {
            role: name,
            changes: vec!["full".into()],
            operator: None,
        });

        Ok(())
    }

    /// 删除角色
    pub fn delete_role(&self, name: &str) -> Result<bool, RbacError> {
        let deleted = self.store.delete_role(name)?;
        if deleted {
            self.refresh_hierarchy()?;
            self.invalidate_cache("role deleted");

            self.dispatch_event(RbacEvent::RoleDeleted {
                role: name.into(),
                operator: None,
            });
        }
        Ok(deleted)
    }

    /// 获取角色
    pub fn get_role(&self, name: &str) -> Result<Option<Role>, RbacError> {
        self.store.get_role(name)
    }

    /// 列出所有角色
    pub fn list_roles(&self) -> Result<Vec<Role>, RbacError> {
        self.store.list_roles()
    }

    /// 刷新继承树
    fn refresh_hierarchy(&self) -> Result<(), RbacError> {
        let new_hierarchy = self.store.build_hierarchy()?;
        if let Ok(mut h) = self.hierarchy.write() {
            *h = new_hierarchy;
        }
        Ok(())
    }

    // ── 策略管理 ──────────────────────────────────────────────────────────

    /// 添加策略
    pub fn add_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let id = policy.id.clone();
        let name = policy.name.clone();
        self.store.create_policy(policy)?;
        self.invalidate_cache("policy added");

        self.dispatch_event(RbacEvent::PolicyCreated {
            policy_id: id,
            policy_name: name,
            operator: None,
        });

        Ok(())
    }

    /// 更新策略
    pub fn update_policy(&self, policy: Policy) -> Result<(), RbacError> {
        let id = policy.id.clone();
        self.store.update_policy(policy)?;
        self.invalidate_cache("policy updated");

        self.dispatch_event(RbacEvent::PolicyUpdated {
            policy_id: id,
            changes: vec!["full".into()],
            operator: None,
        });

        Ok(())
    }

    /// 删除策略
    pub fn delete_policy(&self, id: &str) -> Result<bool, RbacError> {
        let deleted = self.store.delete_policy(id)?;
        if deleted {
            self.invalidate_cache("policy deleted");

            self.dispatch_event(RbacEvent::PolicyDeleted {
                policy_id: id.into(),
                operator: None,
            });
        }
        Ok(deleted)
    }

    /// 获取策略
    pub fn get_policy(&self, id: &str) -> Result<Option<Policy>, RbacError> {
        self.store.get_policy(id)
    }

    /// 列出所有策略
    pub fn list_policies(&self) -> Result<Vec<Policy>, RbacError> {
        self.store.list_policies()
    }

    // ── 缓存管理 ──────────────────────────────────────────────────────────

    /// 失效所有缓存
    fn invalidate_cache(&self, reason: &str) {
        if let Some(cache) = &self.cache {
            if let Ok(count) = cache.invalidate_all() {
                self.dispatch_event(RbacEvent::CacheInvalidated {
                    reason: reason.into(),
                    invalidated_count: count,
                });
            }
        }
    }

    /// 获取缓存统计
    pub fn cache_stats(&self) -> Option<crate::cache::CacheStats> {
        self.cache.as_ref().map(|c| c.stats())
    }

    // ── 权限检查（便捷 API） ──────────────────────────────────────────────

    /// 便捷权限检查（字符串参数）
    ///
    /// 这是最简单的使用方式：提供主体 ID、角色列表、动作和资源路径。
    pub fn check(
        &self,
        subject_id: &str,
        roles: &[String],
        action: &str,
        resource_path: &str,
    ) -> EvaluationResult {
        let subject = Subject::new(subject_id, roles.to_vec());
        let resource = Resource::new(resource_path);
        let action = Action::from_str(action);
        let ctx = EvaluationContext::new(subject, resource, action);
        self.evaluate(&ctx)
    }

    /// 带租户的权限检查
    pub fn check_with_tenant(
        &self,
        subject_id: &str,
        roles: &[String],
        action: &str,
        resource_path: &str,
        tenant: &str,
    ) -> EvaluationResult {
        let subject = Subject::new(subject_id, roles.to_vec());
        let resource = Resource::with_tenant(resource_path, tenant);
        let action = Action::from_str(action);
        let ctx = EvaluationContext::new(subject, resource, action);
        self.evaluate(&ctx)
    }

    // ── 核心评估 ──────────────────────────────────────────────────────────

    /// 完整评估（使用 EvaluationContext）
    pub fn evaluate(&self, ctx: &EvaluationContext) -> EvaluationResult {
        let start = Instant::now();

        // 1. 检查缓存
        if let Some(cache) = &self.cache {
            let cache_key = CacheKey::new(
                &ctx.subject.roles,
                &ctx.resource.path,
                ctx.action.clone(),
                ctx.resource.tenant.as_deref(),
            );

            if let Some(cached) = cache.get(&cache_key) {
                self.dispatch_event(RbacEvent::CacheHit {
                    key: format!("{}:{}", ctx.subject.roles.join(","), ctx.resource.path),
                });
                let duration_us = start.elapsed().as_micros() as u64;
                self.dispatch_decision_event(ctx, &cached, duration_us);
                return cached;
            } else {
                self.dispatch_event(RbacEvent::CacheMiss {
                    key: format!("{}:{}", ctx.subject.roles.join(","), ctx.resource.path),
                });
            }
        }

        // 2. 执行评估
        let result = self.evaluate_impl(ctx);

        // 3. 写入缓存
        if let Some(cache) = &self.cache {
            let cache_key = CacheKey::new(
                &ctx.subject.roles,
                &ctx.resource.path,
                ctx.action.clone(),
                ctx.resource.tenant.as_deref(),
            );
            let _ = cache.put(cache_key, result.clone());
        }

        // 4. 派发决策事件
        let duration_us = start.elapsed().as_micros() as u64;
        self.dispatch_decision_event(ctx, &result, duration_us);

        result
    }

    /// 派发决策事件
    fn dispatch_decision_event(&self, ctx: &EvaluationContext, result: &EvaluationResult, duration_us: u64) {
        let event = RbacEvent::from_evaluation(
            &ctx.subject.id,
            &ctx.resource.path,
            &ctx.action,
            result,
            duration_us,
        );
        self.dispatch_event(event);
    }

    /// 评估实现
    fn evaluate_impl(&self, ctx: &EvaluationContext) -> EvaluationResult {
        // 无角色 → 拒绝
        if ctx.subject.roles.is_empty() {
            return EvaluationResult::Denied {
                reason: "no roles assigned".into(),
                denied_by_policy: None,
            };
        }

        // 跨租户隔离检查
        if let Err(denied) = self.check_tenant_isolation(ctx) {
            return denied;
        }

        // 获取继承树
        let hierarchy = match self.hierarchy.read() {
            Ok(h) => h,
            Err(_) => {
                return EvaluationResult::Denied {
                    reason: "engine internal error: hierarchy lock poisoned".into(),
                    denied_by_policy: None,
                }
            }
        };

        // 展开角色权限
        let permissions = hierarchy.resolve_permissions_multi(&ctx.subject.roles);

        // 策略评估（基于策略的检查）
        // 先收集适用的策略
        let policies = match self.collect_applicable_policies(ctx, &hierarchy) {
            Ok(p) => p,
            Err(e) => {
                return EvaluationResult::Denied {
                    reason: format!("policy evaluation error: {e}"),
                    denied_by_policy: None,
                }
            }
        };

        // 策略评估：拒绝优先
        let mut matched_allow = Vec::new();
        let matched_deny: Option<String> = None;

        for policy in &policies {
            // 检查资源匹配
            let resource_matches = policy
                .resource_patterns
                .iter()
                .any(|pattern| resource_pattern_matches(pattern, &ctx.resource.path));

            if !resource_matches {
                continue;
            }

            // 检查动作匹配
            let action_matches = policy.actions.iter().any(|a| a.matches(&ctx.action));
            if !action_matches {
                continue;
            }

            // 检查 ABAC 条件
            if self.config.abac_enabled {
                if let Some(condition) = &policy.condition {
                    match ConditionEvaluator::evaluate(condition, ctx) {
                        Ok(true) => {} // 条件满足，继续
                        Ok(false) => continue, // 条件不满足，跳过此策略
                        Err(e) => {
                            // 条件评估失败，默认安全失败（保守拒绝）
                            return EvaluationResult::Denied {
                                reason: format!("condition evaluation failed: {e}"),
                                denied_by_policy: Some(policy.id.clone()),
                            };
                        }
                    }
                }
            }

            // 策略匹配
            match policy.effect {
                Effect::Deny => {
                    // 拒绝优先：立即返回
                    return EvaluationResult::Denied {
                        reason: format!(
                            "denied by policy '{}' ({})",
                            policy.name, policy.id
                        ),
                        denied_by_policy: Some(policy.id.clone()),
                    };
                }
                Effect::Allow => {
                    matched_allow.push(policy.id.clone());
                }
            }
        }

        // 基于角色权限的检查（RBAC 核心）
        let role_permission_granted = permissions
            .iter()
            .any(|p| p.matches(&ctx.action, &ctx.resource.path));

        // 综合判断：策略允许 或 角色权限允许
        let is_granted = !matched_allow.is_empty() || role_permission_granted;

        if is_granted {
            // 如果是角色权限匹配但没有匹配的策略，添加一个虚拟标识
            if matched_allow.is_empty() {
                matched_allow.push("role-permission".into());
            }
            EvaluationResult::Granted {
                matched_policies: matched_allow,
            }
        } else if self.config.default_deny {
            EvaluationResult::Denied {
                reason: format!(
                    "role(s) '{}' lacks permission {}:{}",
                    ctx.subject.roles.join(", "),
                    ctx.action,
                    ctx.resource.path
                ),
                denied_by_policy: matched_deny,
            }
        } else {
            EvaluationResult::Granted {
                matched_policies: vec!["default-allow".into()],
            }
        }
    }

    /// 跨租户隔离检查
    fn check_tenant_isolation(&self, ctx: &EvaluationContext) -> Result<(), EvaluationResult> {
        if let (Some(resource_tenant), Some(subject_tenant)) =
            (&ctx.resource.tenant, ctx.subject.tenant())
        {
            // 获取角色权限以检查 admin 权限
            if let Ok(hierarchy) = self.hierarchy.read() {
                let permissions = hierarchy.resolve_permissions_multi(&ctx.subject.roles);
                let has_admin = permissions.iter().any(|p| {
                    p.action == Action::Admin || p.action == Action::All
                });

                if !has_admin && subject_tenant != resource_tenant {
                    return Err(EvaluationResult::Denied {
                        reason: format!(
                            "cross-tenant access denied: subject '{}' tenant '{}' != resource tenant '{}'",
                            ctx.subject.id, subject_tenant, resource_tenant
                        ),
                        denied_by_policy: None,
                    });
                }
            }
        }
        Ok(())
    }

    /// 收集适用的策略（按优先级排序）
    fn collect_applicable_policies(
        &self,
        ctx: &EvaluationContext,
        hierarchy: &RoleHierarchy,
    ) -> Result<Vec<Policy>, RbacError> {
        // 获取所有角色（包括继承的）
        let mut all_roles = ctx.subject.roles.clone();
        for role in &ctx.subject.roles {
            all_roles.extend(hierarchy.all_ancestors(role));
        }
        // 去重
        let mut seen = std::collections::HashSet::new();
        all_roles.retain(|r| seen.insert(r.clone()));

        // 收集所有角色适用的策略
        let mut all_policies = Vec::new();
        let mut policy_ids = std::collections::HashSet::new();

        for role in &all_roles {
            let policies = self.store.find_policies_by_role(role)?;
            for p in policies {
                if policy_ids.insert(p.id.clone()) {
                    all_policies.push(p);
                }
            }
        }

        // 按优先级排序（数字越小优先级越高）
        all_policies.sort_by_key(|p| p.priority);

        Ok(all_policies)
    }

    // ── 继承树查询 ────────────────────────────────────────────────────────

    /// 获取角色的所有祖先
    pub fn get_role_ancestors(&self, role: &str) -> Result<Vec<String>, RbacError> {
        let hierarchy = self
            .hierarchy
            .read()
            .map_err(|e| RbacError::StoreError(format!("hierarchy lock poisoned: {e}")))?;
        Ok(hierarchy.all_ancestors(role))
    }

    /// 获取角色的所有后代
    pub fn get_role_descendants(&self, role: &str) -> Result<Vec<String>, RbacError> {
        let hierarchy = self
            .hierarchy
            .read()
            .map_err(|e| RbacError::StoreError(format!("hierarchy lock poisoned: {e}")))?;
        Ok(hierarchy.all_descendants(role))
    }

    /// 检查角色是否继承自另一个角色
    pub fn role_inherits_from(&self, role: &str, ancestor: &str) -> Result<bool, RbacError> {
        let hierarchy = self
            .hierarchy
            .read()
            .map_err(|e| RbacError::StoreError(format!("hierarchy lock poisoned: {e}")))?;
        Ok(hierarchy.inherits_from(role, ancestor))
    }

    /// 展开角色权限
    pub fn resolve_permissions(&self, role: &str) -> Result<Vec<Permission>, RbacError> {
        let hierarchy = self
            .hierarchy
            .read()
            .map_err(|e| RbacError::StoreError(format!("hierarchy lock poisoned: {e}")))?;
        Ok(hierarchy.resolve_permissions(role))
    }

    /// 展开多个角色的权限
    pub fn resolve_permissions_multi(&self, roles: &[String]) -> Result<Vec<Permission>, RbacError> {
        let hierarchy = self
            .hierarchy
            .read()
            .map_err(|e| RbacError::StoreError(format!("hierarchy lock poisoned: {e}")))?;
        Ok(hierarchy.resolve_permissions_multi(roles))
    }

    // ── 重载 ──────────────────────────────────────────────────────────────

    /// 从存储重载所有策略和角色
    pub fn reload(&self) -> Result<(), RbacError> {
        self.refresh_hierarchy()?;
        let count = self.invalidate_cache_for_reload();

        let role_count = self.store.list_roles().map(|r| r.len()).unwrap_or(0);
        let policy_count = self.store.list_policies().map(|p| p.len()).unwrap_or(0);

        self.dispatch_event(RbacEvent::PolicyReloaded {
            policy_count,
            role_count,
        });

        let _ = count; // 使编译器满意
        Ok(())
    }

    fn invalidate_cache_for_reload(&self) -> usize {
        if let Some(cache) = &self.cache {
            cache.invalidate_all().unwrap_or(0)
        } else {
            0
        }
    }
}

// ── 辅助函数 ────────────────────────────────────────────────────────────────

/// 资源模式匹配
fn resource_pattern_matches(pattern: &str, resource: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    // 尾缀通配：db:prod/* 匹配 db:prod/a、db:prod/a/b
    if let Some(stripped) = pattern.strip_suffix("/*") {
        return resource.starts_with(stripped)
            && resource.as_bytes().get(stripped.len()) == Some(&b'/');
    }
    // 前缀通配：db:* 匹配 db:anything
    if let Some(stripped) = pattern.strip_suffix('*') {
        return resource.starts_with(stripped);
    }
    // 精确匹配
    pattern == resource
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Action, Effect, EvaluationContext, Policy, Resource, Subject};

    fn make_engine() -> RbacEngine {
        RbacEngine::with_builtin_roles()
    }

    // ── 基本权限检查 ──

    #[test]
    fn admin_can_do_anything() {
        let engine = make_engine();
        let result = engine.check(
            "user:admin",
            &["admin".into()],
            "write",
            "db:prod/anything",
        );
        assert!(result.is_granted());
    }

    #[test]
    fn editor_can_write_test() {
        let engine = make_engine();
        let result = engine.check(
            "user:bob",
            &["editor".into()],
            "write",
            "db:test/data",
        );
        assert!(result.is_granted());
    }

    #[test]
    fn viewer_cannot_write_prod() {
        let engine = make_engine();
        let result = engine.check(
            "user:alice",
            &["viewer".into()],
            "write",
            "db:prod/citizen_info",
        );
        assert!(!result.is_granted());
        assert!(result.denied_reason().unwrap().contains("viewer"));
    }

    #[test]
    fn empty_roles_denied() {
        let engine = make_engine();
        let result = engine.check("user:frank", &[], "read", "db:test");
        assert!(!result.is_granted());
        assert!(result.denied_reason().unwrap().contains("no roles"));
    }

    #[test]
    fn multiple_roles_combine() {
        let engine = make_engine();
        // viewer + auditor 应该能读 audit
        let result = engine.check(
            "user:multi",
            &["viewer".into(), "auditor".into()],
            "read",
            "audit:logs",
        );
        assert!(result.is_granted());
    }

    // ── 租户隔离 ──

    #[test]
    fn cross_tenant_denied() {
        let engine = make_engine();
        let result = engine.check_with_tenant(
            "tenant:A:user:alice",
            &["editor".into()],
            "write",
            "db:prod/data",
            "tenant:B",
        );
        assert!(!result.is_granted());
        assert!(result.denied_reason().unwrap().contains("cross-tenant"));
    }

    #[test]
    fn cross_tenant_admin_bypasses() {
        let engine = make_engine();
        let result = engine.check_with_tenant(
            "tenant:A:user:alice",
            &["admin".into()],
            "write",
            "db:prod/data",
            "tenant:B",
        );
        // admin 有 admin 权限，应绕过租户隔离
        assert!(result.is_granted());
    }

    // ── 继承 ──

    #[test]
    fn role_inheritance_works() {
        let engine = make_engine();
        // editor 继承 viewer，所以应该能读
        let result = engine.check(
            "user:bob",
            &["editor".into()],
            "read",
            "db:prod/data",
        );
        assert!(result.is_granted());
    }

    #[test]
    fn get_role_ancestors() {
        let engine = make_engine();
        let ancestors = engine.get_role_ancestors("admin").unwrap();
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.iter().any(|a| a == "editor"));
        assert!(ancestors.iter().any(|a| a == "viewer"));
    }

    #[test]
    fn role_inherits_from_check() {
        let engine = make_engine();
        assert!(engine.role_inherits_from("admin", "viewer").unwrap());
        assert!(engine.role_inherits_from("editor", "viewer").unwrap());
        assert!(!engine.role_inherits_from("viewer", "admin").unwrap());
    }

    // ── 策略评估 ──

    #[test]
    fn policy_allow_takes_effect() {
        let engine = make_engine();

        // 添加一个允许策略：viewer 可以写 db:test/special
        engine
            .add_policy(
                Policy::new("p-viewer-special-write", "viewer special write", Effect::Allow)
                    .for_role("viewer")
                    .on_resource("db:test/special/*")
                    .with_action(Action::Write)
                    .with_priority(10),
            )
            .unwrap();

        let result = engine.check(
            "user:viewer1",
            &["viewer".into()],
            "write",
            "db:test/special/report",
        );
        assert!(result.is_granted());
    }

    #[test]
    fn policy_deny_overrides_allow() {
        let engine = make_engine();

        // 添加一个拒绝策略：editor 不能写 db:test/secret
        engine
            .add_policy(
                Policy::new("p-deny-editor-secret", "deny editor secret", Effect::Deny)
                    .for_role("editor")
                    .on_resource("db:test/secret/*")
                    .with_action(Action::Write)
                    .with_priority(1), // 高优先级
            )
            .unwrap();

        // editor 通常可以写 db:test/*，但被拒绝策略覆盖
        let result = engine.check(
            "user:bob",
            &["editor".into()],
            "write",
            "db:test/secret/data",
        );
        assert!(!result.is_granted());
        assert!(result.denied_reason().unwrap().contains("deny-editor-secret"));
    }

    // ── ABAC 条件 ──

    #[test]
    fn abac_condition_granted_when_met() {
        let engine = make_engine();

        engine
            .add_policy(
                Policy::new("p-abac-owner", "owner can write", Effect::Allow)
                    .for_role("viewer")
                    .on_resource("doc:*")
                    .with_action(Action::Write)
                    .with_condition("resource.owner == subject.id")
                    .with_priority(10),
            )
            .unwrap();

        let subject = Subject::new("user:alice", vec!["viewer".into()]);
        let resource = Resource::new("doc:report-1").with_attr("owner", "user:alice");
        let ctx = EvaluationContext::new(subject, resource, Action::Write);

        let result = engine.evaluate(&ctx);
        assert!(result.is_granted());
    }

    #[test]
    fn abac_condition_denied_when_not_met() {
        let engine = make_engine();

        engine
            .add_policy(
                Policy::new("p-abac-owner2", "owner can write", Effect::Allow)
                    .for_role("viewer")
                    .on_resource("doc:*")
                    .with_action(Action::Write)
                    .with_condition("resource.owner == subject.id")
                    .with_priority(10),
            )
            .unwrap();

        let subject = Subject::new("user:alice", vec!["viewer".into()]);
        let resource = Resource::new("doc:report-1").with_attr("owner", "user:bob");
        let ctx = EvaluationContext::new(subject, resource, Action::Write);

        let result = engine.evaluate(&ctx);
        // 条件不满足，策略不适用 → 默认拒绝（viewer 没有 doc 的写权限）
        assert!(!result.is_granted());
    }

    // ── 缓存 ──

    #[test]
    fn cache_improves_performance() {
        let engine = make_engine();

        // 第一次检查（未命中缓存）
        let result1 = engine.check("user:test", &["viewer".into()], "read", "db:test/data");
        assert!(result1.is_granted());

        // 第二次检查（应命中缓存）
        let result2 = engine.check("user:test", &["viewer".into()], "read", "db:test/data");
        assert!(result2.is_granted());

        let stats = engine.cache_stats().unwrap();
        assert!(stats.hits >= 1);
        assert!(stats.misses >= 1);
    }

    #[test]
    fn cache_invalidated_on_role_change() {
        let engine = make_engine();

        // 先做一次检查填充缓存
        engine.check("user:test", &["viewer".into()], "read", "db:test/data");
        let stats_before = engine.cache_stats().unwrap();
        assert_eq!(stats_before.size, 1);

        // 添加新角色，应使缓存失效
        engine
            .add_role(Role::new("test_role").grant_str("read", "test:*"))
            .unwrap();

        let stats_after = engine.cache_stats().unwrap();
        assert_eq!(stats_after.size, 0);
    }

    // ── 事件 ──

    #[test]
    fn events_dispatched_on_access() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use crate::events::{EventCategory, FnEventListener};

        let engine = make_engine();
        let count = Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();

        let listener = FnEventListener::new(move |_evt| {
            count_clone.fetch_add(1, Ordering::Relaxed);
        })
        .with_filter(EventCategory::AccessDecision);

        engine.add_listener(Arc::new(listener));

        engine.check("user:test", &["viewer".into()], "read", "db:test/data");
        engine.check("user:test", &["viewer".into()], "write", "db:prod/data");

        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    // ── 角色管理 ──

    #[test]
    fn add_and_remove_role() {
        let engine = make_engine();
        let initial_count = engine.list_roles().unwrap().len();

        engine
            .add_role(Role::new("custom_role").grant_str("read", "custom:*"))
            .unwrap();
        assert_eq!(engine.list_roles().unwrap().len(), initial_count + 1);
        assert!(engine.get_role("custom_role").unwrap().is_some());

        assert!(engine.delete_role("custom_role").unwrap());
        assert_eq!(engine.list_roles().unwrap().len(), initial_count);
        assert!(engine.get_role("custom_role").unwrap().is_none());
    }

    // ── 权限展开 ──

    #[test]
    fn resolve_permissions_multi() {
        let engine = make_engine();
        let perms = engine
            .resolve_permissions_multi(&["viewer".into(), "auditor".into()])
            .unwrap();

        // viewer 有 4 个权限，auditor 有 4 个权限，但 read:db:* 和 read:flow:* 和 read:mem:* 重复
        // 所以总共 4 + 1 = 5 个（auditor 多了 read:audit:*）
        assert!(perms.len() >= 5);
    }

    // ── 默认拒绝 ──

    #[test]
    fn default_deny_behavior() {
        let engine = RbacEngine::empty();
        // 空引擎，没有任何角色/策略，默认拒绝
        let result = engine.check("user:test", &["unknown".into()], "read", "anything");
        assert!(!result.is_granted());
    }
}
