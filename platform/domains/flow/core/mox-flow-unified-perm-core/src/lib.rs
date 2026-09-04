// Copyright (c) 2026 璇玑 RelGraph · 统一权限核心 (Unified Permission Core)
// Licensed under the MIT License.

//! 统一权限核心
//!
//! mox 模块化系统架构维度权限管理系统，支持：
//! - RBAC（基于角色的访问控制）
//! - ABAC（基于属性的访问控制）
//! - 多租户隔离
//! - 数据级权限
//! - SSO 单点登录
//! - 策略引擎

pub mod error;
pub mod types;
pub mod rbac;
pub mod abac;
pub mod tenant;
pub mod data_perm;
pub mod policy_engine;
pub mod sso;

pub use error::{PermError, PermResult};
pub use types::{
    Action, Permission, PermissionEffect, ResourceScope, Role, RoleBinding, Subject,
    SubjectType, Tenant, TenantStatus, User, UserStatus,
};
pub use rbac::RbacManager;
pub use abac::{AbacEngine, AbacPolicy, AttributeValue};
pub use tenant::TenantManager;
pub use data_perm::{DataPermissionManager, DataScope, DataFilterRule};
pub use policy_engine::PolicyEngine;
pub use sso::{SsoManager, SsoProvider, SsoConfig, TokenInfo};
