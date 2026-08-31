// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX 平台级 RBAC 权限引擎
//!
//! 企业级访问控制引擎，支持 RBAC + ABAC 混合模型。
//!
//! # 核心特性
//!
//! - **资源级粒度**：`db:prod/*` 写权限 ≠ `db:test/*` 写权限
//! - **角色继承链**：`editor` 继承 `viewer` 全部权限，支持多继承
//! - **策略引擎**：基于策略的访问控制，拒绝优先（Deny-overrides）
//! - **ABAC 支持**：基于属性的条件表达式（`subject.department == resource.owner`）
//! - **缓存层**：LRU 评估结果缓存，策略变更自动失效
//! - **可插拔存储**：`PolicyStore` trait，内存实现开箱即用，可扩展数据库/远程服务
//! - **事件系统**：策略变更、权限决策、缓存事件，支持监听器
//! - **跨租户隔离**：基于租户前缀的自动隔离，admin 可绕过
//! - **统一错误码**：PL05xxx 系列，可集成 `mox-error`
//!
//! # 模块结构
//!
//! - [`types`] — 核心类型（Action, Effect, Subject, Resource, Permission, Role, Policy, EvaluationContext, EvaluationResult）
//! - [`engine`] — 评估引擎（RbacEngine，统一入口）
//! - [`hierarchy`] — 角色继承树（DAG，多继承，循环检测）
//! - [`store`] — 策略存储（PolicyStore trait + MemoryPolicyStore）
//! - [`cache`] — 评估结果缓存（LRU，自动失效）
//! - [`abac`] — ABAC 条件表达式求值器
//! - [`error`] — 错误类型（RbacError，PL05xxx 错误码）
//! - [`events`] — 事件系统（RbacEvent + EventListener）
//!
//! # Feature Flags
//!
//! - `default` — 核心功能（无额外依赖）
//! - `mox-error` — 集成 `mox-error` 统一错误码系统
//! - `audit` — 集成 `mox-audit`，权限拒绝自动产生审计事件
//! - `serde` — 启用序列化/反序列化支持
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mox_rbac_engine::RbacEngine;
//!
//! // 使用内置角色初始化引擎
//! let engine = RbacEngine::with_builtin_roles();
//!
//! // 检查权限
//! let result = engine.check(
//!     "user:alice",
//!     &["editor".into()],
//!     "write",
//!     "db:test/citizen_info",
//! );
//!
//! match result {
//!     mox_rbac_engine::EvaluationResult::Granted { .. } => println!("权限通过"),
//!     mox_rbac_engine::EvaluationResult::Denied { reason, .. } => println!("权限拒绝: {}", reason),
//! }
//! ```
//!
//! # ABAC 示例
//!
//! ```rust,ignore
//! use mox_rbac_engine::{RbacEngine, Policy, Effect, Action};
//!
//! let engine = RbacEngine::with_builtin_roles();
//!
//! // 添加带 ABAC 条件的策略：只有文档所有者可以写
//! engine.add_policy(
//!     Policy::new("owner-write", "Owner can write", Effect::Allow)
//!         .for_role("viewer")
//!         .on_resource("doc:*")
//!         .with_action(Action::Write)
//!         .with_condition("resource.owner == subject.id"),
//! ).unwrap();
//! ```
//!
//! # 错误码规范
//!
//! 错误码格式：`PL05XXX`（Platform 域，模块 05 = RBAC 权限引擎）
//!
//! | 错误码 | 类型 | 说明 |
//! |--------|------|------|
//! | PL05001 | RoleNotFound | 角色不存在 |
//! | PL05002 | PolicyNotFound | 策略不存在 |
//! | PL05003 | CyclicInheritance | 循环继承 |
//! | PL05004 | PolicyLoadFailed | 策略加载失败 |
//! | PL05005 | StoreError | 存储错误 |
//! | PL05006 | CacheError | 缓存错误 |
//! | PL05007 | ConditionParseError | 条件表达式解析失败 |
//! | PL05008 | ConditionEvalError | 条件表达式评估失败 |
//! | PL05009 | AuditWriteFailed | 审计写入失败 |
//! | PL05010 | PolicyInitFailed | 策略初始化失败 |
//! | PL05011 | InvalidResourcePath | 无效资源路径 |
//! | PL05012 | InvalidRoleName | 无效角色名 |
//! | PL05013 | IncompleteContext | 上下文不完整 |
//! | PL05014 | EngineNotInitialized | 引擎未初始化 |
//! | PL05015 | ConfigError | 配置错误 |

// ── 模块声明 ────────────────────────────────────────────────────────────────

pub mod abac;
pub mod cache;
pub mod engine;
pub mod error;
pub mod events;
pub mod hierarchy;
pub mod store;
pub mod types;

// ── 重导出（核心类型） ─────────────────────────────────────────────────────

// 引擎
pub use engine::{RbacEngine, RbacEngineConfig};

// 核心类型
pub use types::{
    Action, AttributeValue, Attributes, BuiltinRoles, Effect, EvaluationContext,
    EvaluationResult, Permission, Policy, Resource, Role, Subject,
};

// 角色继承
pub use hierarchy::RoleHierarchy;

// 存储
pub use store::{MemoryPolicyStore, PolicyStore};

// 缓存
pub use cache::{CacheKey, CacheStats, EvaluationCache};

// 错误
pub use error::{ErrorLevel, RbacError};

// 事件
pub use events::{
    EventCategory, EventEnvelope, EventListener, FnEventListener, RbacEvent,
};

// ── 便捷类型别名 ────────────────────────────────────────────────────────────

/// RBAC 结果类型别名
pub type RbacResult<T> = Result<T, RbacError>;

// ── 向后兼容（v0.1 API） ────────────────────────────────────────────────────
// 保留旧 API 以兼容现有使用方（如 mox-ai-expert-svc）

/// 旧版权限检查上下文（兼容 mox-ai-expert-svc）
#[deprecated(note = "Use EvaluationContext instead")]
pub type PermissionCheck = types::EvaluationContext;

/// 旧版权限结果（兼容）
#[deprecated(note = "Use EvaluationResult instead")]
pub type PermissionResult = types::EvaluationResult;

/// 旧版角色定义（兼容）
#[deprecated(note = "Use Role instead")]
pub type RoleDef = types::Role;

/// 旧版策略容器（兼容）
#[deprecated(note = "Use RbacEngine + MemoryPolicyStore instead")]
pub type RbacPolicy = hierarchy::RoleHierarchy;

/// 全局默认策略（兼容旧 API）
///
/// > 注意：新代码应使用 `RbacEngine::with_builtin_roles()` 替代。
/// > 此静态变量仅为向后兼容保留。
pub static POLICY: std::sync::LazyLock<std::sync::RwLock<hierarchy::RoleHierarchy>> =
    std::sync::LazyLock::new(|| {
        let roles = types::BuiltinRoles::all();
        let hierarchy = hierarchy::RoleHierarchy::from_roles(roles)
            .expect("builtin roles should not have cycles");
        std::sync::RwLock::new(hierarchy)
    });

/// 简单权限检查函数（兼容旧 API）
///
/// > 注意：新代码应使用 `RbacEngine::check()` 替代。
/// > 此函数使用全局 POLICY 静态变量，不支持缓存、ABAC、事件等高级特性。
#[deprecated(note = "Use RbacEngine::check() instead")]
pub fn check(ctx: &types::EvaluationContext) -> types::EvaluationResult {
    use std::sync::OnceLock;
    static ENGINE: OnceLock<RbacEngine> = OnceLock::new();
    let engine = ENGINE.get_or_init(|| RbacEngine::with_builtin_roles());
    engine.evaluate(ctx)
}

// ── 测试 ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn reexports_work() {
        // 验证重导出的类型可用
        let _p = Permission::from_str("read", "db:*");
        let _r = Role::new("test");
        let _e = RbacEngine::with_builtin_roles();
        let _h = RoleHierarchy::new();
        let _s = MemoryPolicyStore::new();
        let _c = EvaluationCache::new();
        let _err = RbacError::RoleNotFound("test".into());
        let _evt = RbacEvent::CacheHit { key: "k".into() };
    }

    #[test]
    fn rbac_result_type_alias() {
        let ok: RbacResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: RbacResult<i32> = Err(RbacError::RoleNotFound("test".into()));
        assert!(err.is_err());
    }

    #[test]
    fn engine_basic_check() {
        let engine = RbacEngine::with_builtin_roles();
        let result = engine.check(
            "user:admin",
            &["admin".into()],
            "write",
            "db:prod/anything",
        );
        assert!(result.is_granted());
    }

    #[test]
    fn global_policy_is_accessible() {
        let policy = POLICY.read().unwrap();
        assert!(policy.has_role("admin"));
        assert!(policy.has_role("viewer"));
        assert!(policy.has_role("editor"));
        assert_eq!(policy.len(), 6);
    }

    #[test]
    fn all_builtin_roles_present() {
        let roles = BuiltinRoles::all();
        assert_eq!(roles.len(), 6);
    }
}
