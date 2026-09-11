// =====================================================================
// system/tenant.rs — 租户管理（多租户企业级）
// =====================================================================

use crate::GatewayState;
use crate::system::{ok, err};
use axum::extract::{Path, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Value};

fn tenant_json(t: &mox_platform_iam_core::IamTenant) -> Value {
    json!({
        "id": t.tenant_id,
        "code": t.tenant_code,
        "name": t.tenant_name,
        "mode": t.tenant_mode,
        "status": t.tenant_status,
        "plan": t.tenant_plan,
        "createdAt": t.created_at,
        "updatedAt": t.updated_at,
    })
}

/// GET /api/tenant — 租户列表
pub(crate) async fn list_tenants(State(s): State<GatewayState>) -> ApiResponse<Value> {
    match s.iam.list_tenants() {
        Ok(list) => ok(json!(list.iter().map(tenant_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("tenant list: {e}")),
    }
}

/// GET /api/tenant/:id — 租户详情
pub(crate) async fn get_tenant_detail(State(s): State<GatewayState>, Path(id): Path<String>) -> ApiResponse<Value> {
    match s.iam.get_tenant(&id) {
        Ok(Some(t)) => ok(tenant_json(&t)),
        Ok(None) => err("tenant not found"),
        Err(e) => err(&format!("tenant detail: {e}")),
    }
}

/// POST /api/tenant — 创建租户
pub(crate) async fn create_tenant_handler(State(s): State<GatewayState>, Json(body): Json<Value>) -> ApiResponse<Value> {
    let code = body.get("code").and_then(|v| v.as_str()).unwrap_or("");
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let mode = body.get("mode").and_then(|v| v.as_str());
    let plan = body.get("plan").and_then(|v| v.as_str());
    if code.is_empty() || name.is_empty() {
        return err("tenant code and name are required");
    }
    match s.iam.create_tenant(code, name, mode, plan) {
        Ok(t) => ok(tenant_json(&t)),
        Err(e) => err(&format!("tenant create: {e}")),
    }
}

/// PUT /api/tenant/:id — 更新租户
pub(crate) async fn update_tenant_handler(State(s): State<GatewayState>, Path(id): Path<String>, Json(body): Json<Value>) -> ApiResponse<Value> {
    let name = body.get("name").and_then(|v| v.as_str());
    let status = body.get("status").and_then(|v| v.as_str());
    let plan = body.get("plan").and_then(|v| v.as_str());
    match s.iam.update_tenant(&id, name, status, plan) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("tenant update: {e}")),
    }
}

/// DELETE /api/tenant/:id — 删除租户
pub(crate) async fn delete_tenant_handler(State(s): State<GatewayState>, Path(id): Path<String>) -> ApiResponse<Value> {
    match s.iam.delete_tenant(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("tenant delete: {e}")),
    }
}

/// GET /api/tenant/switch/:id — 切换当前租户（设置响应头，前端据此切换）
pub(crate) async fn switch_tenant(State(s): State<GatewayState>, Path(id): Path<String>) -> ApiResponse<Value> {
    match s.iam.get_tenant(&id) {
        Ok(Some(t)) => ok(json!({
            "tenantId": t.tenant_id,
            "tenantCode": t.tenant_code,
            "tenantName": t.tenant_name,
            "switched": true,
        })),
        Ok(None) => err("tenant not found"),
        Err(e) => err(&format!("tenant switch: {e}")),
    }
}
