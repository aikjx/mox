// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 业务域反向代理适配层（L6 归一化收敛）
//!
//! # 架构定位
//! 网关（3080）原生承载 IAM 域（/api/system/* · /api/security/*）+ KG + AI引擎 + Alliance。
//! 编排器（mox-platform-orchestrator-svc，默认 :3001）承载全部业务域
//! （/api/ai/* · /api/graph/* · /api/market/* · /api/agent/* · /api/mox/* ·
//!  /api/governance/* · /api/caomei/* · /api/automation/* · /api/operators · /api/execute 等）。
//!
//! 本模块作为网关→编排器的反向代理适配层，将未被网关原生路由匹配的 /api/* 请求
//! 透明转发到编排器，保持前端单一入口（:3080），实现「归一化入口 + 模块化后端」。
//!
//! # 路由优先级
//! axum 路由匹配按具体度排序：/api/system/* · /api/security/* · /api/v1/* 等
//! 网关原生路由优先匹配；未命中的 /api/{*path} 落入本代理的 wildcard 路由。

use axum::{
    body::Body,
    extract::{FromRequestParts, OriginalUri, State},
    http::{request::Parts, HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
    Router,
};
use mox_platform_api::UserInfo;
use std::time::Duration;

/// 代理共享状态：持有 reqwest 客户端 + 编排器目标地址
#[derive(Clone)]
pub struct ProxyState {
    client: reqwest::Client,
    /// 编排器服务地址，默认 http://127.0.0.1:3001
    /// 可通过环境变量 ORCHESTRATOR_URL 覆盖
    target: String,
    /// 网关→编排器调用时注入的服务令牌（Bearer），替换客户端 JWT。
    /// 编排器只认 OUS_API_TOKEN / OUS_RBAC_TOKENS，不认网关签发的用户 JWT；
    /// 取 ORCHESTRATOR_SERVICE_TOKEN，回退 OUS_API_TOKEN。
    service_token: Option<String>,
}

impl ProxyState {
    pub fn new() -> Self {
        let target = std::env::var("ORCHESTRATOR_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:3001".to_string());
        Self::with_target(target)
    }

    /// 使用指定目标地址构建代理状态（用于 PrimiFlow 等多目标代理）
    pub fn with_target(target: String) -> Self {
        let service_token = std::env::var("ORCHESTRATOR_SERVICE_TOKEN")
            .ok()
            .or_else(|| std::env::var("OUS_API_TOKEN").ok());
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(5))
            .pool_idle_timeout(Duration::from_secs(30))
            .build()
            .expect("build proxy reqwest client");
        Self { client, target, service_token }
    }

    pub fn target(&self) -> &str {
        &self.target
    }
}

impl Default for ProxyState {
    fn default() -> Self {
        Self::new()
    }
}

/// 可选的已认证用户身份提取器（P1-① 身份降级修复）。
///
/// 仅当请求经过 `auth_middleware` 且用户已认证时，`UserInfo` 才会被注入请求扩展；
/// 公开路径 / 健康探针等未认证请求返回 `None`，此时反代**不写入**任何身份头，
/// 仅保留出站服务令牌 `OUS_API_TOKEN` 做网关→下游的服务认证。
///
/// 与 `crate::auth::ApiAuth` 区别：本提取器对缺失身份不报错（返回 None），
/// 因为反代必须能转发 dev 公开路径/探针这类无身份请求。
pub struct OptionalUserInfo(pub Option<UserInfo>);

impl<S> FromRequestParts<S> for OptionalUserInfo
where
    S: Send + Sync + 'static,
{
    type Rejection = std::convert::Infallible;

    // 手写 async_trait 展开签名（与 auth.rs `ApiAuth` 同策略，避免引入 async_trait 依赖）。
    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        S: 'async_trait,
    {
        Box::pin(async move {
            Ok(OptionalUserInfo(parts.extensions.get::<UserInfo>().cloned()))
        })
    }
}

/// 构建业务域代理路由：未被网关原生路由匹配的 /api/* 请求 → 编排器 / PrimiFlow
///
/// 多目标代理架构（归一化入口 + 模块化后端）：
/// - `/api/projects/*` → PrimiFlow（默认 :8000，项目/拓扑/资产业务）
/// - 其余 `/api/*` → 编排器（默认 :3001，AI/图谱/算子/治理/商城等全业务域）
///
/// axum 路由匹配按具体度排序：/api/projects 比 /api 更具体，优先命中 PrimiFlow；
/// /api/system/* · /api/security/* 等网关原生路由在 lib.rs 中独立 merge，优先级最高。
///
/// 注册为 Router<()> 自包含路由，由网关 lib.rs 通过 .with_state(()) 升级后 merge。
pub fn build_proxy_router() -> Router<()> {
    // 编排器目标（默认 :3001，ORCHESTRATOR_URL 可覆盖）
    let orchestrator = ProxyState::new();
    // PrimiFlow 目标（默认 :8000，PRIMIFLOW_URL 可覆盖）
    let primiflow_target = std::env::var("PRIMIFLOW_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
    let primiflow = ProxyState::with_target(primiflow_target);

    Router::new()
        // PrimiFlow 项目域（更具体前缀，优先匹配）
        .nest(
            "/api/projects",
            Router::new().fallback(proxy_handler).with_state(primiflow),
        )
        // 编排器 catch-all（其余 /api/*）
        .nest(
            "/api",
            Router::new().fallback(proxy_handler).with_state(orchestrator),
        )
        .with_state(())
}

/// 构造反代出向请求头（P1-① 身份头白名单清洗 + 服务令牌注入）。
///
/// 规则：
/// 1. 透传客户端请求头，但剥离 hop-by-hop、host、authorization；
/// 2. **防身份伪造**：客户端直接发来的 `x-user-id` / `x-user-roles` / `x-tenant-id`
///    （任意大小写变体，因 `HeaderName::as_str()` 恒为小写）一律不转发；
/// 3. 注入出站服务令牌 `Authorization: Bearer <OUS_API_TOKEN>`（仍作网关→下游服务认证）；
/// 4. 仅当 `user`（auth_middleware 已认证结果）存在时，用其可信值写入
///    `x-user-id` / `x-user-roles`（逗号拼接）/ `x-tenant-id`；未认证（dev 公开路径/探针）
///    则这些头一个都不写。
fn build_out_headers(
    headers: &HeaderMap,
    user: &Option<UserInfo>,
    service_token: &Option<String>,
    path_for_log: &str,
) -> Vec<(HeaderName, HeaderValue)> {
    let mut out_headers: Vec<(HeaderName, HeaderValue)> = Vec::new();
    for (name, value) in headers.iter() {
        let n = name.as_str();
        let hop_by_hop = matches!(
            n,
            "host" | "authorization" | "content-length" | "connection"
                | "proxy-authorization" | "proxy-authenticate" | "te"
                | "trailer" | "transfer-encoding" | "upgrade"
        );
        // 防身份伪造：外部同名身份头一律剥离
        let identity_forgery = matches!(n, "x-user-id" | "x-user-roles" | "x-tenant-id");
        if hop_by_hop || identity_forgery {
            continue;
        }
        if let Ok(vs) = value.to_str() {
            if let (Ok(hname), Ok(hval)) =
                (HeaderName::from_bytes(n.as_bytes()), HeaderValue::from_str(vs))
            {
                out_headers.push((hname, hval));
            }
        }
    }

    // 出站服务令牌（网关→下游服务认证；不代表真实用户）
    if let Some(tok) = service_token {
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {tok}")) {
            out_headers.push((axum::http::header::AUTHORIZATION, v));
        }
    } else {
        tracing::warn!(target: "gateway.proxy", path = %path_for_log, "proxy has NO service token configured");
    }

    // 仅已认证时注入可信身份头
    if let Some(u) = user {
        if let Ok(v) = HeaderValue::from_str(&u.id) {
            out_headers.push((HeaderName::from_static("x-user-id"), v));
        }
        if !u.roles.is_empty() {
            if let Ok(v) = HeaderValue::from_str(&u.roles.join(",")) {
                out_headers.push((HeaderName::from_static("x-user-roles"), v));
            }
        }
        if let Ok(v) = HeaderValue::from_str(&u.tenant_id) {
            out_headers.push((HeaderName::from_static("x-tenant-id"), v));
        }
        tracing::debug!(target: "gateway.proxy", user_id = %u.id, path = %path_for_log, "proxy injected trusted identity headers");
    }

    out_headers
}

/// 透明反向代理处理器（fallback 签名：接收完整 Request）
///
/// 将请求方法、路径、查询参数、请求头、请求体原样转发到编排器，
/// 再将编排器的响应状态码、响应头、响应体原样返回。
async fn proxy_handler(
    State(state): State<ProxyState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    OptionalUserInfo(user): OptionalUserInfo,
    body: Body,
) -> Response {
    // 构造目标 URL：保留原始 path_and_query（含查询参数）
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(uri.path());
    let target_url = format!("{}{}", state.target, path_and_query);

    // ===== 1) 构造出向请求头（一次性；P1-② 重试时复用同一份头表）=====
    // 纯函数化（build_out_headers）：P1-① 身份头白名单清洗逻辑可单测回归。
    let out_headers = build_out_headers(&headers, &user, &state.service_token, path_and_query);

    // 读取请求体字节并转发（Bytes 廉价克隆，重试时复用同一份）
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("{{\"success\":false,\"error\":\"读取请求体失败: {e}\"}}")))
                .unwrap();
        }
    };

    // ===== 2) 出向发送（P1-②：仅幂等 GET 做指数退避重试；写操作绝不重试）=====
    // 最多额外 2 次重试，退避 200ms→400ms；仅对连接错误 / 超时 / 5xx 重试。
    // POST/PUT/DELETE 等写操作 max_extra_retries=0，绝不重试以防重复提交。
    let is_get = method == Method::GET;
    let max_extra_retries: u32 = if is_get { 2 } else { 0 };
    let mut attempt: u32 = 0;
    let upstream_resp: reqwest::Response = loop {
        let mut builder = state.client.request(method.clone(), &target_url);
        for (h, v) in &out_headers {
            builder = builder.header(h, v);
        }
        match builder.body(body_bytes.clone()).send().await {
            Ok(resp) => {
                let status = resp.status();
                // 仅 5xx 且幂等 GET 才重试；4xx 属客户端错误，立即返回
                if status.is_server_error() && attempt < max_extra_retries {
                    attempt += 1;
                    let delay_ms = 200u64 * (1 << (attempt - 1)); // 200ms → 400ms
                    tracing::warn!(
                        target: "gateway.proxy",
                        %status, attempt, delay_ms, path = %path_and_query,
                        "upstream 5xx, retrying idempotent GET"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                break resp;
            }
            Err(e) => {
                // 仅连接错误 / 超时 且幂等 GET 才重试
                let retryable = e.is_connect() || e.is_timeout();
                if retryable && attempt < max_extra_retries {
                    attempt += 1;
                    let delay_ms = 200u64 * (1 << (attempt - 1));
                    tracing::warn!(
                        target: "gateway.proxy",
                        attempt, delay_ms, path = %path_and_query, error = %e,
                        "upstream connect/timeout error, retrying idempotent GET"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
                // 快速失败：下游不可达立即 502/504，不让每次请求等满 120s 超时
                let status = if e.is_connect() {
                    StatusCode::BAD_GATEWAY
                } else if e.is_timeout() {
                    StatusCode::GATEWAY_TIMEOUT
                } else {
                    StatusCode::BAD_GATEWAY
                };
                return Response::builder()
                    .status(status)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        "{{\"success\":false,\"code\":\"UPSTREAM_UNREACHABLE\",\"error\":\"上游服务不可达: {e}\",\"target\":\"{}\"}}",
                        state.target
                    )))
                    .unwrap();
            }
        }
    };

    // 构建响应：复制状态码
    let status = upstream_resp.status();
    let mut resp_builder = Response::builder().status(status);

    // 复制响应头（跳过 hop-by-hop）
    for (name, value) in upstream_resp.headers().iter() {
        let n = name.as_str();
        if matches!(n, "connection" | "transfer-encoding" | "proxy-authenticate"
            | "proxy-authorization" | "te" | "trailer" | "upgrade")
        {
            continue;
        }
        if let (Ok(hname), Ok(hval)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            resp_builder = resp_builder.header(hname, hval);
        }
    }

    // 转发响应体
    let resp_body = upstream_resp.bytes().await.unwrap_or_default();
    resp_builder.body(Body::from(resp_body)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_state_default_target() {
        let state = ProxyState::new();
        // 默认指向 3001（除非环境变量覆盖）
        assert!(state.target().contains("3001") || std::env::var("ORCHESTRATOR_URL").is_ok());
    }

    fn value_of<'a>(
        headers: &'a [(HeaderName, HeaderValue)],
        name: &str,
    ) -> Option<&'a HeaderValue> {
        headers.iter().find(|(n, _)| n.as_str() == name).map(|(_, v)| v)
    }

    fn sample_user() -> UserInfo {
        UserInfo {
            id: "alice".into(),
            username: "alice".into(),
            email: "alice@x".into(),
            tenant_id: "tenant-1".into(),
            roles: vec!["admin".into(), "editor".into()],
            enabled: true,
            created_at: String::new(),
        }
    }

    /// P1-①：客户端伪造的身份头（含大小写变体）必须被剥离；
    /// 已认证时用可信值覆盖写入。
    #[test]
    fn test_identity_headers_are_stripped_and_reinjected_when_authenticated() {
        let mut incoming = HeaderMap::new();
        // 客户端伪造：全大写/混合大小写（用 from_bytes 构造，模拟外部任意大小写）
        incoming.insert(
            HeaderName::from_bytes("X-User-Id".as_bytes()).unwrap(),
            HeaderValue::from_static("forge-admin"),
        );
        incoming.insert(
            HeaderName::from_bytes("X-Tenant-ID".as_bytes()).unwrap(),
            HeaderValue::from_static("evil-tenant"),
        );
        incoming.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("application/json"));

        let user = Some(sample_user());
        let out = build_out_headers(&incoming, &user, &Some("svc-token".into()), "/api/x");

        // 伪造值不得透传
        assert!(
            value_of(&out, "x-user-id").map(|v| v.as_bytes()) != Some("forge-admin".as_bytes()),
            "伪造的 X-User-Id 不得透传到下游"
        );
        assert!(
            value_of(&out, "x-tenant-id").map(|v| v.as_bytes()) != Some("evil-tenant".as_bytes()),
            "伪造的 X-Tenant-Id 不得透传到下游"
        );
        // 可信值写入
        assert_eq!(value_of(&out, "x-user-id").unwrap(), "alice");
        assert_eq!(value_of(&out, "x-user-roles").unwrap(), "admin,editor");
        assert_eq!(value_of(&out, "x-tenant-id").unwrap(), "tenant-1");
        // 服务令牌仍在
        assert!(value_of(&out, "authorization").unwrap().to_str().unwrap().starts_with("Bearer "));
        // 普通头透传
        assert_eq!(value_of(&out, "content-type").unwrap(), "application/json");
    }

    /// P1-①：未认证（公开路径/探针）时，外部伪造身份头必须全部被剥离，且不补写。
    #[test]
    fn test_identity_headers_fully_stripped_when_unauthenticated() {
        let mut incoming = HeaderMap::new();
        incoming.insert(HeaderName::from_static("x-user-id"), HeaderValue::from_static("forge"));
        incoming.insert(
            HeaderName::from_bytes("X-User-Roles".as_bytes()).unwrap(),
            HeaderValue::from_static("admin"),
        );
        incoming.insert(HeaderName::from_static("x-tenant-id"), HeaderValue::from_static("forge"));

        let out = build_out_headers(&incoming, &None, &Some("svc".into()), "/health");
        assert!(value_of(&out, "x-user-id").is_none(), "未认证不得输出 x-user-id");
        assert!(value_of(&out, "x-user-roles").is_none(), "未认证不得输出 x-user-roles");
        assert!(value_of(&out, "x-tenant-id").is_none(), "未认证不得输出 x-tenant-id");
        // 服务令牌仍作为网关→下游认证保留
        assert!(value_of(&out, "authorization").is_some());
    }

    /// 客户端 Authorization（JWT）必须被剥离，统一换成服务令牌。
    #[test]
    fn test_client_jwt_replaced_by_service_token() {
        let mut incoming = HeaderMap::new();
        incoming.insert(axum::http::header::AUTHORIZATION, HeaderValue::from_static("Bearer client-jwt-xyz"));
        let out = build_out_headers(&incoming, &None, &Some("svc-token".into()), "/api/x");
        let auth = value_of(&out, "authorization").unwrap().to_str().unwrap().to_string();
        assert!(auth.starts_with("Bearer svc-token"));
        assert!(!auth.contains("client-jwt-xyz"));
    }
}
