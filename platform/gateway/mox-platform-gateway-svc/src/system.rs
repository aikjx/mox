// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 系统管理 + 安全域路由（L5）：`/api/system/*` · `/api/security/*`
//!
//! # 迁移策略（企业级分层收敛）
//! - **读接口**：IAM 仓储真实现（部门 / 角色 / 用户角色 / 菜单树 / 权限），
//!   并把 IAM 蛇形模型映射为前端期望的驼峰形状（`menu_id → id` 等）。
//! - **写接口 / 未落库域**（岗位/字典/参数配置/操作日志/登录日志/API Key 列表/审计）：
//!   返回标准 `{success, data}` 信封的 stub，避免 404 与前端 mock 兜底，
//!   待 IAM 仓储补充对应写方法后逐域收敛为真实现。
//! - **认证**：迁移期 `/api/system`、`/api/security` 位于 `config.public_paths`
//!   （前端 dev 令牌非合法 JWT），生产环境须移出并回收为受保护路由。
//!
//! # 数据口径
//! 默认租户为种子演示租户 `T001`（含部门/管理员），可通过 `?tenant_id=` 覆盖；
//! 用户相关读接口默认 `admin-user`（种子超级管理员）。

use crate::GatewayState;
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// 种子演示租户（seed_builtins 中名为"企业演示租户"，含部门 D001 与管理员）
const DEFAULT_TENANT: &str = "T001";
/// 种子超级管理员 user_id
const DEFAULT_USER: &str = "admin-user";

// =====================================================================
// 响应信封辅助
// =====================================================================

fn ok(data: Value) -> Json<Value> {
    Json(json!({ "success": true, "data": data }))
}

fn err(msg: &str) -> Json<Value> {
    Json(json!({ "success": false, "code": "IAM_REPO_ERR", "error": msg }))
}

fn q_str(q: &HashMap<String, String>, key: &str, default: &str) -> String {
    q.get(key).cloned().unwrap_or_else(|| default.to_string())
}

/// 解析租户参数：优先按 tenant_code 解析出 tenant_id；若参数本身已是有效 tenant_id 则直接用。
/// IAM seed 中 `T001` 是 tenant_code，`t001-tenant` 是 tenant_id，两者都需兼容。
fn resolve_tenant(s: &GatewayState, input: &str) -> Result<String, String> {
    if let Ok(Some(_)) = s.iam.get_tenant(input) {
        return Ok(input.to_string());
    }
    if let Some(t) = s.iam.find_tenant_by_code(input) {
        return Ok(t.tenant_id);
    }
    Ok(input.to_string())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// 把扁平 `{id, parentId, ...}` 节点挂成树（parentId 缺失/自引用视为根）。
/// 先全部挂载 children，再统一收集根节点，避免顺序依赖。
fn build_tree(items: Vec<Map<String, Value>>) -> Vec<Value> {
    let mut nodes: HashMap<String, Value> = HashMap::new();
    for mut m in items {
        // 先克隆 id，结束对 m 的不可变借用，再写入 children
        let id = m.get("id").and_then(|v| v.as_str()).map(String::from);
        if let Some(id) = id {
            m.insert("children".into(), json!([]));
            nodes.insert(id, json!(m));
        }
    }
    let all_ids: Vec<String> = nodes.keys().cloned().collect();
    for id in &all_ids {
        let parent = nodes
            .get(id)
            .and_then(|v| v.get("parentId"))
            .and_then(|v| v.as_str())
            .map(String::from);
        if let Some(p) = parent {
            if p != *id && nodes.contains_key(&p) {
                // 先克隆子节点，结束不可变借用，再挂载到父节点
                if let Some(child) = nodes.get(id).cloned() {
                    if let Some(arr) = nodes
                        .get_mut(&p)
                        .and_then(|v| v.get_mut("children"))
                        .and_then(|c| c.as_array_mut())
                    {
                        arr.push(child);
                    }
                }
            }
        }
    }
    let root_ids: Vec<String> = all_ids
        .iter()
        .filter(|id| {
            let p = nodes
                .get(*id)
                .and_then(|v| v.get("parentId"))
                .and_then(|v| v.as_str())
                .map(String::from);
            match p {
                Some(p) if p != **id && nodes.contains_key(&p) => false,
                _ => true,
            }
        })
        .cloned()
        .collect();
    root_ids
        .iter()
        .map(|id| nodes.get(id).cloned().unwrap_or(json!(null)))
        .collect()
}

// =====================================================================
// 系统管理路由
// =====================================================================

pub fn build_system_router() -> Router<GatewayState> {
    Router::new()
        // ===== 权限 =====
        .route("/api/system/permissions", get(get_permissions))
        // ===== 部门 =====
        .route("/api/system/dept", get(list_dept).post(stub_create))
        .route("/api/system/dept/tree", get(dept_tree))
        .route(
            "/api/system/dept/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        .route("/api/system/dept/:id/users", get(stub_list))
        // ===== 岗位（stub）=====
        .route("/api/system/post", get(stub_list).post(stub_create))
        .route("/api/system/post/dept/:deptId", get(stub_list))
        .route(
            "/api/system/post/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        // ===== 用户 =====
        .route("/api/system/user", get(stub_list).post(stub_create))
        .route(
            "/api/system/user/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        .route("/api/system/user/:id/resetPwd", put(stub_update))
        .route("/api/system/user/:id/changeStatus", put(stub_update))
        .route(
            "/api/system/user/:id/roles",
            get(get_user_roles).put(stub_update),
        )
        // ===== 角色 =====
        .route("/api/system/role", get(list_roles).post(stub_create))
        .route(
            "/api/system/role/:id",
            get(get_role).put(stub_update).delete(stub_delete),
        )
        .route("/api/system/role/:id/menuPerms", get(stub_list).put(stub_update))
        .route("/api/system/role/:id/dataPerms", get(stub_list).put(stub_update))
        .route("/api/system/role/:id/users", get(stub_list))
        .route("/api/system/role/:id/copy", post(stub_create))
        // ===== 菜单 =====
        .route("/api/system/menu/tree", get(menu_tree))
        .route("/api/system/menu", get(stub_list).post(stub_create))
        .route(
            "/api/system/menu/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        // ===== 字典类型（stub）=====
        .route("/api/system/dict/type", get(stub_list).post(stub_create))
        .route("/api/system/dict/type/all", get(stub_list))
        .route(
            "/api/system/dict/type/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        // ===== 字典数据（stub）=====
        .route("/api/system/dict/data", get(stub_list).post(stub_create))
        .route("/api/system/dict/data/type/:dictType", get(stub_list))
        .route(
            "/api/system/dict/data/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        // ===== 参数配置（stub）=====
        .route("/api/system/config", get(stub_list).post(stub_create))
        .route("/api/system/config/refresh-cache", delete(stub_delete))
        .route(
            "/api/system/config/:id",
            get(stub_detail).put(stub_update).delete(stub_delete),
        )
        .route("/api/system/config/key/:key", get(stub_detail))
        // ===== 操作日志（stub）=====
        .route("/api/system/operlog", get(stub_list))
        .route("/api/system/operlog/clean", delete(stub_delete))
        .route(
            "/api/system/operlog/:id",
            get(stub_detail).delete(stub_delete),
        )
        .route("/api/system/operlog/export", get(stub_list))
        // ===== 登录日志（stub）=====
        .route("/api/system/logininfor", get(stub_list))
        .route("/api/system/logininfor/clean", delete(stub_delete))
        .route("/api/system/logininfor/:id", delete(stub_delete))
        .route("/api/system/logininfor/export", get(stub_list))
}

// =====================================================================
// 安全域路由
// =====================================================================

pub fn build_security_router() -> Router<GatewayState> {
    Router::new()
        .route("/api/security/status", get(security_status))
        .route("/api/security/api-keys", get(list_api_keys).post(create_api_key))
        .route("/api/security/api-keys/:id", delete(revoke_api_key))
        .route("/api/security/validate", post(validate_api_key))
        .route("/api/security/audit-log", get(audit_log))
}

// =====================================================================
// 读接口（IAM 仓储真实现 + 形状映射）
// =====================================================================

/// GET /api/system/permissions —— 当前用户权限/角色/菜单
async fn get_permissions(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let user = q_str(&q, "user_id", DEFAULT_USER);
    let roles = s.iam.get_user_roles(&tenant, &user).unwrap_or_default();
    let perms = s.iam.get_user_permissions(&tenant, &user).unwrap_or_default();
    ok(json!({
        "user_id": user,
        "tenant_id": tenant,
        "roles": roles.iter().map(|r| json!({
            "id": r.role_id, "code": r.role_code, "name": r.role_name
        })).collect::<Vec<_>>(),
        "permissions": perms,
        "menus": [],
    }))
}

/// GET /api/system/dept —— 部门列表
async fn list_dept(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_departments(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|d| json!({
                "id": d.dept_id,
                "name": d.dept_name,
                "code": d.dept_code,
                "parentId": d.parent_id,
                "sort": d.sort_order,
                "status": d.status,
                "leaderId": d.manager_user_id,
                "createdAt": d.created_at,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dept list: {e}")),
    }
}

/// GET /api/system/dept/tree —— 部门树
async fn dept_tree(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_departments(&tenant) {
        Ok(list) => {
            let flat = list
                .iter()
                .map(|d| {
                    let mut m = Map::new();
                    m.insert("id".into(), json!(d.dept_id));
                    m.insert("name".into(), json!(d.dept_name));
                    m.insert("code".into(), json!(d.dept_code));
                    m.insert("parentId".into(), json!(d.parent_id));
                    m.insert("sort".into(), json!(d.sort_order));
                    m.insert("status".into(), json!(d.status));
                    m
                })
                .collect();
            ok(json!(build_tree(flat)))
        }
        Err(e) => err(&format!("dept tree: {e}")),
    }
}

/// GET /api/system/role —— 角色列表
async fn list_roles(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_roles(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|r| json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "type": r.role_type,
                "dataScope": r.data_scope,
                "sort": r.sort_order,
                "status": r.status,
                "remark": r.description,
                "createdAt": r.created_at,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("role list: {e}")),
    }
}

/// GET /api/system/role/:id —— 角色详情
async fn get_role(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_roles(&tenant) {
        Ok(list) => match list.into_iter().find(|r| r.role_id == id) {
            Some(r) => ok(json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "type": r.role_type,
                "dataScope": r.data_scope,
                "sort": r.sort_order,
                "status": r.status,
                "remark": r.description,
                "createdAt": r.created_at,
            })),
            None => err(&format!("role not found: {id}")),
        },
        Err(e) => err(&format!("role detail: {e}")),
    }
}

/// GET /api/system/user/:id/roles —— 用户已分配角色
async fn get_user_roles(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_user_roles(&tenant, &id) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|r| json!({
                "id": r.role_id,
                "code": r.role_code,
                "name": r.role_name,
                "status": r.status,
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("user roles: {e}")),
    }
}

/// GET /api/system/menu/tree —— 用户可见菜单树
async fn menu_tree(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> Json<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let user = q_str(&q, "user_id", DEFAULT_USER);
    match s.iam.list_user_menus(&tenant, &user) {
        Ok(list) => {
            let flat = list
                .iter()
                .map(|m| {
                    let mut node = Map::new();
                    node.insert("id".into(), json!(m.menu_id));
                    node.insert("name".into(), json!(m.menu_name));
                    node.insert("parentId".into(), json!(m.parent_id));
                    node.insert("type".into(), json!(m.menu_type));
                    node.insert("path".into(), json!(m.route_path));
                    node.insert("component".into(), json!(m.component_path));
                    node.insert("icon".into(), json!(m.icon));
                    node.insert("permission".into(), json!(m.permission_code));
                    node.insert("visible".into(), json!(m.is_visible));
                    node.insert("isCache".into(), json!(m.is_cached));
                    node.insert("sort".into(), json!(m.sort_order));
                    node.insert("status".into(), json!(m.status));
                    node
                })
                .collect();
            ok(json!(build_tree(flat)))
        }
        Err(e) => err(&format!("menu tree: {e}")),
    }
}

// =====================================================================
// 安全域 handler
// =====================================================================

/// GET /api/security/status —— 安全状态
async fn security_status(State(s): State<GatewayState>) -> Json<Value> {
    ok(json!({
        "auth_enabled": s.config.auth.enabled,
        "rate_limit_enabled": s.config.rate_limit.enabled,
        "iam": "ready",
        "db": "sqlite",
        "default_tenant": DEFAULT_TENANT,
        "ts": now_iso(),
    }))
}

/// GET /api/security/api-keys —— 凭证列表（stub，持久化待后续）
async fn list_api_keys() -> Json<Value> {
    ok(json!([]))
}

/// POST /api/security/api-keys —— 创建凭证（生成明文 key 并注册进 auth 中间件）
async fn create_api_key(
    State(s): State<GatewayState>,
    Json(body): Json<Value>,
) -> Json<Value> {
    let name = body
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("api-key")
        .to_string();
    let key = format!("mox_{}", uuid::Uuid::new_v4().simple());
    s.auth.register_api_key(DEFAULT_USER, &key);
    ok(json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "name": name,
        "api_key": key,
        "active": true,
        "createdAt": now_iso(),
    }))
}

/// DELETE /api/security/api-keys/:id —— 吊销凭证（stub）
async fn revoke_api_key() -> Json<Value> {
    ok(json!(null))
}

/// POST /api/security/validate —— 校验凭证明文
async fn validate_api_key(
    State(s): State<GatewayState>,
    Json(body): Json<Value>,
) -> Json<Value> {
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

/// GET /api/security/audit-log —— 审计日志（stub，待 IAM 审计表接入）
async fn audit_log(Query(_q): Query<HashMap<String, String>>) -> Json<Value> {
    ok(json!([]))
}

// =====================================================================
// 写接口 / 未落库域 stub（统一信封，避免 404 与 mock 兜底）
// =====================================================================

async fn stub_list() -> Json<Value> {
    ok(json!([]))
}

async fn stub_detail() -> Json<Value> {
    ok(json!(null))
}

async fn stub_create() -> Json<Value> {
    ok(json!({ "id": uuid::Uuid::new_v4().to_string() }))
}

async fn stub_update() -> Json<Value> {
    ok(json!(null))
}

async fn stub_delete() -> Json<Value> {
    ok(json!(null))
}
