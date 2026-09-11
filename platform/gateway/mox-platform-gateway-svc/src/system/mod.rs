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
pub(crate) const DEFAULT_TENANT: &str = "T001";
/// 种子超级管理员 user_id
pub(crate) const DEFAULT_USER: &str = "admin-user";


// ====================================================================
// system/mod.rs — 系统管理模块根（路由装配 + 通用工具 + JSON 映射）
// 子模块：dept/post/user/role/menu/dict/config/operlog/logininfor/security/permission
// ====================================================================

pub mod dept;
pub mod post;
pub mod user;
pub mod role;
pub mod menu;
pub mod dict;
pub mod config;
pub mod operlog;
pub mod logininfor;
pub mod security;
pub mod permission;
pub mod tenant;

pub(crate) fn ok(data: Value) -> ApiResponse<Value> {
    api_ok(data)
}


pub(crate) fn err(msg: &str) -> ApiResponse<Value> {
    api_error(500, msg)
}


pub(crate) fn q_str(q: &HashMap<String, String>, key: &str, default: &str) -> String {
    q.get(key).cloned().unwrap_or_else(|| default.to_string())
}

/// 解析租户参数：优先按 tenant_code 解析出 tenant_id；若参数本身已是有效 tenant_id 则直接用。
/// IAM seed 中 `T001` 是 tenant_code，`t001-tenant` 是 tenant_id，两者都需兼容。

pub(crate) fn resolve_tenant(s: &GatewayState, input: &str) -> Result<String, String> {
    if let Ok(Some(_)) = s.iam.get_tenant(input) {
        return Ok(input.to_string());
    }
    if let Some(t) = s.iam.find_tenant_by_code(input) {
        return Ok(t.tenant_id);
    }
    Ok(input.to_string())
}


pub(crate) fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// IAM 状态字符串 → 前端数字契约（1 启用 / 0 停用）。
/// 后端 IAM 模型用 "active"/"disabled" 字符串；前端 admin 面板按 status===1 判断启用/停用。
/// 归一化在网关映射层完成，前端无需感知 IAM 域语义。

pub(crate) fn status_flag(s: &str) -> i64 {
    match s.to_ascii_lowercase().as_str() {
        "active" | "enabled" | "normal" | "1" => 1,
        _ => 0,
    }
}

/// 把扁平 `{id, parentId, ...}` 节点挂成树（parentId 缺失/自引用视为根）。
/// 先全部挂载 children，再统一收集根节点，避免顺序依赖。

pub(crate) fn build_tree(items: Vec<Map<String, Value>>) -> Vec<Value> {
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


pub(crate) fn opt_str<'a>(body: &'a Value, key: &str) -> Option<&'a str> {
    body.get(key).and_then(|v| v.as_str())
}

pub(crate) fn opt_i64(body: &Value, key: &str) -> Option<i64> {
    body.get(key).and_then(|v| v.as_i64())
}

/// 前端 status 数字（1/0）→ IAM 状态字符串（active/disabled）

pub(crate) fn opt_status(body: &Value) -> Option<&'static str> {
    body
        .get("status")
        .and_then(|v| v.as_i64())
        .map(|n| if n == 1 { "active" } else { "disabled" })
}

// ----- 模型 → 前端驼峰映射 -----


pub(crate) fn user_json(u: &IamUser) -> Value {
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


pub(crate) fn dept_json(d: &IamDepartment) -> Value {
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


pub(crate) fn role_json(r: &IamRole) -> Value {
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


pub(crate) fn post_json(p: &SysPost) -> Value {
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


pub(crate) fn menu_json(m: &IamMenu) -> Value {
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


pub(crate) fn dict_type_json(d: &SysDictType) -> Value {
    json!({
        "id": d.dict_id,
        "dictName": d.dict_name,
        "dictType": d.dict_type,
        "status": status_flag(&d.status),
        "remark": d.remark,
        "createdAt": d.created_at,
    })
}


pub(crate) fn dict_data_json(d: &SysDictData) -> Value {
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


pub(crate) fn config_json(c: &SysConfig) -> Value {
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


pub(crate) fn oper_log_json(l: &SysOperLog) -> Value {
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


pub(crate) fn login_log_json(l: &SysLoginLog) -> Value {
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


pub(crate) fn api_key_json(k: &SysApiKey) -> Value {
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


pub fn build_system_router() -> Router<GatewayState> {
    Router::new()
        .route("/api/auth/me", get(permission::current_user_handler))
        .route("/api/tenant", get(tenant::list_tenants).post(tenant::create_tenant_handler))
        .route("/api/tenant/:id", get(tenant::get_tenant_detail).put(tenant::update_tenant_handler).delete(tenant::delete_tenant_handler))
        .route("/api/tenant/switch/:id", get(tenant::switch_tenant))
        // ===== 权限 =====
        .route("/api/system/permissions", get(permission::get_permissions))
        // ===== 部门 =====
        .route("/api/system/dept", get(dept::list_dept).post(dept::create_dept_handler))
        .route("/api/system/dept/tree", get(dept::dept_tree))
        .route(
            "/api/system/dept/:id",
            get(dept::get_dept_detail_handler)
                .put(dept::update_dept_handler)
                .delete(dept::delete_dept_handler),
        )
        .route("/api/system/dept/:id/users", get(dept::list_dept_users_handler))
        // ===== 岗位 =====
        .route(
            "/api/system/post",
            get(post::list_posts_handler).post(post::create_post_handler),
        )
        .route(
            "/api/system/post/dept/:deptId",
            get(post::list_posts_by_dept_handler),
        )
        .route(
            "/api/system/post/:id",
            get(post::get_post_detail_handler)
                .put(post::update_post_handler)
                .delete(post::delete_post_handler),
        )
        // ===== 用户 =====
        .route(
            "/api/system/user",
            get(user::list_users_handler).post(user::create_user_handler),
        )
        .route(
            "/api/system/user/:id",
            get(user::get_user_detail_handler)
                .put(user::update_user_handler)
                .delete(user::delete_user_handler),
        )
        .route("/api/system/user/:id/resetPwd", put(user::reset_user_pwd_handler))
        .route(
            "/api/system/user/:id/changeStatus",
            put(user::change_user_status_handler),
        )
        .route(
            "/api/system/user/:id/roles",
            get(permission::get_user_roles).put(user::assign_user_roles_handler),
        )
        .route(
            "/api/system/user/:id/depts",
            get(user::get_user_depts_handler).put(user::set_user_depts_handler),
        )
        // ===== 角色 =====
        .route("/api/system/role", get(role::list_roles).post(role::create_role_handler))
        .route(
            "/api/system/role/:id",
            get(role::get_role)
                .put(role::update_role_handler)
                .delete(role::delete_role_handler),
        )
        .route(
            "/api/system/role/:id/menuPerms",
            get(role::get_role_menu_perms_handler).put(role::set_role_menu_perms_handler),
        )
        .route(
            "/api/system/role/:id/dataPerms",
            get(role::get_role_data_perms_handler).put(role::set_role_data_perms_handler),
        )
        .route("/api/system/role/:id/users", get(role::list_role_users_handler))
        .route("/api/system/role/:id/copy", post(role::copy_role_handler))
        // ===== 菜单 =====
        .route("/api/system/menu/tree", get(menu::menu_tree))
        .route(
            "/api/system/menu",
            get(menu::list_menus_handler).post(menu::create_menu_handler),
        )
        .route(
            "/api/system/menu/:id",
            get(menu::get_menu_detail_handler)
                .put(menu::update_menu_handler)
                .delete(menu::delete_menu_handler),
        )
        // ===== 字典类型 =====
        .route(
            "/api/system/dict/type",
            get(dict::list_dict_types_handler).post(dict::create_dict_type_handler),
        )
        .route(
            "/api/system/dict/type/all",
            get(dict::list_all_dict_types_handler),
        )
        .route(
            "/api/system/dict/type/:id",
            get(dict::get_dict_type_detail_handler)
                .put(dict::update_dict_type_handler)
                .delete(dict::delete_dict_type_handler),
        )
        // ===== 字典数据 =====
        .route(
            "/api/system/dict/data",
            get(dict::list_dict_data_handler).post(dict::create_dict_data_handler),
        )
        .route(
            "/api/system/dict/data/type/:dictType",
            get(dict::list_dict_data_by_type_handler),
        )
        .route(
            "/api/system/dict/data/:id",
            get(dict::get_dict_data_detail_handler)
                .put(dict::update_dict_data_handler)
                .delete(dict::delete_dict_data_handler),
        )
        // ===== 参数配置 =====
        .route(
            "/api/system/config",
            get(config::list_configs_handler).post(config::create_config_handler),
        )
        .route(
            "/api/system/config/refresh-cache",
            delete(config::refresh_config_cache_handler),
        )
        .route(
            "/api/system/config/:id",
            get(config::get_config_detail_handler)
                .put(config::update_config_handler)
                .delete(config::delete_config_handler),
        )
        .route(
            "/api/system/config/key/:key",
            get(config::get_config_by_key_handler),
        )
        // ===== 操作日志 =====
        .route("/api/system/operlog", get(operlog::list_oper_logs_handler))
        .route(
            "/api/system/operlog/clean",
            delete(operlog::clean_oper_logs_handler),
        )
        .route(
            "/api/system/operlog/:id",
            get(operlog::get_oper_log_detail_handler).delete(operlog::delete_oper_log_handler),
        )
        .route("/api/system/operlog/export", get(operlog::export_oper_logs_handler))
        // ===== 登录日志 =====
        .route("/api/system/logininfor", get(logininfor::list_login_logs_handler))
        .route(
            "/api/system/logininfor/clean",
            delete(logininfor::clean_login_logs_handler),
        )
        .route(
            "/api/system/logininfor/:id",
            delete(logininfor::delete_login_log_handler),
        )
        .route(
            "/api/system/logininfor/export",
            get(logininfor::export_login_logs_handler),
        )
}


pub fn build_security_router() -> Router<GatewayState> {
    Router::new()
        .route("/api/security/status", get(security::security_status))
        .route("/api/security/api-keys", get(security::list_api_keys).post(security::create_api_key))
        .route("/api/security/api-keys/:id", delete(security::revoke_api_key))
        .route("/api/security/validate", post(security::validate_api_key))
        .route("/api/security/audit-log", get(security::audit_log))
}

