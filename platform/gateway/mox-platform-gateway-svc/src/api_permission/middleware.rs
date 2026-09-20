//! API自动注册与权限控制中间件
//!
//! 核心能力：
//! 1. API自动注册 - 启动时自动扫描所有路由并注册到权限中心
//! 2. 权限控制中间件 - 统一的请求权限验证
//! 3. 权限点初始化 - 为所有模块预定义标准权限点

use crate::api_permission::{ApiEndpoint, ApiPermissionState, Permission};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ==================== 权限控制中间件 ====================

/// 权限控制中间件
///
/// 从请求头提取用户身份，验证是否有权限访问API端点。
/// 公开接口和不需要权限验证的接口直接放行。
pub async fn permission_middleware(
    State(state): State<Arc<ApiPermissionState>>,
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let endpoint_id = format!("{}:{}", method, path);

    // 跳过健康检查和公开接口
    if path == "/health" || path == "/api/enterprise/health" || path.starts_with("/api/enterprise/batch/export/") {
        return next.run(req).await;
    }

    // 从请求头提取用户ID
    let user_id = extract_user_id(req.headers());

    // 如果没有用户ID，检查是否为公开接口
    let user_id = match user_id {
        Some(uid) => uid,
        None => {
            // 无用户身份时，检查端点是否为公开
            let endpoints = state.endpoints.read().await;
            if let Some(ep) = endpoints.get(&endpoint_id) {
                if ep.is_public || !ep.require_auth {
                    drop(endpoints);
                    return next.run(req).await;
                }
            }
            drop(endpoints);
            // 非公开接口需要认证
            return unauthorized_response("未提供认证信息");
        }
    };

    // 检查权限
    let result = state.check_permission(&user_id, &method, &path).await;

    if !result.allowed {
        return forbidden_response(&format!(
            "权限不足：用户 {} 无权访问 {}（需要权限：{}）",
            user_id,
            endpoint_id,
            result.required_permission.unwrap_or_else(|| "未知".to_string())
        ));
    }

    // 权限验证通过，继续处理请求
    next.run(req).await
}

/// 从请求头提取用户ID
fn extract_user_id(headers: &HeaderMap) -> Option<String> {
    // 优先从 X-User-ID 头提取
    if let Some(uid) = headers.get("x-user-id") {
        return uid.to_str().ok().map(|s| s.to_string());
    }

    // 从 Authorization Bearer token 中提取（简化版，实际应解析JWT）
    if let Some(auth) = headers.get(header::AUTHORIZATION) {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                // 简化：如果token是dev-secret-token，返回admin用户
                if token == "dev-secret-token" {
                    return Some("admin".to_string());
                }
                // 其他情况尝试将token作为user_id
                return Some(token.to_string());
            }
        }
    }

    None
}

/// 未授权响应
fn unauthorized_response(message: &str) -> Response {
    let body = serde_json::json!({
        "code": 401,
        "message": message,
        "data": null
    });
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap_or_default()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

/// 禁止访问响应
fn forbidden_response(message: &str) -> Response {
    let body = serde_json::json!({
        "code": 403,
        "message": message,
        "data": null
    });
    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap_or_default()))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

// ==================== API自动注册 ====================

/// 自动注册API端点
///
/// 根据预定义的路由列表自动注册所有API端点到权限中心。
/// 每个端点自动关联对应的权限点。
pub async fn auto_register_endpoints(state: &Arc<ApiPermissionState>) {
    let endpoints = build_endpoint_registry();
    let count = endpoints.len();
    state.register_endpoints(endpoints).await;
    println!("[API权限中心] 自动注册了 {} 个API端点", count);
}

/// 初始化标准权限点
///
/// 为所有企业级模块预定义标准权限点。
pub async fn init_standard_permissions(state: &Arc<ApiPermissionState>) {
    let permissions = build_standard_permissions();
    let count = permissions.len();
    let mut perm_map = state.permissions.write().await;
    for p in permissions {
        perm_map.insert(p.permission_code.clone(), p);
    }
    drop(perm_map);
    println!("[API权限中心] 初始化了 {} 个标准权限点", count);
}

/// 构建API端点注册表
fn build_endpoint_registry() -> Vec<ApiEndpoint> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut endpoints = Vec::new();

    // 企业级模块路由定义
    let modules = vec![
        ("integration", "/api/enterprise/integration", vec![
            ("GET", "/connections", "integration:read", "read", false),
            ("POST", "/connections", "integration:create", "write", false),
            ("GET", "/connections/:id", "integration:read", "read", false),
            ("PUT", "/connections/:id", "integration:update", "write", false),
            ("DELETE", "/connections/:id", "integration:delete", "delete", false),
            ("POST", "/connections/:id/test", "integration:test", "write", false),
            ("GET", "/adapters", "integration:read", "read", false),
            ("POST", "/sync", "integration:sync", "write", false),
            ("GET", "/sync-logs", "integration:read", "read", false),
            ("GET", "/stats", "integration:read", "read", false),
        ]),
        ("sso", "/api/enterprise/sso", vec![
            ("GET", "/providers", "sso:read", "read", false),
            ("POST", "/providers", "sso:create", "write", false),
            ("GET", "/providers/:id", "sso:read", "read", false),
            ("PUT", "/providers/:id", "sso:update", "write", false),
            ("DELETE", "/providers/:id", "sso:delete", "delete", false),
            ("POST", "/oauth2/authorize", "sso:login", "read", true),
            ("POST", "/oauth2/token", "sso:login", "read", true),
            ("POST", "/saml/acs", "sso:login", "read", true),
            ("POST", "/logout", "sso:logout", "read", false),
        ]),
        ("message_center", "/api/enterprise/message", vec![
            ("GET", "/messages", "message:read", "read", false),
            ("POST", "/messages", "message:create", "write", false),
            ("GET", "/messages/:id", "message:read", "read", false),
            ("PUT", "/messages/:id/read", "message:update", "write", false),
            ("DELETE", "/messages/:id", "message:delete", "delete", false),
            ("GET", "/templates", "message:read", "read", false),
            ("POST", "/templates", "message:create", "write", false),
            ("GET", "/stats", "message:read", "read", false),
        ]),
        ("document", "/api/enterprise/document", vec![
            ("GET", "/documents", "document:read", "read", false),
            ("POST", "/documents", "document:create", "write", false),
            ("GET", "/documents/:id", "document:read", "read", false),
            ("PUT", "/documents/:id", "document:update", "write", false),
            ("DELETE", "/documents/:id", "document:delete", "delete", false),
            ("POST", "/documents/:id/sign", "document:sign", "write", false),
            ("GET", "/documents/:id/download", "document:download", "read", false),
            ("GET", "/categories", "document:read", "read", false),
            ("GET", "/stats", "document:read", "read", false),
            ("GET", "/seals", "document:read", "read", false),
            ("POST", "/seals", "document:create", "write", false),
        ]),
        ("admin", "/api/enterprise/admin", vec![
            ("GET", "/tenants", "admin:tenant:read", "read", false),
            ("POST", "/tenants", "admin:tenant:create", "write", false),
            ("GET", "/tenants/:id", "admin:tenant:read", "read", false),
            ("PUT", "/tenants/:id", "admin:tenant:update", "write", false),
            ("DELETE", "/tenants/:id", "admin:tenant:delete", "delete", false),
            ("GET", "/departments", "admin:dept:read", "read", false),
            ("POST", "/departments", "admin:dept:create", "write", false),
            ("GET", "/roles", "admin:role:read", "read", false),
            ("POST", "/roles", "admin:role:create", "write", false),
            ("GET", "/audit-logs", "admin:audit:read", "read", false),
        ]),
        ("scheduler", "/api/enterprise/scheduler", vec![
            ("GET", "/tasks", "scheduler:read", "read", false),
            ("POST", "/tasks", "scheduler:create", "write", false),
            ("GET", "/tasks/:id", "scheduler:read", "read", false),
            ("PUT", "/tasks/:id", "scheduler:update", "write", false),
            ("DELETE", "/tasks/:id", "scheduler:delete", "delete", false),
            ("POST", "/tasks/:id/trigger", "scheduler:execute", "write", false),
            ("POST", "/tasks/:id/pause", "scheduler:update", "write", false),
            ("POST", "/tasks/:id/resume", "scheduler:update", "write", false),
            ("GET", "/tasks/:id/executions", "scheduler:read", "read", false),
            ("GET", "/executions/:id", "scheduler:read", "read", false),
            ("GET", "/templates", "scheduler:read", "read", false),
            ("GET", "/stats", "scheduler:read", "read", false),
            ("GET", "/logs", "scheduler:read", "read", false),
            ("POST", "/tasks/batch", "scheduler:create", "write", false),
            ("DELETE", "/tasks/batch", "scheduler:delete", "delete", false),
        ]),
        ("system_config", "/api/enterprise/config", vec![
            ("GET", "/configs", "config:read", "read", false),
            ("POST", "/configs", "config:create", "write", false),
            ("GET", "/configs/:key", "config:read", "read", false),
            ("PUT", "/configs/:key", "config:update", "write", false),
            ("DELETE", "/configs/:key", "config:delete", "delete", false),
            ("GET", "/configs/:key/history", "config:read", "read", false),
            ("POST", "/configs/:key/rollback", "config:update", "write", false),
            ("GET", "/groups", "config:read", "read", false),
            ("GET", "/feature-flags", "config:read", "read", false),
            ("PUT", "/feature-flags/:key", "config:update", "write", false),
            ("POST", "/cache/refresh", "config:update", "write", false),
            ("GET", "/cache/stats", "config:read", "read", false),
            ("POST", "/import", "config:import", "write", false),
            ("GET", "/export", "config:export", "read", false),
            ("GET", "/stats", "config:read", "read", false),
            ("GET", "/health", "config:read", "read", false),
        ]),
        ("dictionary", "/api/enterprise/dictionary", vec![
            ("GET", "/types", "dictionary:read", "read", false),
            ("POST", "/types", "dictionary:create", "write", false),
            ("GET", "/types/:type", "dictionary:read", "read", false),
            ("PUT", "/types/:type", "dictionary:update", "write", false),
            ("DELETE", "/types/:type", "dictionary:delete", "delete", false),
            ("GET", "/types/:type/items", "dictionary:read", "read", false),
            ("POST", "/types/:type/items", "dictionary:create", "write", false),
        ]),
        ("operation_log", "/api/enterprise/operation-logs", vec![
            ("GET", "/", "operation_log:read", "read", false),
            ("GET", "/:id", "operation_log:read", "read", false),
            ("GET", "/login", "operation_log:read", "read", false),
            ("GET", "/stats", "operation_log:read", "read", false),
            ("GET", "/export", "operation_log:export", "read", false),
            ("DELETE", "/:id", "operation_log:delete", "delete", false),
        ]),
        ("file_storage", "/api/enterprise/files", vec![
            ("GET", "/", "file:read", "read", false),
            ("POST", "/upload", "file:upload", "write", false),
            ("GET", "/:id", "file:read", "read", false),
            ("GET", "/:id/download", "file:download", "read", false),
            ("DELETE", "/:id", "file:delete", "delete", false),
            ("POST", "/:id/restore", "file:update", "write", false),
            ("GET", "/stats", "file:read", "read", false),
            ("GET", "/trash", "file:read", "read", false),
            ("POST", "/batch/delete", "file:delete", "delete", false),
        ]),
        ("organization", "/api/enterprise/org", vec![
            ("GET", "/companies", "org:read", "read", false),
            ("POST", "/companies", "org:create", "write", false),
            ("GET", "/companies/:id", "org:read", "read", false),
            ("PUT", "/companies/:id", "org:update", "write", false),
            ("GET", "/departments", "org:read", "read", false),
            ("POST", "/departments", "org:create", "write", false),
            ("GET", "/departments/tree", "org:read", "read", false),
            ("GET", "/positions", "org:read", "read", false),
            ("POST", "/positions", "org:create", "write", false),
            ("GET", "/employees", "org:read", "read", false),
            ("POST", "/employees", "org:create", "write", false),
            ("GET", "/employees/:id", "org:read", "read", false),
            ("PUT", "/employees/:id", "org:update", "write", false),
        ]),
        ("mailer", "/api/enterprise/mailer", vec![
            ("GET", "/config", "mailer:read", "read", false),
            ("PUT", "/config", "mailer:update", "write", false),
            ("POST", "/send", "mailer:send", "write", false),
            ("POST", "/send/test", "mailer:send", "write", false),
            ("GET", "/templates", "mailer:read", "read", false),
            ("POST", "/templates", "mailer:create", "write", false),
            ("GET", "/logs", "mailer:read", "read", false),
            ("GET", "/stats", "mailer:read", "read", false),
        ]),
        ("api_permission", "/api/enterprise/api-permission", vec![
            ("GET", "/endpoints", "api_permission:read", "read", false),
            ("GET", "/endpoints/:id", "api_permission:read", "read", false),
            ("PUT", "/endpoints/:id", "api_permission:update", "write", false),
            ("GET", "/permissions", "api_permission:read", "read", false),
            ("POST", "/permissions", "api_permission:create", "write", false),
            ("DELETE", "/permissions/:code", "api_permission:delete", "delete", false),
            ("GET", "/roles", "api_permission:read", "read", false),
            ("POST", "/roles", "api_permission:create", "write", false),
            ("PUT", "/roles/:id", "api_permission:update", "write", false),
            ("DELETE", "/roles/:id", "api_permission:delete", "delete", false),
            ("GET", "/users/:user_id", "api_permission:read", "read", false),
            ("PUT", "/users/:user_id", "api_permission:update", "write", false),
            ("POST", "/quick-grant", "api_permission:grant", "write", false),
            ("POST", "/check", "api_permission:read", "read", false),
            ("GET", "/stats", "api_permission:read", "read", false),
        ]),
        ("batch_operation", "/api/enterprise/batch", vec![
            ("GET", "/templates", "batch:read", "read", false),
            ("GET", "/templates/:id", "batch:read", "read", false),
            ("POST", "/templates", "batch:create", "write", false),
            ("PUT", "/templates/:id", "batch:update", "write", false),
            ("DELETE", "/templates/:id", "batch:delete", "delete", false),
            ("POST", "/import/preview", "batch:import", "write", false),
            ("POST", "/export/template", "batch:export", "read", false),
            ("GET", "/stats", "batch:read", "read", false),
        ]),
    ];

    for (module, base_path, routes) in &modules {
        for (method, path, permission_code, category, is_public) in routes {
            let full_path = format!("{}{}", base_path, path);
            let endpoint_id = format!("{}:{}", method, full_path);
            endpoints.push(ApiEndpoint {
                endpoint_id,
                method: method.to_string(),
                path: full_path,
                module: module.to_string(),
                description: None,
                permission_code: Some(permission_code.to_string()),
                require_auth: !is_public,
                require_permission: !is_public,
                is_public: *is_public,
                category: category.to_string(),
                registered_at: now.clone(),
            });
        }
    }

    endpoints
}

/// 构建标准权限点
fn build_standard_permissions() -> Vec<Permission> {
    let now = chrono::Utc::now().to_rfc3339();
    let mut permissions = Vec::new();

    let module_permissions = vec![
        ("integration", "集成适配器", vec![
            ("integration:read", "查看集成", "read"),
            ("integration:create", "创建集成", "write"),
            ("integration:update", "更新集成", "write"),
            ("integration:delete", "删除集成", "delete"),
            ("integration:test", "测试连接", "write"),
            ("integration:sync", "同步数据", "write"),
        ]),
        ("sso", "单点登录", vec![
            ("sso:read", "查看SSO配置", "read"),
            ("sso:create", "创建SSO配置", "write"),
            ("sso:update", "更新SSO配置", "write"),
            ("sso:delete", "删除SSO配置", "delete"),
            ("sso:login", "登录", "read"),
            ("sso:logout", "登出", "read"),
        ]),
        ("message", "消息中心", vec![
            ("message:read", "查看消息", "read"),
            ("message:create", "发送消息", "write"),
            ("message:update", "更新消息", "write"),
            ("message:delete", "删除消息", "delete"),
        ]),
        ("document", "文档管理", vec![
            ("document:read", "查看文档", "read"),
            ("document:create", "创建文档", "write"),
            ("document:update", "更新文档", "write"),
            ("document:delete", "删除文档", "delete"),
            ("document:sign", "电子签章", "write"),
            ("document:download", "下载文档", "read"),
        ]),
        ("admin", "系统管理", vec![
            ("admin:tenant:read", "查看租户", "read"),
            ("admin:tenant:create", "创建租户", "write"),
            ("admin:tenant:update", "更新租户", "write"),
            ("admin:tenant:delete", "删除租户", "delete"),
            ("admin:dept:read", "查看部门", "read"),
            ("admin:dept:create", "创建部门", "write"),
            ("admin:role:read", "查看角色", "read"),
            ("admin:role:create", "创建角色", "write"),
            ("admin:audit:read", "查看审计日志", "read"),
        ]),
        ("scheduler", "定时任务", vec![
            ("scheduler:read", "查看任务", "read"),
            ("scheduler:create", "创建任务", "write"),
            ("scheduler:update", "更新任务", "write"),
            ("scheduler:delete", "删除任务", "delete"),
            ("scheduler:execute", "执行任务", "write"),
        ]),
        ("config", "系统配置", vec![
            ("config:read", "查看配置", "read"),
            ("config:create", "创建配置", "write"),
            ("config:update", "更新配置", "write"),
            ("config:delete", "删除配置", "delete"),
            ("config:import", "导入配置", "import"),
            ("config:export", "导出配置", "export"),
        ]),
        ("dictionary", "数据字典", vec![
            ("dictionary:read", "查看字典", "read"),
            ("dictionary:create", "创建字典", "write"),
            ("dictionary:update", "更新字典", "write"),
            ("dictionary:delete", "删除字典", "delete"),
        ]),
        ("operation_log", "操作日志", vec![
            ("operation_log:read", "查看日志", "read"),
            ("operation_log:delete", "删除日志", "delete"),
            ("operation_log:export", "导出日志", "export"),
        ]),
        ("file", "文件存储", vec![
            ("file:read", "查看文件", "read"),
            ("file:upload", "上传文件", "write"),
            ("file:download", "下载文件", "read"),
            ("file:update", "更新文件", "write"),
            ("file:delete", "删除文件", "delete"),
        ]),
        ("org", "组织机构", vec![
            ("org:read", "查看组织", "read"),
            ("org:create", "创建组织", "write"),
            ("org:update", "更新组织", "write"),
            ("org:delete", "删除组织", "delete"),
        ]),
        ("mailer", "邮件服务", vec![
            ("mailer:read", "查看邮件配置", "read"),
            ("mailer:update", "更新邮件配置", "write"),
            ("mailer:create", "创建邮件模板", "write"),
            ("mailer:send", "发送邮件", "write"),
        ]),
        ("api_permission", "API权限", vec![
            ("api_permission:read", "查看权限", "read"),
            ("api_permission:create", "创建权限", "write"),
            ("api_permission:update", "更新权限", "write"),
            ("api_permission:delete", "删除权限", "delete"),
            ("api_permission:grant", "一键授权", "write"),
        ]),
        ("batch", "批量操作", vec![
            ("batch:read", "查看批量操作", "read"),
            ("batch:create", "创建模板", "write"),
            ("batch:update", "更新模板", "write"),
            ("batch:delete", "删除模板", "delete"),
            ("batch:import", "导入数据", "import"),
            ("batch:export", "导出数据", "export"),
        ]),
    ];

    for (module, module_name, perms) in &module_permissions {
        for (i, (code, name, perm_type)) in perms.iter().enumerate() {
            permissions.push(Permission {
                permission_code: code.to_string(),
                permission_name: name.to_string(),
                module: module.to_string(),
                permission_type: perm_type.to_string(),
                description: Some(format!("{}-{}", module_name, name)),
                endpoints: Vec::new(),
                is_system: true,
                parent_code: None,
                sort_order: i as i32 + 1,
                created_at: now.clone(),
            });
        }
    }

    permissions
}

// ==================== 权限验证工具 ====================

/// 权限验证结果（用于测试）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionVerificationResult {
    pub endpoint_id: String,
    pub method: String,
    pub path: String,
    pub module: String,
    pub required_permission: Option<String>,
    pub is_public: bool,
    pub registered: bool,
    pub permission_exists: bool,
}

/// 验证所有API端点的权限配置
pub async fn verify_all_endpoints(state: &Arc<ApiPermissionState>) -> Vec<PermissionVerificationResult> {
    let endpoints = state.endpoints.read().await;
    let permissions = state.permissions.read().await;

    let mut results = Vec::new();
    for ep in endpoints.values() {
        let permission_exists = match &ep.permission_code {
            Some(code) => permissions.contains_key(code),
            None => true,
        };

        results.push(PermissionVerificationResult {
            endpoint_id: ep.endpoint_id.clone(),
            method: ep.method.clone(),
            path: ep.path.clone(),
            module: ep.module.clone(),
            required_permission: ep.permission_code.clone(),
            is_public: ep.is_public,
            registered: true,
            permission_exists,
        });
    }

    results
}
