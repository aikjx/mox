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

/// 透明反向代理处理器（fallback 签名：接收完整 Request）
///
/// 将请求方法、路径、查询参数、请求头、请求体原样转发到编排器，
/// 再将编排器的响应状态码、响应头、响应体原样返回。
async fn proxy_handler(
    State(state): State<ProxyState>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
    body: Body,
) -> Response {
    // 构造目标 URL：保留原始 path_and_query（含查询参数）
    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(uri.path());
    let target_url = format!("{}{}", state.target, path_and_query);

    // 构建转发请求：复制方法、URL、请求体
    let mut req_builder = state
        .client
        .request(method.clone(), &target_url);

    // 转发请求头（跳过 hop-by-hop 和 host；authorization 由下方服务令牌统一注入）
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if name_str == "host"
            || name_str == "authorization"
            || name_str == "content-length"
            || name_str == "connection"
            || name_str == "proxy-authorization"
            || name_str == "proxy-authenticate"
            || name_str == "te"
            || name_str == "trailer"
            || name_str == "transfer-encoding"
            || name_str == "upgrade"
        {
            continue;
        }
        if let Ok(v) = value.to_str() {
            req_builder = req_builder.header(name_str, v);
        }
    }

    // 注入服务令牌：编排器只认 OUS_API_TOKEN / OUS_RBAC_TOKENS，
    // 用服务令牌替换客户端 JWT，实现「网关鉴用户、后端用服务凭证」。
    if let Some(tok) = &state.service_token {
        if let Ok(v) = HeaderValue::from_str(&format!("Bearer {tok}")) {
            req_builder = req_builder.header("authorization", v);
            tracing::debug!(target: "gateway.proxy", path = %path_and_query, "proxy injected service token");
        }
    } else {
        tracing::warn!(target: "gateway.proxy", path = %path_and_query, "proxy has NO service token configured");
    }

    // 读取请求体字节并转发
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("{{\"success\":false,\"error\":\"读取请求体失败: {e}\"}}")))
                .unwrap();
        }
    };

    // 发送转发请求
    let upstream_resp = match req_builder.body(body_bytes).send().await {
        Ok(r) => r,
        Err(e) => {
            // 编排器不可达时返回 502 + 标准错误信封，前端可优雅降级
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
                    "{{\"success\":false,\"code\":\"ORCHESTRATOR_UNREACHABLE\",\"error\":\"编排器服务不可达: {e}\",\"target\":\"{}\"}}",
                    state.target
                )))
                .unwrap();
        }
    };

    // 构建响应：复制状态码
    let status = upstream_resp.status();
    let mut resp_builder = Response::builder().status(status);

    // 复制响应头（跳过 hop-by-hop）
    for (name, value) in upstream_resp.headers().iter() {
        let name_str = name.as_str();
        if name_str == "connection"
            || name_str == "transfer-encoding"
            || name_str == "proxy-authenticate"
            || name_str == "proxy-authorization"
            || name_str == "te"
            || name_str == "trailer"
            || name_str == "upgrade"
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
}
