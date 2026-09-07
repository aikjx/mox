// ====================================================================
// system/security.rs — 系统管理子模块
// ====================================================================

use crate::auth::ApiAuth;
use crate::GatewayState;
use crate::system::{DEFAULT_TENANT, DEFAULT_USER, ok, err, q_str, resolve_tenant, now_iso, status_flag, build_tree,
    opt_str, opt_i64, opt_status, user_json, dept_json, role_json, post_json,
    menu_json, dict_type_json, dict_data_json, config_json, oper_log_json,
    login_log_json, api_key_json};
use axum::extract::{Path, Query, State};
use axum::Json;
use mox_api_protocol::ApiResponse;
use serde_json::{json, Map, Value};
use std::collections::HashMap;

pub(crate) async fn security_status(State(s): State<GatewayState>) -> ApiResponse<Value> {
    ok(json!({
        "auth_enabled": s.config.auth.enabled,
        "rate_limit_enabled": s.config.rate_limit.enabled,
        "iam": "ready",
        "db": "sqlite",
        "default_tenant": DEFAULT_TENANT,
        "ts": now_iso(),
    }))
}

/// GET /api/security/api-keys —— 凭证列表（SQLite 持久化，api_key 脱敏）

pub(crate) async fn list_api_keys(State(s): State<GatewayState>) -> ApiResponse<Value> {
    match s.iam.list_api_keys(DEFAULT_TENANT) {
        Ok(list) => ok(json!(list
            .iter()
            .map(api_key_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("api key list: {e}")),
    }
}

/// POST /api/security/api-keys —— 创建凭证（生成明文 key，注册 auth 中间件 + 持久化 SQLite）

pub(crate) async fn create_api_key(
    State(s): State<GatewayState>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("api-key")
        .to_string();
    let key = format!("mox_{}", uuid::Uuid::new_v4().simple());
    s.auth.register_api_key(DEFAULT_USER, &key);
    match s.iam.create_api_key(DEFAULT_TENANT, &name, &key, Some(DEFAULT_USER), None) {
        Ok(k) => ok(json!({
            "id": k.key_id,
            "name": k.name,
            "api_key": key,
            "active": true,
            "createdAt": k.created_at,
        })),
        Err(e) => err(&format!("api key create: {e}")),
    }
}

/// DELETE /api/security/api-keys/:id —— 吊销凭证（DB 吊销 + auth 中间件移除）

pub(crate) async fn revoke_api_key(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    // 先从 DB 取出原始 key，用于从 auth 中间件内存表中移除
    if let Ok(Some(k)) = s.iam.get_api_key(&id) {
        s.auth.revoke_api_key(&k.api_key);
    }
    match s.iam.revoke_api_key(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("api key revoke: {e}")),
    }
}

/// POST /api/security/validate —— 校验凭证明文

pub(crate) async fn validate_api_key(
    State(s): State<GatewayState>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let key = body.get("api_key").and_then(|v| v.as_str()).unwrap_or("");
    match s.auth.validate_api_key(key) {
        Some(uid) => ok(json!({
            "valid": true,
            "name": "api-key",
            "user_id": uid,
            "permissions": ["read", "write"],
        })),
        None => ok(json!({ "valid": false, "reason": "key not found or revoked" })),
    }
}

/// GET /api/security/audit-log —— 审计日志（SQLite 读取）

pub(crate) async fn audit_log(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_audit_logs(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|l| json!({
                "id": l.log_id,
                "action": l.action,
                "actionDetail": l.action_detail,
                "userId": l.user_id,
                "userIp": l.user_ip,
                "resourceType": l.resource_type,
                "resourceId": l.resource_id,
                "statusCode": l.status_code,
                "httpMethod": l.http_method,
                "httpPath": l.http_path,
                "latencyMs": l.latency_ms,
                "createdAt": l.created_at,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("audit log: {e}")),
    }
}

