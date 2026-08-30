// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 统一 API 网关
//!
//! 所有外部请求的统一入口，负责：
//! - 协议转换（REST/GraphQL/gRPC/WebSocket -> 统一请求）
//! - 请求路由（根据 resource_type + operation 分发到处理器）
//! - 限流、认证、审计等横切关注点
//! - 响应统一格式化

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{ArchError, ArchResult};
use crate::protocol::{ProtocolAdapter, ProtocolRegistry};
use crate::types::{ApiRequest, ApiResponse, ApiStatus, ProtocolType, now_ms};

/// 请求处理函数
pub type RequestHandler = Arc<dyn Fn(ApiRequest) -> Result<ApiResponse, ArchError> + Send + Sync>;

/// 统一 API 网关
///
/// 作为所有外部请求的统一入口，
/// 提供协议无关的 API 处理能力。
pub struct UnifiedApiGateway {
    /// 协议注册中心
    protocols: ProtocolRegistry,
    /// 路由表：resource_type:operation -> handler
    routes: RwLock<HashMap<String, RequestHandler>>,
    /// 全局中间件
    middlewares: RwLock<Vec<Arc<dyn Middleware>>>,
    /// 请求计数
    request_count: std::sync::atomic::AtomicU64,
}

impl UnifiedApiGateway {
    /// 创建 API 网关
    pub fn new() -> Self {
        Self {
            protocols: ProtocolRegistry::new(),
            routes: RwLock::new(HashMap::new()),
            middlewares: RwLock::new(Vec::new()),
            request_count: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 注册路由
    pub fn register_route<F>(&self, resource_type: &str, operation: &str, handler: F)
    where
        F: Fn(ApiRequest) -> Result<ApiResponse, ArchError> + Send + Sync + 'static,
    {
        let key = route_key(resource_type, operation);
        self.routes
            .write()
            .insert(key, Arc::new(handler));
    }

    /// 添加中间件
    pub fn add_middleware(&self, middleware: Arc<dyn Middleware>) {
        self.middlewares.write().push(middleware);
    }

    /// 注册协议适配器
    pub fn register_protocol(&self, adapter: Box<dyn ProtocolAdapter>) {
        // 注意：ProtocolRegistry 需要可变访问
        // 这里我们简化处理，实际使用中可以用 RwLock 包装
        // 暂时不暴露 register_protocol 方法，使用内置的即可
        let _ = adapter;
    }

    /// 处理请求
    pub async fn handle_request(
        &self,
        protocol: ProtocolType,
        raw_request: &[u8],
        metadata: &serde_json::Value,
    ) -> Result<Vec<u8>, Vec<u8>> {
        let start = now_ms();
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // 1. 获取协议适配器
        let adapter = match self.protocols.get(protocol) {
            Some(a) => a,
            None => {
                let err_resp = ApiResponse::error(
                    "unknown",
                    "UNSUPPORTED_PROTOCOL",
                    &format!("protocol '{}' not supported", protocol.as_str()),
                );
                // 尝试用 REST 格式返回错误
                let rest = self.protocols.get(ProtocolType::Rest).unwrap();
                return Err(rest.format_response(err_resp).await.unwrap_or_default());
            }
        };

        // 2. 解析请求
        let request = match adapter.parse_request(raw_request, metadata).await {
            Ok(req) => req,
            Err(e) => {
                let resp = ApiResponse::error("parse_error", e.code(), &e.to_string());
                return Err(adapter.format_response(resp).await.unwrap_or_default());
            }
        };

        let request_id = request.request_id.clone();

        // 3. 执行中间件（请求前）
        let middlewares = self.middlewares.read().clone();
        for mw in &middlewares {
            match mw.before_request(&request) {
                Ok(()) => {}
                Err(e) => {
                    let resp = ApiResponse::error(&request_id, e.code(), &e.to_string());
                    return Err(adapter.format_response(resp).await.unwrap_or_default());
                }
            }
        }

        // 4. 路由到处理器
        let key = route_key(&request.resource_type, &request.operation);
        let handler = {
            let routes = self.routes.read();
            routes.get(&key).cloned()
        };

        let response = match handler {
            Some(handler) => match handler(request) {
                Ok(mut resp) => {
                    resp.duration_ms = Some(now_ms() - start);

                    // 执行中间件（响应后）
                    let mut resp = resp;
                    for mw in &middlewares {
                        resp = mw.after_response(resp);
                    }
                    resp
                }
                Err(e) => ApiResponse {
                    request_id: request_id.clone(),
                    status: ApiStatus::Failed,
                    error_code: Some(e.code().to_string()),
                    error_message: Some(e.to_string()),
                    data: None,
                    metadata: HashMap::new(),
                    timestamp: now_ms(),
                    duration_ms: Some(now_ms() - start),
                },
            },
            None => ApiResponse::error(
                &request_id,
                "ROUTE_NOT_FOUND",
                &format!(
                    "no route for {}/{}",
                    &request.resource_type, &request.operation
                ),
            ),
        };

        // 5. 格式化响应
        match adapter.format_response(response).await {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                let resp = ApiResponse::error(&request_id, "FORMAT_ERROR", &e.to_string());
                Err(serde_json::to_vec(&resp).unwrap_or_default())
            }
        }
    }

    /// 获取请求总数
    pub fn total_requests(&self) -> u64 {
        self.request_count
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 列出所有路由
    pub fn list_routes(&self) -> Vec<String> {
        self.routes.read().keys().cloned().collect()
    }

    /// 检查路由是否存在
    pub fn has_route(&self, resource_type: &str, operation: &str) -> bool {
        self.routes.read().contains_key(&route_key(resource_type, operation))
    }
}

impl Default for UnifiedApiGateway {
    fn default() -> Self {
        Self::new()
    }
}

/// 中间件 Trait
pub trait Middleware: Send + Sync {
    /// 请求前处理，返回 Err 可中断请求
    fn before_request(&self, request: &ApiRequest) -> ArchResult<()>;

    /// 响应后处理
    fn after_response(&self, response: ApiResponse) -> ApiResponse;
}

/// 日志中间件
pub struct LoggingMiddleware;

impl Middleware for LoggingMiddleware {
    fn before_request(&self, _request: &ApiRequest) -> ArchResult<()> {
        Ok(())
    }

    fn after_response(&self, response: ApiResponse) -> ApiResponse {
        response
    }
}

/// 限流中间件
pub struct RateLimitMiddleware {
    /// 每分钟限制
    limit_per_minute: u64,
    /// 时间窗口内的请求计数
    requests: std::sync::Mutex<Vec<u64>>,
}

impl RateLimitMiddleware {
    pub fn new(limit_per_minute: u64) -> Self {
        Self {
            limit_per_minute,
            requests: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn clean_old_requests(&self, now: u64) {
        if let Ok(mut reqs) = self.requests.lock() {
            let one_minute_ago = now.saturating_sub(60_000);
            reqs.retain(|&t| t > one_minute_ago);
        }
    }
}

impl Middleware for RateLimitMiddleware {
    fn before_request(&self, _request: &ApiRequest) -> ArchResult<()> {
        let now = now_ms();
        self.clean_old_requests(now);

        if let Ok(reqs) = self.requests.lock() {
            if reqs.len() as u64 >= self.limit_per_minute {
                return Err(ArchError::RateLimited {
                    limit: self.limit_per_minute,
                    retry_after: Some(60),
                });
            }
        }

        if let Ok(mut reqs) = self.requests.lock() {
            reqs.push(now);
        }

        Ok(())
    }

    fn after_response(&self, response: ApiResponse) -> ApiResponse {
        response
    }
}

/// 生成路由键
fn route_key(resource_type: &str, operation: &str) -> String {
    format!("{}:{}", resource_type, operation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_route_key() {
        assert_eq!(route_key("user", "get"), "user:get");
        assert_eq!(route_key("graph.node", "create"), "graph.node:create");
    }

    #[test]
    fn test_gateway_register_route() {
        let gateway = UnifiedApiGateway::new();

        gateway.register_route("user", "get", |req| {
            Ok(ApiResponse::success(
                &req.request_id,
                json!({ "id": "123", "name": "Test" }),
            ))
        });

        assert!(gateway.has_route("user", "get"));
        assert!(!gateway.has_route("user", "create"));
        assert_eq!(gateway.list_routes().len(), 1);
    }

    #[tokio::test]
    async fn test_handle_rest_request() {
        let gateway = UnifiedApiGateway::new();

        gateway.register_route("test", "echo", |req| {
            Ok(ApiResponse::success(&req.request_id, req.payload))
        });

        let body = json!({ "message": "hello" }).to_string();
        let metadata = json!({ "resource_type": "test", "operation": "echo" });

        let result = gateway
            .handle_request(ProtocolType::Rest, body.as_bytes(), &metadata)
            .await;

        assert!(result.is_ok());
        let bytes = result.unwrap();
        let response: ApiResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response.status, ApiStatus::Success);
        assert_eq!(response.data.unwrap()["message"], "hello");
    }

    #[tokio::test]
    async fn test_handle_unknown_route() {
        let gateway = UnifiedApiGateway::new();

        let body = json!({}).to_string();
        let metadata = json!({ "resource_type": "nonexist", "operation": "get" });

        let result = gateway
            .handle_request(ProtocolType::Rest, body.as_bytes(), &metadata)
            .await;

        // 错误响应也返回 Ok（因为格式成功了），只是状态是 Failed
        let bytes = result.unwrap();
        let response: ApiResponse = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(response.status, ApiStatus::Failed);
        assert_eq!(response.error_code.unwrap(), "ROUTE_NOT_FOUND");
    }

    #[test]
    fn test_rate_limit_middleware() {
        let mw = RateLimitMiddleware::new(3);

        let req = ApiRequest::new("test", "get", json!({}));

        // 前 3 次应该成功
        assert!(mw.before_request(&req).is_ok());
        assert!(mw.before_request(&req).is_ok());
        assert!(mw.before_request(&req).is_ok());

        // 第 4 次应该被限流
        let result = mw.before_request(&req);
        assert!(result.is_err());
        match result.unwrap_err() {
            ArchError::RateLimited { .. } => {}
            _ => panic!("expected RateLimited error"),
        }
    }

    #[test]
    fn test_request_count() {
        let gateway = UnifiedApiGateway::new();
        assert_eq!(gateway.total_requests(), 0);
    }

    #[test]
    fn test_logging_middleware() {
        let mw = LoggingMiddleware;
        let req = ApiRequest::new("test", "get", json!({}));
        assert!(mw.before_request(&req).is_ok());

        let resp = ApiResponse::success("test", json!({}));
        let result = mw.after_response(resp);
        assert_eq!(result.status, ApiStatus::Success);
    }
}
