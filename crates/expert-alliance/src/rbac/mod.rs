//! RBAC 引擎 — 资源级权限控制
//!
//! 相比原有 `roles: Vec<String>` 的角色白名单，RBAC Engine 提供：
//! - **资源级粒度**：`db:prod/*` 写权限 ≠ `db:test/*` 写权限
//! - **继承链**：`editor` 继承 `viewer` 全部权限，无需重复列举
//! - **拒绝理由**：`check` 返回 `Denied(String)` 含具体缺失的权限路径
//! - **与审计集成**：`check()` 失败自动调用外部审计

pub mod check;
pub mod policy;
pub mod error;

// Re-export from both submodules
pub use check::{PermissionCheck, PermissionResult, check, check_with_audit};
pub use policy::{Permission, RbacPolicy, RoleDef, BuiltinRoles, POLICY};
pub use error::RbacError;
