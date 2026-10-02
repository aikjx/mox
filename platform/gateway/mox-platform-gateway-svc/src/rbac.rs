// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! RBAC 域路由（L1）：`/rbac/v1/*` —— 角色 / 权限 / 当前用户
//!
//! # 数据口径（与 system 域一致，IAM 真实 SQLite 仓储）
//! - 租户和用户来自验签身份，查询参数不能覆盖作用域；
//! - 角色列表来自 `IamRepository::list_roles`；
//! - 用户权限来自 `IamRepository::get_user_permissions`（角色-权限授予真实链路）；
//! - `/rbac/v1/current` 使用可信当前用户和租户（`/api/auth/me` 同源）。
//!
//! # 状态
//! `ready`：三个端点全部对接 IAM 仓储真实数据，无 stub。

use crate::GatewayState;
use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use mox_api_protocol::{api_ok, ApiResponse};
use serde_json::{json, Value};
use std::collections::HashMap;

/// GET /rbac/v1/roles —— 角色列表（IAM 仓储真实现）
async fn rbac_roles(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    crate::auth::ApiAuth(_identity): crate::auth::ApiAuth,
    crate::alliance::experts_common::TenantId(tenant): crate::alliance::experts_common::TenantId,
) -> ApiResponse<Value> {
    if q.get("tenant_id").is_some_and(|value| value != &tenant) {
        return mox_api_protocol::api_error(403, "Tenant scope conflicts with trusted identity");
    }
    let roles = match s.iam.list_roles(&tenant) {
        Ok(value) => value,
        Err(_) => return mox_api_protocol::api_error(503, "IAM unavailable"),
    };
    api_ok(json!({
        "tenant_id": tenant,
        "total": roles.len(),
        "roles": roles.iter().map(|r| json!({
            "role_id": r.role_id,
            "code": r.role_code,
            "name": r.role_name,
            "type": r.role_type,
            "is_builtin": r.is_builtin,
            "data_scope": r.data_scope,
            "status": r.status,
            "parent_id": r.parent_id,
            "sort_order": r.sort_order,
        })).collect::<Vec<_>>(),
    }))
}

/// GET /rbac/v1/permissions —— 指定用户权限授予清单（IAM 角色-权限真实链路）
async fn rbac_permissions(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    crate::auth::ApiAuth(identity): crate::auth::ApiAuth,
    crate::alliance::experts_common::TenantId(tenant): crate::alliance::experts_common::TenantId,
) -> ApiResponse<Value> {
    if q.get("tenant_id").is_some_and(|value| value != &tenant) {
        return mox_api_protocol::api_error(403, "Tenant scope conflicts with trusted identity");
    }
    if q.get("user_id").is_some_and(|value| value != &identity.id) {
        return mox_api_protocol::api_error(403, "User scope conflicts with trusted identity");
    }
    let user = identity.id;
    let perms = match s.iam.get_user_permissions(&tenant, &user) {
        Ok(value) => value,
        Err(_) => return mox_api_protocol::api_error(503, "IAM unavailable"),
    };
    api_ok(json!({
        "tenant_id": tenant,
        "user_id": user,
        "total": perms.len(),
        "permissions": perms,
    }))
}

/// GET /rbac/v1/current —— 当前用户角色 + 权限摘要（Bearer 解析，/api/auth/me 同源）
async fn rbac_current(
    crate::auth::ApiAuth(user): crate::auth::ApiAuth,
    crate::alliance::experts_common::TenantId(tenant): crate::alliance::experts_common::TenantId,
    State(s): State<GatewayState>,
) -> ApiResponse<Value> {
    let user_id = user.id.clone();
    let roles = match s.iam.get_user_roles(&tenant, &user_id) {
        Ok(value) => value,
        Err(_) => return mox_api_protocol::api_error(503, "IAM unavailable"),
    };
    let perms = match s.iam.get_user_permissions(&tenant, &user_id) {
        Ok(value) => value,
        Err(_) => return mox_api_protocol::api_error(503, "IAM unavailable"),
    };
    api_ok(json!({
        "user_id": user_id,
        "tenant_id": tenant,
        "roles": roles.iter().map(|r| json!({
            "id": r.role_id, "code": r.role_code, "name": r.role_name,
        })).collect::<Vec<_>>(),
        "permissions": perms,
    }))
}

/// 装配 RBAC 域路由（受保护：随系统域进入鉴权层）
pub fn build_rbac_router() -> Router<GatewayState> {
    Router::new()
        .route("/rbac/v1/roles", get(rbac_roles))
        .route("/rbac/v1/permissions", get(rbac_permissions))
        .route("/rbac/v1/current", get(rbac_current))
}
