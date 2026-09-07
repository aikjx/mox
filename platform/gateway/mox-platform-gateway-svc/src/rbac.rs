// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! RBAC 域路由（L1）：`/rbac/v1/*` —— 角色 / 权限 / 当前用户
//!
//! # 数据口径（与 system 域一致，IAM 真实 SQLite 仓储）
//! - 默认租户 `T001`（种子演示租户），可用 `?tenant_id=` 覆盖；
//! - 角色列表来自 `IamRepository::list_roles`；
//! - 用户权限来自 `IamRepository::get_user_permissions`（角色-权限授予真实链路）；
//! - `/rbac/v1/current` 从 Bearer 解析当前用户（`/api/auth/me` 同源）。
//!
//! # 状态
//! `ready`：三个端点全部对接 IAM 仓储真实数据，无 stub。

use crate::GatewayState;
use mox_api_protocol::{ApiResponse, api_ok};
use axum::{
    extract::{Query, State},
    routing::get,
    Router,
};
use serde_json::{Value, json};
use std::collections::HashMap;

/// 种子演示租户（与 system.rs 口径一致）
const DEFAULT_TENANT: &str = "T001";
/// 种子超级管理员 user_id（与 system.rs 口径一致）
const DEFAULT_USER: &str = "admin-user";

fn q_str(q: &HashMap<String, String>, key: &str, default: &str) -> String {
    q.get(key).cloned().unwrap_or_else(|| default.to_string())
}

/// 解析租户参数（与 system.rs `resolve_tenant` 同逻辑）
fn resolve_tenant(s: &GatewayState, input: &str) -> Result<String, String> {
    if let Ok(Some(_)) = s.iam.get_tenant(input) {
        return Ok(input.to_string());
    }
    if let Some(t) = s.iam.find_tenant_by_code(input) {
        return Ok(t.tenant_id);
    }
    Ok(input.to_string())
}

/// GET /rbac/v1/roles —— 角色列表（IAM 仓储真实现）
async fn rbac_roles(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return api_ok(json!({"ok": false, "error": format!("tenant resolve: {e}")})),
    };
    let roles = s.iam.list_roles(&tenant).unwrap_or_default();
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
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return api_ok(json!({"ok": false, "error": format!("tenant resolve: {e}")})),
    };
    let user = q_str(&q, "user_id", DEFAULT_USER);
    let perms = s.iam.get_user_permissions(&tenant, &user).unwrap_or_default();
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
    State(s): State<GatewayState>,
) -> ApiResponse<Value> {
    let tenant = DEFAULT_TENANT.to_string();
    let user_id = user.id.clone();
    let roles = s.iam.get_user_roles(&tenant, &user_id).unwrap_or_default();
    let perms = s.iam.get_user_permissions(&tenant, &user_id).unwrap_or_default();
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
