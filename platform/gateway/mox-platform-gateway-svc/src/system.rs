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
use mox_api_protocol::{ApiResponse, api_ok, api_error};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use mox_platform_iam_core::{
    IamDepartment, IamMenu, IamRole, IamUser, SysApiKey, SysConfig, SysDictData, SysDictType,
    SysLoginLog, SysOperLog, SysPost,
};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// 种子演示租户（seed_builtins 中名为"企业演示租户"，含部门 D001 与管理员）
const DEFAULT_TENANT: &str = "T001";
/// 种子超级管理员 user_id
const DEFAULT_USER: &str = "admin-user";

// =====================================================================
// 响应信封辅助
// =====================================================================

async fn current_user_handler(crate::auth::ApiAuth(user): crate::auth::ApiAuth) -> ApiResponse<Value> {
    api_ok(json!(user))
}

fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}

fn err(msg: &str) -> ApiResponse<Value> {
    api_error(500, msg)
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

/// IAM 状态字符串 → 前端数字契约（1 启用 / 0 停用）。
/// 后端 IAM 模型用 "active"/"disabled" 字符串；前端 admin 面板按 status===1 判断启用/停用。
/// 归一化在网关映射层完成，前端无需感知 IAM 域语义。
fn status_flag(s: &str) -> i64 {
    match s.to_ascii_lowercase().as_str() {
        "active" | "enabled" | "normal" | "1" => 1,
        _ => 0,
    }
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
        .route("/api/auth/me", get(current_user_handler))
        // ===== 权限 =====
        .route("/api/system/permissions", get(get_permissions))
        // ===== 部门 =====
        .route("/api/system/dept", get(list_dept).post(create_dept_handler))
        .route("/api/system/dept/tree", get(dept_tree))
        .route(
            "/api/system/dept/:id",
            get(get_dept_detail_handler)
                .put(update_dept_handler)
                .delete(delete_dept_handler),
        )
        .route("/api/system/dept/:id/users", get(list_dept_users_handler))
        // ===== 岗位 =====
        .route(
            "/api/system/post",
            get(list_posts_handler).post(create_post_handler),
        )
        .route(
            "/api/system/post/dept/:deptId",
            get(list_posts_by_dept_handler),
        )
        .route(
            "/api/system/post/:id",
            get(get_post_detail_handler)
                .put(update_post_handler)
                .delete(delete_post_handler),
        )
        // ===== 用户 =====
        .route(
            "/api/system/user",
            get(list_users_handler).post(create_user_handler),
        )
        .route(
            "/api/system/user/:id",
            get(get_user_detail_handler)
                .put(update_user_handler)
                .delete(delete_user_handler),
        )
        .route("/api/system/user/:id/resetPwd", put(reset_user_pwd_handler))
        .route(
            "/api/system/user/:id/changeStatus",
            put(change_user_status_handler),
        )
        .route(
            "/api/system/user/:id/roles",
            get(get_user_roles).put(assign_user_roles_handler),
        )
        // ===== 角色 =====
        .route("/api/system/role", get(list_roles).post(create_role_handler))
        .route(
            "/api/system/role/:id",
            get(get_role)
                .put(update_role_handler)
                .delete(delete_role_handler),
        )
        .route(
            "/api/system/role/:id/menuPerms",
            get(get_role_menu_perms_handler).put(set_role_menu_perms_handler),
        )
        .route(
            "/api/system/role/:id/dataPerms",
            get(get_role_data_perms_handler).put(set_role_data_perms_handler),
        )
        .route("/api/system/role/:id/users", get(list_role_users_handler))
        .route("/api/system/role/:id/copy", post(copy_role_handler))
        // ===== 菜单 =====
        .route("/api/system/menu/tree", get(menu_tree))
        .route(
            "/api/system/menu",
            get(list_menus_handler).post(create_menu_handler),
        )
        .route(
            "/api/system/menu/:id",
            get(get_menu_detail_handler)
                .put(update_menu_handler)
                .delete(delete_menu_handler),
        )
        // ===== 字典类型 =====
        .route(
            "/api/system/dict/type",
            get(list_dict_types_handler).post(create_dict_type_handler),
        )
        .route(
            "/api/system/dict/type/all",
            get(list_all_dict_types_handler),
        )
        .route(
            "/api/system/dict/type/:id",
            get(get_dict_type_detail_handler)
                .put(update_dict_type_handler)
                .delete(delete_dict_type_handler),
        )
        // ===== 字典数据 =====
        .route(
            "/api/system/dict/data",
            get(list_dict_data_handler).post(create_dict_data_handler),
        )
        .route(
            "/api/system/dict/data/type/:dictType",
            get(list_dict_data_by_type_handler),
        )
        .route(
            "/api/system/dict/data/:id",
            get(get_dict_data_detail_handler)
                .put(update_dict_data_handler)
                .delete(delete_dict_data_handler),
        )
        // ===== 参数配置 =====
        .route(
            "/api/system/config",
            get(list_configs_handler).post(create_config_handler),
        )
        .route(
            "/api/system/config/refresh-cache",
            delete(refresh_config_cache_handler),
        )
        .route(
            "/api/system/config/:id",
            get(get_config_detail_handler)
                .put(update_config_handler)
                .delete(delete_config_handler),
        )
        .route(
            "/api/system/config/key/:key",
            get(get_config_by_key_handler),
        )
        // ===== 操作日志 =====
        .route("/api/system/operlog", get(list_oper_logs_handler))
        .route(
            "/api/system/operlog/clean",
            delete(clean_oper_logs_handler),
        )
        .route(
            "/api/system/operlog/:id",
            get(get_oper_log_detail_handler).delete(delete_oper_log_handler),
        )
        .route("/api/system/operlog/export", get(export_oper_logs_handler))
        // ===== 登录日志 =====
        .route("/api/system/logininfor", get(list_login_logs_handler))
        .route(
            "/api/system/logininfor/clean",
            delete(clean_login_logs_handler),
        )
        .route(
            "/api/system/logininfor/:id",
            delete(delete_login_log_handler),
        )
        .route(
            "/api/system/logininfor/export",
            get(export_login_logs_handler),
        )
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
) -> ApiResponse<Value> {
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
) -> ApiResponse<Value> {
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
                "status": status_flag(&d.status),
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
) -> ApiResponse<Value> {
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
                    m.insert("status".into(), json!(status_flag(&d.status)));
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
) -> ApiResponse<Value> {
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
                "status": status_flag(&r.status),
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
) -> ApiResponse<Value> {
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
                "status": status_flag(&r.status),
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
) -> ApiResponse<Value> {
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
                "status": status_flag(&r.status),
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("user roles: {e}")),
    }
}

/// GET /api/system/menu/tree —— 用户可见菜单树
async fn menu_tree(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
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
                    node.insert("status".into(), json!(status_flag(&m.status)));
                    node
                })
                .collect();
            ok(json!(build_tree(flat)))
        }
        Err(e) => err(&format!("menu tree: {e}")),
    }
}

// =====================================================================
// 写接口 / CRUD handler（IAM 仓储真实现 + 形状映射）
// =====================================================================

// ----- body 提取辅助 -----

fn opt_str<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(|v| v.as_str())
}

fn opt_i64(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(|v| v.as_i64())
}

/// 前端 status 数字（1/0）→ IAM 状态字符串（active/disabled）
fn opt_status(body: &Value) -> Option<&'static str> {
    body
        .get("status")
        .and_then(|v| v.as_i64())
        .map(|n| if n == 1 { "active" } else { "disabled" })
}

// ----- 模型 → 前端驼峰映射 -----

fn user_json(u: &IamUser) -> Value {
    json!({
        "id": u.user_id,
        "userCode": u.user_code,
        "username": u.username,
        "realName": u.real_name,
        "nickname": u.nickname,
        "email": u.email,
        "phone": u.phone,
        "avatar": u.avatar,
        "deptId": u.dept_id,
        "position": u.position,
        "status": status_flag(&u.user_status),
        "isSuperuser": u.is_superuser,
        "createdAt": u.created_at,
    })
}

fn dept_json(d: &IamDepartment) -> Value {
    json!({
        "id": d.dept_id,
        "name": d.dept_name,
        "code": d.dept_code,
        "parentId": d.parent_id,
        "sort": d.sort_order,
        "status": status_flag(&d.status),
        "leaderId": d.manager_user_id,
        "createdAt": d.created_at,
    })
}

fn role_json(r: &IamRole) -> Value {
    json!({
        "id": r.role_id,
        "code": r.role_code,
        "name": r.role_name,
        "type": r.role_type,
        "dataScope": r.data_scope,
        "sort": r.sort_order,
        "status": status_flag(&r.status),
        "remark": r.description,
        "createdAt": r.created_at,
    })
}

fn post_json(p: &SysPost) -> Value {
    json!({
        "id": p.post_id,
        "code": p.post_code,
        "name": p.post_name,
        "deptId": p.dept_id,
        "sort": p.sort_order,
        "status": status_flag(&p.status),
        "remark": p.remark,
        "createdAt": p.created_at,
    })
}

fn menu_json(m: &IamMenu) -> Value {
    json!({
        "id": m.menu_id,
        "code": m.menu_code,
        "name": m.menu_name,
        "parentId": m.parent_id,
        "type": m.menu_type,
        "path": m.route_path,
        "component": m.component_path,
        "icon": m.icon,
        "permission": m.permission_code,
        "visible": m.is_visible,
        "isCache": m.is_cached,
        "sort": m.sort_order,
        "status": status_flag(&m.status),
    })
}

fn dict_type_json(d: &SysDictType) -> Value {
    json!({
        "id": d.dict_id,
        "dictName": d.dict_name,
        "dictType": d.dict_type,
        "status": status_flag(&d.status),
        "remark": d.remark,
        "createdAt": d.created_at,
    })
}

fn dict_data_json(d: &SysDictData) -> Value {
    json!({
        "id": d.dict_code,
        "dictSort": d.dict_sort,
        "dictLabel": d.dict_label,
        "dictValue": d.dict_value,
        "dictType": d.dict_type,
        "cssClass": d.css_class,
        "listClass": d.list_class,
        "isDefault": d.is_default,
        "status": status_flag(&d.status),
        "remark": d.remark,
        "createdAt": d.created_at,
    })
}

fn config_json(c: &SysConfig) -> Value {
    json!({
        "id": c.config_id,
        "configName": c.config_name,
        "configKey": c.config_key,
        "configValue": c.config_value,
        "configType": c.config_type,
        "status": status_flag(&c.status),
        "remark": c.remark,
        "createdAt": c.created_at,
    })
}

fn oper_log_json(l: &SysOperLog) -> Value {
    json!({
        "id": l.oper_id,
        "title": l.title,
        "businessType": l.business_type,
        "method": l.method,
        "requestMethod": l.request_method,
        "operName": l.oper_name,
        "deptName": l.dept_name,
        "operUrl": l.oper_url,
        "operIp": l.oper_ip,
        "operLocation": l.oper_location,
        "operParam": l.oper_param,
        "jsonResult": l.json_result,
        "status": l.status,
        "errorMsg": l.error_msg,
        "operTime": l.oper_time,
        "costTime": l.cost_time,
    })
}

fn login_log_json(l: &SysLoginLog) -> Value {
    json!({
        "id": l.info_id,
        "userName": l.user_name,
        "ipaddr": l.ipaddr,
        "loginLocation": l.login_location,
        "browser": l.browser,
        "os": l.os,
        "status": l.status,
        "msg": l.msg,
        "loginTime": l.login_time,
    })
}

fn api_key_json(k: &SysApiKey) -> Value {
    let masked = format!("{}***", &k.api_key[..8.min(k.api_key.len())]);
    json!({
        "id": k.key_id,
        "name": k.name,
        "apiKey": masked,
        "userId": k.user_id,
        "scopes": k.scopes,
        "status": k.status,
        "createdAt": k.created_at,
        "revokedAt": k.revoked_at,
    })
}

// ----- 部门 Dept -----

async fn create_dept_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "code").unwrap_or("");
    let name = opt_str(&body, "name").unwrap_or("");
    let parent_id = opt_str(&body, "parentId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let manager = opt_str(&body, "leaderId");
    match s.iam.create_dept(&tenant, code, name, parent_id, sort, status, manager) {
        Ok(d) => ok(dept_json(&d)),
        Err(e) => err(&format!("dept create: {e}")),
    }
}

async fn get_dept_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dept(&id) {
        Ok(Some(d)) => ok(dept_json(&d)),
        Ok(None) => err("dept not found"),
        Err(e) => err(&format!("dept detail: {e}")),
    }
}

async fn update_dept_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let parent_id = opt_str(&body, "parentId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let manager = opt_str(&body, "leaderId");
    match s.iam.update_dept(&id, name, parent_id, sort, status, manager) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dept update: {e}")),
    }
}

async fn delete_dept_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dept(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dept delete: {e}")),
    }
}

async fn list_dept_users_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users_by_dept(&tenant, &id) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("dept users: {e}")),
    }
}

// ----- 岗位 Post -----

async fn list_posts_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_posts(&tenant) {
        Ok(list) => ok(json!(list.iter().map(post_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("post list: {e}")),
    }
}

async fn create_post_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "postCode").unwrap_or("");
    let name = opt_str(&body, "postName").unwrap_or("");
    let dept_id = opt_str(&body, "deptId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_post(&tenant, code, name, dept_id, sort, status, remark) {
        Ok(p) => ok(post_json(&p)),
        Err(e) => err(&format!("post create: {e}")),
    }
}

async fn list_posts_by_dept_handler(
    State(s): State<GatewayState>,
    Path(dept_id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_posts_by_dept(&tenant, &dept_id) {
        Ok(list) => ok(json!(list.iter().map(post_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("post list by dept: {e}")),
    }
}

async fn get_post_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_post(&id) {
        Ok(Some(p)) => ok(post_json(&p)),
        Ok(None) => err("post not found"),
        Err(e) => err(&format!("post detail: {e}")),
    }
}

async fn update_post_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let code = opt_str(&body, "postCode");
    let name = opt_str(&body, "postName");
    let dept_id = opt_str(&body, "deptId");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_post(&id, code, name, dept_id, sort, status, remark) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("post update: {e}")),
    }
}

async fn delete_post_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_post(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("post delete: {e}")),
    }
}

// ----- 用户 User -----

async fn list_users_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users(&tenant) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("user list: {e}")),
    }
}

async fn create_user_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let username = opt_str(&body, "username").unwrap_or("");
    let user_code = opt_str(&body, "userCode")
        .map(String::from)
        .unwrap_or_else(|| format!("U{}", chrono::Utc::now().timestamp()));
    let real_name = opt_str(&body, "realName");
    let password_hash = opt_str(&body, "password");
    let dept_id = opt_str(&body, "deptId");
    match s.iam.create_user(
        &tenant,
        &user_code,
        username,
        real_name,
        password_hash,
        dept_id,
        false,
    ) {
        Ok(u) => ok(user_json(&u)),
        Err(e) => err(&format!("user create: {e}")),
    }
}

async fn get_user_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_user(&id) {
        Ok(Some(u)) => ok(user_json(&u)),
        Ok(None) => err("user not found"),
        Err(e) => err(&format!("user detail: {e}")),
    }
}

async fn update_user_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let username = opt_str(&body, "username");
    let real_name = opt_str(&body, "realName");
    let email = opt_str(&body, "email");
    let phone = opt_str(&body, "phone");
    let dept_id = opt_str(&body, "deptId");
    let position = opt_str(&body, "position");
    let user_status = opt_status(&body);
    match s.iam.update_user(
        &id,
        username,
        real_name,
        email,
        phone,
        dept_id,
        position,
        user_status,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("user update: {e}")),
    }
}

async fn delete_user_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_user(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("user delete: {e}")),
    }
}

async fn reset_user_pwd_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let password = opt_str(&body, "password").unwrap_or("");
    match s.iam.reset_password(&id, password) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("reset password: {e}")),
    }
}

async fn change_user_status_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let status = opt_status(&body).unwrap_or("active");
    match s.iam.change_user_status(&id, status) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("change status: {e}")),
    }
}

async fn assign_user_roles_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let role_ids: Vec<String> = body
        .get("roleIds")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_user_roles(&tenant, &id, &role_ids) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("assign roles: {e}")),
    }
}

// ----- 角色 Role -----

async fn create_role_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "code").unwrap_or("");
    let name = opt_str(&body, "name").unwrap_or("");
    let role_type = opt_str(&body, "type");
    let data_scope = opt_str(&body, "dataScope");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body).unwrap_or("active");
    let description = opt_str(&body, "remark");
    match s.iam.create_role(
        &tenant,
        code,
        name,
        role_type,
        data_scope,
        sort,
        status,
        description,
    ) {
        Ok(r) => ok(role_json(&r)),
        Err(e) => err(&format!("role create: {e}")),
    }
}

async fn update_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let code = opt_str(&body, "code");
    let data_scope = opt_str(&body, "dataScope");
    let sort = opt_i64(&body, "sort");
    let status = opt_status(&body);
    let description = opt_str(&body, "remark");
    match s.iam.update_role(&id, name, code, data_scope, sort, status, description) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("role update: {e}")),
    }
}

async fn delete_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_role(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("role delete: {e}")),
    }
}

async fn get_role_menu_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_role_menu_ids(&tenant, &id) {
        Ok(menu_ids) => ok(json!(menu_ids)),
        Err(e) => err(&format!("role menu perms: {e}")),
    }
}

async fn set_role_menu_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let menu_ids: Vec<String> = body
        .get("menuIds")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_role_menus(&tenant, &id, &menu_ids) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("set role menus: {e}")),
    }
}

async fn get_role_data_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_role_data_perms(&tenant, &id) {
        Ok(list) => ok(json!(list
            .iter()
            .map(|dp| json!({
                "id": dp.dp_id,
                "code": dp.dp_code,
                "name": dp.dp_name,
                "scopeType": dp.scope_type,
                "resourceCode": dp.resource_code,
                "status": status_flag(&dp.status),
            }))
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("role data perms: {e}")),
    }
}

async fn set_role_data_perms_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dp_codes: Vec<String> = body
        .get("dpCodes")
        .and_then(|v| v.as_array())
        .or_else(|| body.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match s.iam.set_role_data_perms(&tenant, &id, &dp_codes) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("set role data perms: {e}")),
    }
}

async fn list_role_users_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_users_by_role(&tenant, &id) {
        Ok(list) => ok(json!(list.iter().map(user_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("role users: {e}")),
    }
}

async fn copy_role_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let new_code = opt_str(&body, "code").unwrap_or("");
    let new_name = opt_str(&body, "name").unwrap_or("");
    match s.iam.copy_role(&tenant, &id, new_code, new_name) {
        Ok(r) => ok(role_json(&r)),
        Err(e) => err(&format!("copy role: {e}")),
    }
}

// ----- 菜单 Menu -----

async fn list_menus_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_all_menus(&tenant) {
        Ok(list) => ok(json!(list.iter().map(menu_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("menu list: {e}")),
    }
}

async fn create_menu_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let code = opt_str(&body, "code").unwrap_or("");
    let name = opt_str(&body, "name").unwrap_or("");
    let menu_type = opt_str(&body, "type");
    let parent_id = opt_str(&body, "parentId");
    let route_path = opt_str(&body, "path");
    let component_path = opt_str(&body, "component");
    let icon = opt_str(&body, "icon");
    let permission_code = opt_str(&body, "permission");
    let sort = opt_i64(&body, "sort");
    let is_visible = opt_i64(&body, "visible");
    let status = opt_status(&body).unwrap_or("active");
    match s.iam.create_menu(
        &tenant,
        code,
        name,
        menu_type,
        parent_id,
        route_path,
        component_path,
        icon,
        permission_code,
        sort,
        is_visible,
        status,
    ) {
        Ok(m) => ok(menu_json(&m)),
        Err(e) => err(&format!("menu create: {e}")),
    }
}

async fn get_menu_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_menu(&id) {
        Ok(Some(m)) => ok(menu_json(&m)),
        Ok(None) => err("menu not found"),
        Err(e) => err(&format!("menu detail: {e}")),
    }
}

async fn update_menu_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let name = opt_str(&body, "name");
    let parent_id = opt_str(&body, "parentId");
    let route_path = opt_str(&body, "path");
    let component_path = opt_str(&body, "component");
    let icon = opt_str(&body, "icon");
    let permission_code = opt_str(&body, "permission");
    let sort = opt_i64(&body, "sort");
    let is_visible = opt_i64(&body, "visible");
    let status = opt_status(&body);
    match s.iam.update_menu(
        &id,
        name,
        parent_id,
        route_path,
        component_path,
        icon,
        permission_code,
        sort,
        is_visible,
        status,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("menu update: {e}")),
    }
}

async fn delete_menu_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_menu(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("menu delete: {e}")),
    }
}

// ----- 字典类型 DictType -----

async fn list_dict_types_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_types(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_type_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict type list: {e}")),
    }
}

async fn create_dict_type_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dict_name = opt_str(&body, "dictName").unwrap_or("");
    let dict_type = opt_str(&body, "dictType").unwrap_or("");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_dict_type(&tenant, dict_name, dict_type, status, remark) {
        Ok(d) => ok(dict_type_json(&d)),
        Err(e) => err(&format!("dict type create: {e}")),
    }
}

async fn list_all_dict_types_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_types(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_type_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict type all: {e}")),
    }
}

async fn get_dict_type_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dict_type(&id) {
        Ok(Some(d)) => ok(dict_type_json(&d)),
        Ok(None) => err("dict type not found"),
        Err(e) => err(&format!("dict type detail: {e}")),
    }
}

async fn update_dict_type_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let dict_name = opt_str(&body, "dictName");
    let dict_type = opt_str(&body, "dictType");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_dict_type(&id, dict_name, dict_type, status, remark) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict type update: {e}")),
    }
}

async fn delete_dict_type_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dict_type(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict type delete: {e}")),
    }
}

// ----- 字典数据 DictData -----

async fn list_dict_data_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_data(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_data_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict data list: {e}")),
    }
}

async fn create_dict_data_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let dict_label = opt_str(&body, "dictLabel").unwrap_or("");
    let dict_value = opt_str(&body, "dictValue").unwrap_or("");
    let dict_type = opt_str(&body, "dictType").unwrap_or("");
    let dict_sort = opt_i64(&body, "dictSort");
    let css_class = opt_str(&body, "cssClass");
    let list_class = opt_str(&body, "listClass");
    let is_default = opt_str(&body, "isDefault");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_dict_data(
        &tenant,
        dict_sort,
        dict_label,
        dict_value,
        dict_type,
        css_class,
        list_class,
        is_default,
        status,
        remark,
    ) {
        Ok(d) => ok(dict_data_json(&d)),
        Err(e) => err(&format!("dict data create: {e}")),
    }
}

async fn list_dict_data_by_type_handler(
    State(s): State<GatewayState>,
    Path(dict_type): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_dict_data_by_type(&tenant, &dict_type) {
        Ok(list) => ok(json!(list
            .iter()
            .map(dict_data_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("dict data by type: {e}")),
    }
}

async fn get_dict_data_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_dict_data(&id) {
        Ok(Some(d)) => ok(dict_data_json(&d)),
        Ok(None) => err("dict data not found"),
        Err(e) => err(&format!("dict data detail: {e}")),
    }
}

async fn update_dict_data_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let dict_sort = opt_i64(&body, "dictSort");
    let dict_label = opt_str(&body, "dictLabel");
    let dict_value = opt_str(&body, "dictValue");
    let dict_type = opt_str(&body, "dictType");
    let css_class = opt_str(&body, "cssClass");
    let list_class = opt_str(&body, "listClass");
    let is_default = opt_str(&body, "isDefault");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_dict_data(
        &id,
        dict_sort,
        dict_label,
        dict_value,
        dict_type,
        css_class,
        list_class,
        is_default,
        status,
        remark,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict data update: {e}")),
    }
}

async fn delete_dict_data_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_dict_data(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("dict data delete: {e}")),
    }
}

// ----- 参数配置 Config -----

async fn list_configs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_configs(&tenant) {
        Ok(list) => ok(json!(list.iter().map(config_json).collect::<Vec<_>>())),
        Err(e) => err(&format!("config list: {e}")),
    }
}

async fn create_config_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    let config_name = opt_str(&body, "configName").unwrap_or("");
    let config_key = opt_str(&body, "configKey").unwrap_or("");
    let config_value = opt_str(&body, "configValue");
    let config_type = opt_str(&body, "configType");
    let status = opt_status(&body).unwrap_or("active");
    let remark = opt_str(&body, "remark");
    match s.iam.create_config(
        &tenant,
        config_name,
        config_key,
        config_value,
        config_type,
        status,
        remark,
    ) {
        Ok(c) => ok(config_json(&c)),
        Err(e) => err(&format!("config create: {e}")),
    }
}

async fn get_config_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_config(&id) {
        Ok(Some(c)) => ok(config_json(&c)),
        Ok(None) => err("config not found"),
        Err(e) => err(&format!("config detail: {e}")),
    }
}

async fn get_config_by_key_handler(
    State(s): State<GatewayState>,
    Path(key): Path<String>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.get_config_by_key(&tenant, &key) {
        Ok(Some(c)) => ok(config_json(&c)),
        Ok(None) => err("config not found"),
        Err(e) => err(&format!("config by key: {e}")),
    }
}

async fn update_config_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
    Json(body): Json<Value>,
) -> ApiResponse<Value> {
    let config_name = opt_str(&body, "configName");
    let config_key = opt_str(&body, "configKey");
    let config_value = opt_str(&body, "configValue");
    let config_type = opt_str(&body, "configType");
    let status = opt_status(&body);
    let remark = opt_str(&body, "remark");
    match s.iam.update_config(
        &id,
        config_name,
        config_key,
        config_value,
        config_type,
        status,
        remark,
    ) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("config update: {e}")),
    }
}

async fn delete_config_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_config(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("config delete: {e}")),
    }
}

async fn refresh_config_cache_handler() -> ApiResponse<Value> {
    ok(json!({
        "refreshed": true,
        "note": "SQLite direct-read, no cache layer",
    }))
}

// ----- 操作日志 OperLog -----

async fn list_oper_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_oper_logs(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(oper_log_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("oper log list: {e}")),
    }
}

async fn get_oper_log_detail_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.get_oper_log(&id) {
        Ok(Some(l)) => ok(oper_log_json(&l)),
        Ok(None) => err("oper log not found"),
        Err(e) => err(&format!("oper log detail: {e}")),
    }
}

async fn delete_oper_log_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_oper_log(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("oper log delete: {e}")),
    }
}

async fn clean_oper_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.clean_oper_logs(&tenant) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("oper log clean: {e}")),
    }
}

async fn export_oper_logs_handler() -> ApiResponse<Value> {
    ok(json!({
        "exported": false,
        "note": "CSV export not yet implemented; use list endpoint",
    }))
}

// ----- 登录日志 LoginLog -----

async fn list_login_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.list_login_logs(&tenant) {
        Ok(list) => ok(json!(list
            .iter()
            .map(login_log_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("login log list: {e}")),
    }
}

async fn delete_login_log_handler(
    State(s): State<GatewayState>,
    Path(id): Path<String>,
) -> ApiResponse<Value> {
    match s.iam.delete_login_log(&id) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("login log delete: {e}")),
    }
}

async fn clean_login_logs_handler(
    State(s): State<GatewayState>,
    Query(q): Query<HashMap<String, String>>,
) -> ApiResponse<Value> {
    let tenant = match resolve_tenant(&s, &q_str(&q, "tenant_id", DEFAULT_TENANT)) {
        Ok(t) => t,
        Err(e) => return err(&format!("tenant resolve: {e}")),
    };
    match s.iam.clean_login_logs(&tenant) {
        Ok(_) => ok(json!(null)),
        Err(e) => err(&format!("login log clean: {e}")),
    }
}

async fn export_login_logs_handler() -> ApiResponse<Value> {
    ok(json!({
        "exported": false,
        "note": "CSV export not yet implemented; use list endpoint",
    }))
}

// =====================================================================
// 安全域 handler
// =====================================================================

/// GET /api/security/status —— 安全状态
async fn security_status(State(s): State<GatewayState>) -> ApiResponse<Value> {
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
async fn list_api_keys(State(s): State<GatewayState>) -> ApiResponse<Value> {
    match s.iam.list_api_keys(DEFAULT_TENANT) {
        Ok(list) => ok(json!(list
            .iter()
            .map(api_key_json)
            .collect::<Vec<_>>())),
        Err(e) => err(&format!("api key list: {e}")),
    }
}

/// POST /api/security/api-keys —— 创建凭证（生成明文 key，注册 auth 中间件 + 持久化 SQLite）
async fn create_api_key(
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
async fn revoke_api_key(
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
async fn validate_api_key(
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
async fn audit_log(
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
