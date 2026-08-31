// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX 平台级 RBAC 权限引擎
//!
//! 相比简单的 `roles: Vec<String>` 角色白名单，RBAC Engine 提供：
//! - **资源级粒度**：`db:prod/*` 写权限 ≠ `db:test/*` 写权限
//! - **继承链**：`editor` 继承 `viewer` 全部权限，无需重复列举
//! - **拒绝理由**：`check` 返回 `Denied(String)` 含具体缺失的权限路径
//! - **与审计集成**：`check_with_audit()` 失败自动调用外部审计（`audit` feature）
//!
//! # 模块结构
//!
//! - [`policy`] — RBAC 策略模型 + 角色继承链 + 全局默认策略
//! - [`check`] — 权限检查器 + 资源级检查 + 跨租户隔离
//! - [`error`] — `RbacError` 错误类型（可集成 `mox-error`）
//!
//! # Feature Flags
//!
//! - `default` — 无额外依赖，仅核心功能
//! - `mox-error` — 集成 `mox-error` 统一错误码系统
//! - `audit` — 集成 `mox-audit`，权限拒绝自动产生审计事件
//! - `serde` — 启用序列化/反序列化支持
//!
//! # 快速开始
//!
//! ```rust,ignore
//! use mox_rbac_engine::{check, PermissionCheck, Resource, PermissionResult};
//!
//! let ctx = PermissionCheck::new(
//!     "user:alice",
//!     vec!["editor".into()],
//!     "write",
//!     Resource::new("db:test/citizen_info"),
//! );
//!
//! match check(&ctx) {
//!     PermissionResult::Granted => println!("权限通过"),
//!     PermissionResult::Denied(reason) => println!("权限拒绝: {}", reason),
//! }
//! ```
//!
//! # 审计集成（audit feature）
//!
//! ```rust,ignore
//! use mox_rbac_engine::check_with_audit;
//! use std::sync::Arc;
//!
//! let audit_ctx = Arc::new(mox_audit::AuditContext::new(/* ... */));
//! let result = check_with_audit(&ctx, Some(&audit_ctx));
//! ```

// ── 模块声明 ────────────────────────────────────────────────────

pub mod check;
pub mod error;
pub mod policy;

// ── 重导出 ──────────────────────────────────────────────────────

pub use check::{check, PermissionCheck, PermissionResult, Resource};

#[cfg(feature = "audit")]
pub use check::check_with_audit;

pub use error::RbacError;
pub use policy::{BuiltinRoles, Permission, RbacPolicy, RoleDef, POLICY};

// ── 便捷类型别名 ────────────────────────────────────────────────

/// RBAC 结果类型别名
pub type RbacResult<T> = Result<T, RbacError>;

// ── 测试 ────────────────────────────────────────────────────────

#[cfg(test)]
mod lib_tests {
    use super::*;

    #[test]
    fn reexports_work() {
        // 验证重导出的类型可用
        let _p = Permission::new("read", "db:*");
        let _r = RoleDef::new("test");
        let _pol = RbacPolicy::default();
        let _res = Resource::new("test:path");
        let _check = PermissionCheck::new("user:test", vec![], "read", Resource::new("x"));
        let _err = RbacError::RoleNotFound("test".into());
    }

    #[test]
    fn global_policy_is_accessible() {
        let policy = POLICY.read().unwrap();
        assert!(policy.has_role("admin"));
        assert!(policy.has_role("viewer"));
    }

    #[test]
    fn full_check_flow() {
        let ctx = PermissionCheck::new(
            "user:test",
            vec!["admin".into()],
            "write",
            Resource::new("db:prod/anything"),
        );
        assert!(check(&ctx).is_granted());
    }

    #[test]
    fn rbac_result_type_alias() {
        let ok: RbacResult<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: RbacResult<i32> = Err(RbacError::RoleNotFound("test".into()));
        assert!(err.is_err());
    }
}
