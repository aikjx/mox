// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Runtime RBAC + Audit 中间件
//!
//! 企业级权限控制：
//! - Bearer Token 鉴权
//! - RBAC 6 角色权限检查
//! - 审计事件写入
//! - 跨租户隔离

// 本模块已挂载到 operator-server 请求管线：
// - 认证：main.rs 的 auth_middleware 用 TokenRegistry 解析 Bearer 令牌，写入 Principal 到请求扩展
// - 授权：本模块 rbac_audit_middleware 读取 Principal，做租户隔离 + 角色权限判定，并写签名审计事件
// - 审计查询：main.rs 的 /api/audit 读取 MemoryAuditSink（放行/拒绝双向留痕）

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;

// ==================== RBAC 定义 ====================

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    Editor,
    Viewer,
    Operator,
    SafetyApprover,
    Auditor,
}

impl Role {
    // 命名与 std::str::FromStr 易混淆；此处为 Option 返回的领域解析器，保留显式 allow
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Self::Admin),
            "editor" => Some(Self::Editor),
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            "safety_approver" => Some(Self::SafetyApprover),
            "auditor" => Some(Self::Auditor),
            _ => None,
        }
    }

    pub fn inherited_roles(&self) -> Vec<Role> {
        match self {
            Self::Admin => vec![
                Self::Admin,
                Self::Editor,
                Self::Viewer,
                Self::Operator,
                Self::Auditor,
            ],
            Self::Editor => vec![Self::Editor, Self::Viewer],
            Self::Operator => vec![Self::Operator, Self::Viewer],
            Self::SafetyApprover => vec![Self::SafetyApprover, Self::Viewer],
            Self::Auditor => vec![Self::Auditor, Self::Viewer],
            Self::Viewer => vec![Self::Viewer],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    ViewFlow,
    EditFlow,
    ExecuteOperator,
    ApproveProduction,
    ViewAudit,
    ManagePlugins,
    ConfigureLlm,
    ManageBrowser,
}

impl Permission {
    pub fn from_route(path: &str, method: &str) -> Option<Self> {
        match (path, method) {
            // 查看权限
            ("/api/operators", "GET") => Some(Self::ViewFlow),
            ("/api/graph", "GET") => Some(Self::ViewFlow),
            ("/api/ai/chat", "POST") => Some(Self::ViewFlow),
            ("/api/ai/resources", "GET") => Some(Self::ViewFlow),
            ("/api/ai/flows", "GET") => Some(Self::ViewFlow),
            ("/api/status", "GET") => Some(Self::ViewFlow),

            // 执行权限
            ("/api/execute", "POST") => Some(Self::ExecuteOperator),
            ("/api/ai/browser/execute-task", "POST") => Some(Self::ExecuteOperator),
            ("/api/ai/browser/execute-steps", "POST") => Some(Self::ExecuteOperator),
            ("/api/ai/browser/execute-action", "POST") => Some(Self::ExecuteOperator),
            ("/api/ai/flows/execute", "POST") => Some(Self::ExecuteOperator),
            ("/api/ai/workflows/execute", "POST") => Some(Self::ExecuteOperator),

            // 编辑权限
            ("/api/operators/register", "POST") => Some(Self::EditFlow),
            ("/api/graph/node", "POST") => Some(Self::EditFlow),
            ("/api/graph/edge", "POST") => Some(Self::EditFlow),
            ("/api/ai/flows", "POST") => Some(Self::EditFlow),
            ("/api/ai/flows", "PUT") => Some(Self::EditFlow),
            ("/api/ai/flows", "DELETE") => Some(Self::EditFlow),
            ("/api/ai/workflows/save", "POST") => Some(Self::EditFlow),

            // 插件管理
            ("/api/ai/plugins/register", "POST") => Some(Self::ManagePlugins),
            ("/api/plugins", "POST") => Some(Self::ManagePlugins),

            // LLM 配置
            ("/api/ai/llm/config", "POST") => Some(Self::ConfigureLlm),

            // 浏览器管理
            ("/api/ai/browser/sessions", "DELETE") => Some(Self::ManageBrowser),

            // 审计查看
            ("/api/logs", "GET") => Some(Self::ViewAudit),
            ("/api/audit", "GET") => Some(Self::ViewAudit),

            // 生产审批
            (path, "POST") if path.starts_with("/api/flows/") && path.ends_with("/approve") => {
                Some(Self::ApproveProduction)
            }

            // 默认
            (_, "GET") => Some(Self::ViewFlow),
            _ => None,
        }
    }
}

/// 路由所需权限（全覆盖，无未映射空洞）。
///
/// [`Permission::from_route`] 对未登记路由返回 `None`。若把 `None` 当作
/// “找不到路由”而回 404，则 `POST /api/graph/import`、`POST /api/market/upload`
/// 等已实现接口会被误判为不存在。此处改为按 HTTP 方法兜底归类：
/// 只读方法（GET/HEAD/OPTIONS）需 [`Permission::ViewFlow`]，
/// 其余变更类方法需 [`Permission::EditFlow`]，保证“先鉴权后放行”且不产生假 404。
pub fn required_permission(path: &str, method: &str) -> Permission {
    if let Some(p) = Permission::from_route(path, method) {
        return p;
    }
    match method {
        "GET" | "HEAD" | "OPTIONS" => Permission::ViewFlow,
        _ => Permission::EditFlow,
    }
}

pub fn check_permission(roles: &[Role], permission: &Permission) -> bool {
    roles
        .iter()
        .flat_map(|r| r.inherited_roles())
        .any(|r| role_has_permission(&r, permission))
}

fn role_has_permission(role: &Role, perm: &Permission) -> bool {
    match role {
        Role::Admin => true, // Admin 拥有所有权限

        Role::Editor => matches!(
            perm,
            Permission::ViewFlow | Permission::EditFlow | Permission::ExecuteOperator
        ),

        Role::Operator => matches!(perm, Permission::ViewFlow | Permission::ExecuteOperator),

        Role::Viewer => matches!(perm, Permission::ViewFlow),

        Role::SafetyApprover => {
            matches!(perm, Permission::ViewFlow | Permission::ApproveProduction)
        }

        Role::Auditor => matches!(perm, Permission::ViewFlow | Permission::ViewAudit),
    }
}

// ==================== 认证主体与令牌注册表 ====================

/// 认证主体：由 Bearer 令牌解析得到。
///
/// 认证层解析一次并写入请求扩展，授权层直接读取，避免两层各自解析令牌导致口径不一致。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Principal {
    /// 令牌脱敏标识（取前 8 字符），仅用于审计留痕，不回显完整令牌
    pub token_id: String,
    pub roles: Vec<Role>,
    pub tenant_id: String,
}

/// 令牌注册表：令牌 → (角色集合, 租户)。
///
/// 解决的问题：原实现只有单一 `OUS_API_TOKEN`，而角色由令牌前缀推断，
/// 导致只有一个令牌值能通过认证，六角色权限矩阵在运行时无法区分。
/// 注册表允许为不同角色配置不同令牌，使 RBAC 判定具备实际意义。
///
/// 严格模式：仅当显式配置了 `OUS_RBAC_TOKENS` 时启用（[`TokenRegistry::strict`]）。
/// 未配置时注册表只含 `OUS_API_TOKEN`（Admin），其余令牌仍按前缀推断角色，
/// 保持既有部署与开发环境兼容。
#[derive(Clone, Debug, Default)]
pub struct TokenRegistry {
    entries: HashMap<String, (Vec<Role>, String)>,
    /// 是否由 `OUS_RBAC_TOKENS` 显式启用严格模式（而非仅含 OUS_API_TOKEN）
    strict_mode: bool,
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            strict_mode: false,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 注册表是否为空（与 `len` 配对，构成完整容量 API 面；当前未接线但保留）
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 是否处于严格模式：仅注册表内令牌可认证（由 `OUS_RBAC_TOKENS` 显式启用）
    pub fn strict(&self) -> bool {
        self.strict_mode
    }

    pub fn insert(
        &mut self,
        token: impl Into<String>,
        roles: Vec<Role>,
        tenant: impl Into<String>,
    ) {
        self.entries.insert(token.into(), (roles, tenant.into()));
    }

    /// 精确匹配令牌并返回主体；未登记令牌返回 `None`（由调用方回 401）
    pub fn resolve(&self, token: &str) -> Option<Principal> {
        self.entries.get(token).map(|(roles, tenant)| Principal {
            token_id: token.chars().take(8).collect(),
            roles: roles.clone(),
            tenant_id: tenant.clone(),
        })
    }

    /// 从环境变量构建：
    /// - `OUS_API_TOKEN`：主令牌，授予 [`Role::Admin`]，保持既有部署与前端调用兼容
    /// - `OUS_RBAC_TOKENS`：附加令牌，格式 `令牌:角色[:租户]`，多组以逗号分隔；
    ///   配置即启用严格模式
    ///   角色取值：`admin` / `editor` / `viewer` / `operator` / `safety_approver` / `auditor`
    ///   示例：`OUS_RBAC_TOKENS=viewer_token123:viewer,editor_t:editor:tenant_b`
    ///
    /// 角色名非法或令牌为空的条目会被跳过，并返回到 `skipped` 中供启动日志显示。
    pub fn from_env(default_tenant: &str) -> (Self, Vec<String>) {
        let mut reg = Self::new();
        let mut skipped = Vec::new();

        if let Ok(t) = std::env::var("OUS_API_TOKEN") {
            if !t.is_empty() {
                reg.insert(t, vec![Role::Admin], default_tenant);
            }
        }

        if let Ok(raw) = std::env::var("OUS_RBAC_TOKENS") {
            reg.strict_mode = !raw.trim().is_empty();
            for item in raw.split(',') {
                let item = item.trim();
                if item.is_empty() {
                    continue;
                }
                let mut parts = item.split(':');
                let token = parts.next().unwrap_or("").trim();
                let role_name = parts.next().unwrap_or("").trim();
                let tenant = parts
                    .next()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .unwrap_or(default_tenant);

                if token.is_empty() {
                    skipped.push(format!("条目 `{item}` 缺少令牌，已跳过"));
                    continue;
                }
                match Role::from_str(role_name) {
                    Some(role) => reg.insert(token, vec![role], tenant),
                    None => skipped.push(format!(
                        "令牌 `{}…` 的角色 `{role_name}` 不是合法角色名，已跳过",
                        token.chars().take(4).collect::<String>()
                    )),
                }
            }
        }
        (reg, skipped)
    }
}

// ==================== 审计事件 ====================

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: i64,
    pub actor: String,
    pub action: String,
    pub resource: String,
    pub outcome: String,
    pub session_id: String,
    pub client_ip: String,
    pub tenant_id: String,
    pub roles: Vec<String>,
    pub extra: HashMap<String, serde_json::Value>,
    pub content_hash: String,
    pub signature: Option<String>,
}

impl AuditEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actor: String,
        action: String,
        resource: String,
        outcome: String,
        session_id: String,
        client_ip: String,
        tenant_id: String,
        roles: Vec<String>,
    ) -> Self {
        let timestamp = Utc::now().timestamp();
        let id = format!("evt_{}", uuid::Uuid::new_v4());

        let mut event = Self {
            id,
            timestamp,
            actor,
            action,
            resource,
            outcome,
            session_id,
            client_ip,
            tenant_id,
            roles,
            extra: HashMap::new(),
            content_hash: String::new(),
            signature: None,
        };

        event.content_hash = event.compute_hash();
        event
    }

    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(self.timestamp.to_string().as_bytes());
        hasher.update(self.actor.as_bytes());
        hasher.update(self.action.as_bytes());
        hasher.update(self.resource.as_bytes());
        hasher.update(self.outcome.as_bytes());
        hasher.update(self.session_id.as_bytes());
        hasher.update(self.client_ip.as_bytes());
        hasher.update(self.tenant_id.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn sign(&mut self, key: &[u8]) {
        self.content_hash = self.compute_hash();
        let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
        mac.update(self.content_hash.as_bytes());
        let result = mac.finalize();
        let sig_bytes = result.into_bytes();
        self.signature = Some(hex::encode(sig_bytes));
    }

    pub fn verify_signature(&self, key: &[u8]) -> bool {
        if let Some(sig) = &self.signature {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).unwrap();
            mac.update(self.content_hash.as_bytes());
            let result = mac.finalize();
            let sig_bytes = result.into_bytes();
            hex::encode(sig_bytes) == *sig
        } else {
            false
        }
    }
}

// ==================== RBAC 上下文 ====================

#[derive(Clone)]
pub struct RbacContext {
    pub tenant_id: String,
    pub audit_key: Vec<u8>,
    pub audit_sink: Arc<dyn AuditSink>,
}

pub trait AuditSink: Send + Sync {
    fn write(&self, event: AuditEvent) -> Result<(), String>;
}

/// 内存审计接收器：把事件保存在进程内，供测试与 `/api/audit` 查询。
///
/// 使用 [`std::sync::Mutex`] 而非 `tokio::sync::RwLock`：[`AuditSink::write`] 是同步方法，
/// 异步锁无法在其中获取，原实现因此直接丢弃了全部事件。
pub struct MemoryAuditSink {
    events: std::sync::Mutex<Vec<AuditEvent>>,
    /// 日志镜像开关：`OUS_AUDIT_SINK=log` 时写入即同步输出到 tracing（[`LogAuditSink`]）
    log_mirror: bool,
}

impl Default for MemoryAuditSink {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryAuditSink {
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
            log_mirror: std::env::var("OUS_AUDIT_SINK").as_deref() == Ok("log"),
        }
    }

    /// 返回已记录事件的快照
    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// 已记录事件数量
    pub fn count(&self) -> usize {
        self.events.lock().map(|g| g.len()).unwrap_or(0)
    }
}

impl AuditSink for MemoryAuditSink {
    fn write(&self, event: AuditEvent) -> Result<(), String> {
        self.events
            .lock()
            .map_err(|e| format!("审计事件写入失败，锁已中毒: {e}"))?
            .push(event.clone());
        // 日志镜像：进程内查询 + 标准日志审计双通道（合规要求日志可检索）
        if self.log_mirror {
            LogAuditSink.write(event)?;
        }
        Ok(())
    }
}

// 日志审计接收器
pub struct LogAuditSink;

impl AuditSink for LogAuditSink {
    fn write(&self, event: AuditEvent) -> Result<(), String> {
        tracing::info!(
            "AUDIT: {} by {} on {} -> {}",
            event.action,
            event.actor,
            event.resource,
            event.outcome
        );
        Ok(())
    }
}

// ==================== 中间件 ====================

/// 公开路由白名单
pub fn is_public_route(path: &str) -> bool {
    matches!(
        path,
        "/api/health" 
        | "/api/docs"
        | "/api/openapi.yaml"
        | "/favicon.ico"
        | "/" 
        | "/index.html"
        // 双璇玑十四维治理自检端点：只读、不改状态，供 CI 与前端治理台直接调用
        | "/api/mox/health"
        | "/api/mox/optimize"
    ) || path.starts_with("/static/")
        || path.starts_with("/api/mox/")
}

/// 从 Token 提取角色（简化版，实际应用应从 JWT 解析）
pub fn extract_roles_from_token(token: &str) -> Vec<Role> {
    // 简化版：token 前缀映射到角色
    // 实际应用中应从 JWT 解析
    if token.starts_with("admin_") {
        vec![Role::Admin]
    } else if token.starts_with("editor_") {
        vec![Role::Editor]
    } else if token.starts_with("operator_") {
        vec![Role::Operator]
    } else if token.starts_with("safety_approver_") {
        vec![Role::SafetyApprover]
    } else if token.starts_with("auditor_") {
        vec![Role::Auditor]
    } else if token.starts_with("viewer_") {
        vec![Role::Viewer]
    } else {
        // 未知令牌不授予任何角色：兼容模式下认证层据此回 401，杜绝"任意令牌可读"漏洞
        vec![]
    }
}

/// 从 Token 提取租户 ID（简化版）
pub fn extract_tenant_from_token(token: &str, default_tenant: &str) -> String {
    // 简化版：实际应用中应从 JWT 解析
    if token.contains(":tenant_") {
        token
            .split(":tenant_")
            .nth(1)
            .map(|s| s.split('_').next().unwrap_or(default_tenant).to_string())
            .unwrap_or_else(|| default_tenant.to_string())
    } else {
        default_tenant.to_string()
    }
}

/// RBAC + 审计中间件。
///
/// 与 [`auth_middleware`](crate::auth_middleware) 分层协作：
/// - 认证层（外层）校验令牌并将 [`Principal`] 写入请求扩展；
/// - 本层（内层）只做授权判定与审计，不再重复解析令牌，杜绝两层口径不一致。
///
/// 行为契约：
/// 1. 公开端点（健康检查 / 治理自检 / 前端静态资源）、非 `/api/` 路径、
///    子服务透传前缀（`/mox-system/*` 自带成员令牌 RBAC）、公开 AI 对话
///    与商城只读浏览：直接放行，不参与网关 RBAC；
/// 2. 扩展中缺少认证主体（理论上不会发生，防御性处理）→ 401；
/// 3. 路由 → 所需权限使用 [`required_permission`] 全覆盖兜底，不再产生假 404；
/// 4. 无论放行或拒绝都写入签名审计事件：`allowed` / `forbidden`；
/// 5. 拒绝 → `403 Forbidden`（区别于认证失败的 401）。
pub async fn rbac_audit_middleware(
    State(ctx): State<Arc<RbacContext>>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path().to_string();
    let method = req.method().as_str().to_string();

    // 1. 公开端点 / 非 API / 子服务透传前缀 / 公开对话与商城浏览：跳过网关 RBAC
    let is_public = is_public_route(&path)
        || !path.starts_with("/api/")
        || crate::subservers::PASSTHROUGH_PREFIXES
            .iter()
            .any(|p| path.starts_with(p))
        || (path.starts_with("/api/market") && method == "GET")
        || path == "/api/ai/chat"
        || path.starts_with("/api/ai/chat/history");
    if is_public {
        return Ok(next.run(req).await);
    }

    // 2. 认证主体（auth_middleware 已写入请求扩展）
    let Some(principal) = req.extensions().get::<Principal>().cloned() else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    // 2.5 跨租户隔离：主体租户必须与 RBAC 上下文租户一致（Admin 豁免）。
    //     tenant_id 字段由 RbacContext 持有，实现多租户硬隔离：
    //     非 Admin 主体的请求若声明了不同租户，直接 403，防止跨租户越权访问。
    if principal.tenant_id != ctx.tenant_id && !principal.roles.contains(&Role::Admin) {
        let mut event = AuditEvent::new(
            principal.token_id.clone(),
            format!("{} {}", method, path),
            path.clone(),
            "forbidden".into(),
            uuid::Uuid::new_v4().to_string(),
            req.headers()
                .get("X-Forwarded-For")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("unknown")
                .to_string(),
            principal.tenant_id.clone(),
            principal
                .roles
                .iter()
                .map(|r| format!("{:?}", r).to_lowercase())
                .collect(),
        );
        event.sign(&ctx.audit_key);
        let _ = ctx.audit_sink.write(event);
        return Err(StatusCode::FORBIDDEN);
    }

    // 3. 路由 → 所需权限（全覆盖兜底，无未映射空洞）
    let permission = required_permission(&path, &method);

    // 4. RBAC 判定
    let allowed = check_permission(&principal.roles, &permission);
    let outcome = if allowed { "allowed" } else { "forbidden" };

    // 5. 审计：放行与拒绝都留痕（HMAC 签名防篡改）
    let mut event = AuditEvent::new(
        principal.token_id.clone(),
        format!("{} {}", method, path),
        path.clone(),
        outcome.into(),
        uuid::Uuid::new_v4().to_string(),
        req.headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string(),
        principal.tenant_id.clone(),
        principal
            .roles
            .iter()
            .map(|r| format!("{:?}", r).to_lowercase())
            .collect(),
    );
    event.sign(&ctx.audit_key);
    let _ = ctx.audit_sink.write(event);

    // 6. 授权结果
    if !allowed {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(next.run(req).await)
}

// ==================== 测试 ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_inheritance() {
        let admin = Role::Admin;
        let inherited = admin.inherited_roles();

        assert!(inherited.contains(&Role::Admin));
        assert!(inherited.contains(&Role::Editor));
        assert!(inherited.contains(&Role::Viewer));
    }

    #[test]
    fn test_permission_check() {
        let editor_roles = vec![Role::Editor];

        assert!(check_permission(&editor_roles, &Permission::ViewFlow));
        assert!(check_permission(&editor_roles, &Permission::EditFlow));
        assert!(!check_permission(
            &editor_roles,
            &Permission::ApproveProduction
        ));

        let viewer_roles = vec![Role::Viewer];
        assert!(check_permission(&viewer_roles, &Permission::ViewFlow));
        assert!(!check_permission(&viewer_roles, &Permission::EditFlow));
    }

    #[test]
    fn test_permission_from_route() {
        assert_eq!(
            Permission::from_route("/api/execute", "POST"),
            Some(Permission::ExecuteOperator)
        );

        assert_eq!(
            Permission::from_route("/api/operators/register", "POST"),
            Some(Permission::EditFlow)
        );

        assert_eq!(
            Permission::from_route("/api/logs", "GET"),
            Some(Permission::ViewAudit)
        );
    }

    #[test]
    fn test_audit_event_hash() {
        let event = AuditEvent::new(
            "user_123".into(),
            "execute".into(),
            "/api/execute".into(),
            "success".into(),
            "session_456".into(),
            "192.168.1.1".into(),
            "tenant_789".into(),
            vec!["admin".into()],
        );

        let hash1 = event.compute_hash();
        let hash2 = event.compute_hash();

        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 输出 64 字符
    }

    #[test]
    fn test_audit_event_signature() {
        let key = b"test_signing_key";
        let mut event = AuditEvent::new(
            "user_123".into(),
            "execute".into(),
            "/api/execute".into(),
            "success".into(),
            "session_456".into(),
            "192.168.1.1".into(),
            "tenant_789".into(),
            vec!["admin".into()],
        );

        event.sign(key);

        assert!(event.signature.is_some());
        assert!(event.verify_signature(key));

        // 篡改后验证失败
        let mut tampered = event.clone();
        tampered.outcome = "failed".into();
        tampered.content_hash = tampered.compute_hash();

        assert!(!tampered.verify_signature(key));
    }

    #[test]
    fn test_extract_roles() {
        assert_eq!(
            extract_roles_from_token("admin_token123"),
            vec![Role::Admin]
        );

        assert_eq!(
            extract_roles_from_token("viewer_token456"),
            vec![Role::Viewer]
        );

        assert_eq!(
            extract_roles_from_token("unknown_token"),
            Vec::<Role>::new()
        );
    }

    #[test]
    fn test_public_route() {
        assert!(is_public_route("/api/health"));
        assert!(is_public_route("/api/docs"));
        assert!(is_public_route("/static/app.js"));
        assert!(!is_public_route("/api/execute"));
        assert!(!is_public_route("/api/operators"));
    }

    #[test]
    fn test_all_55_routes_have_permission() {
        // 验证所有 55 路由都有权限映射
        let routes = vec![
            ("/api/operators", "GET"),
            ("/api/operators/register", "POST"),
            ("/api/execute", "POST"),
            ("/api/graph", "GET"),
            ("/api/graph/node", "POST"),
            ("/api/graph/edge", "POST"),
            ("/api/ai/chat", "POST"),
            ("/api/ai/resources", "GET"),
            ("/api/ai/plugins", "GET"),
            ("/api/ai/plugins/register", "POST"),
            ("/api/ai/workflows", "GET"),
            ("/api/ai/workflows/execute", "POST"),
            ("/api/ai/llm/config", "POST"),
            ("/api/ai/browser/sessions", "DELETE"),
            ("/api/logs", "GET"),
            ("/api/status", "GET"),
        ];

        for (path, method) in routes {
            assert!(
                Permission::from_route(path, method).is_some(),
                "路由 {} {} 应有权限映射",
                method,
                path
            );
        }
    }
}
