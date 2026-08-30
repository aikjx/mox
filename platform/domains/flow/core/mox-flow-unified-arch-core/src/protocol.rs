// Copyright (c) 2026 璇玑 RelGraph · 统一架构核心 (Unified Architecture Core)
// Licensed under the MIT License.

//! 统一协议层
//!
//! 将 REST/GraphQL/gRPC/WebSocket 等不同协议的请求
//! 归一化为统一的 ApiRequest，响应也统一为 ApiResponse。

use crate::error::{ArchError, ArchResult};
use crate::types::{ApiRequest, ApiResponse, ProtocolType};
use serde_json::Value;

/// 协议适配器 Trait
///
/// 每种接入协议实现这个 trait，将特定协议的请求
/// 转换为统一的 ApiRequest，并将 ApiResponse 转换回协议格式。
#[async_trait::async_trait]
pub trait ProtocolAdapter: Send + Sync {
    /// 协议类型
    fn protocol_type(&self) -> ProtocolType;

    /// 解析请求为统一格式
    async fn parse_request(&self, raw_request: &[u8], metadata: &Value) -> ArchResult<ApiRequest>;

    /// 格式化响应为协议格式
    async fn format_response(&self, response: ApiResponse) -> ArchResult<Vec<u8>>;

    /// 格式化错误响应
    async fn format_error(&self, request_id: &str, error: &ArchError) -> Vec<u8> {
        let response = ApiResponse::error(request_id, error.code(), &error.to_string());
        self.format_response(response).await.unwrap_or_default()
    }
}

/// REST 协议适配器
pub struct RestProtocolAdapter;

impl RestProtocolAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for RestProtocolAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::Rest
    }

    async fn parse_request(&self, raw_request: &[u8], metadata: &Value) -> ArchResult<ApiRequest> {
        let payload: Value = if raw_request.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(raw_request)
                .map_err(|e| ArchError::ProtocolError(format!("invalid JSON: {}", e)))?
        };

        let resource_type = metadata
            .get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let operation = metadata
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("get")
            .to_string();

        let mut request = ApiRequest::new(&resource_type, &operation, payload);
        request.protocol = ProtocolType::Rest;

        // 提取元数据
        if let Some(meta_obj) = metadata.as_object() {
            for (k, v) in meta_obj {
                if let Some(s) = v.as_str() {
                    request.metadata.insert(k.clone(), s.to_string());
                }
            }
        }

        Ok(request)
    }

    async fn format_response(&self, response: ApiResponse) -> ArchResult<Vec<u8>> {
        let json = serde_json::to_vec(&response)
            .map_err(|e| ArchError::InternalError(format!("serialization error: {}", e)))?;
        Ok(json)
    }
}

/// GraphQL 协议适配器
pub struct GraphQLProtocolAdapter;

impl GraphQLProtocolAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for GraphQLProtocolAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::GraphQL
    }

    async fn parse_request(&self, raw_request: &[u8], metadata: &Value) -> ArchResult<ApiRequest> {
        let gql_req: Value = serde_json::from_slice(raw_request)
            .map_err(|e| ArchError::ProtocolError(format!("invalid GraphQL request: {}", e)))?;

        let resource_type = metadata
            .get("resource_type")
            .and_then(|v| v.as_str())
            .unwrap_or("graphql")
            .to_string();

        let operation = gql_req
            .get("operationName")
            .and_then(|v| v.as_str())
            .unwrap_or("query")
            .to_string();

        let mut request = ApiRequest::new(&resource_type, &operation, gql_req);
        request.protocol = ProtocolType::GraphQL;

        Ok(request)
    }

    async fn format_response(&self, response: ApiResponse) -> ArchResult<Vec<u8>> {
        // GraphQL 响应格式：{ data, errors }
        let gql_response = if response.status == crate::types::ApiStatus::Success {
            serde_json::json!({
                "data": response.data,
                "extensions": {
                    "request_id": response.request_id,
                    "timestamp": response.timestamp
                }
            })
        } else {
            serde_json::json!({
                "errors": [{
                    "message": response.error_message.unwrap_or_default(),
                    "extensions": {
                        "code": response.error_code.unwrap_or_default(),
                        "request_id": response.request_id
                    }
                }],
                "data": null
            })
        };

        serde_json::to_vec(&gql_response)
            .map_err(|e| ArchError::InternalError(format!("serialization error: {}", e)))
    }
}

/// WebSocket 协议适配器
pub struct WebSocketProtocolAdapter;

impl WebSocketProtocolAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ProtocolAdapter for WebSocketProtocolAdapter {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::WebSocket
    }

    async fn parse_request(&self, raw_request: &[u8], metadata: &Value) -> ArchResult<ApiRequest> {
        let msg: Value = serde_json::from_slice(raw_request)
            .map_err(|e| ArchError::ProtocolError(format!("invalid WS message: {}", e)))?;

        let resource_type = msg
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let operation = msg
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("message")
            .to_string();

        let payload = msg.get("data").cloned().unwrap_or(Value::Null);

        let mut request = ApiRequest::new(&resource_type, &operation, payload);
        request.protocol = ProtocolType::WebSocket;

        // 传递连接 ID
        if let Some(conn_id) = metadata.get("connection_id").and_then(|v| v.as_str()) {
            request.metadata.insert("connection_id".to_string(), conn_id.to_string());
        }

        Ok(request)
    }

    async fn format_response(&self, response: ApiResponse) -> ArchResult<Vec<u8>> {
        let ws_msg = serde_json::json!({
            "type": "response",
            "request_id": response.request_id,
            "status": response.status.as_str(),
            "data": response.data,
            "error": response.error_message,
            "timestamp": response.timestamp
        });

        serde_json::to_vec(&ws_msg)
            .map_err(|e| ArchError::InternalError(format!("serialization error: {}", e)))
    }
}

/// 协议注册中心
pub struct ProtocolRegistry {
    adapters: std::collections::HashMap<ProtocolType, Box<dyn ProtocolAdapter>>,
}

impl ProtocolRegistry {
    /// 创建协议注册中心并注册内置适配器
    pub fn new() -> Self {
        let mut registry = Self {
            adapters: std::collections::HashMap::new(),
        };
        registry.register(Box::new(RestProtocolAdapter::new()));
        registry.register(Box::new(GraphQLProtocolAdapter::new()));
        registry.register(Box::new(WebSocketProtocolAdapter::new()));
        registry
    }

    /// 注册协议适配器
    pub fn register(&mut self, adapter: Box<dyn ProtocolAdapter>) {
        self.adapters.insert(adapter.protocol_type(), adapter);
    }

    /// 获取协议适配器
    pub fn get(&self, protocol: ProtocolType) -> Option<&dyn ProtocolAdapter> {
        self.adapters.get(&protocol).map(|a| a.as_ref())
    }

    /// 检查协议是否支持
    pub fn supports(&self, protocol: ProtocolType) -> bool {
        self.adapters.contains_key(&protocol)
    }

    /// 列出所有支持的协议
    pub fn list_protocols(&self) -> Vec<ProtocolType> {
        self.adapters.keys().cloned().collect()
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rest_adapter() {
        let adapter = RestProtocolAdapter::new();
        assert_eq!(adapter.protocol_type(), ProtocolType::Rest);

        let body = json!({ "name": "test", "value": 42 }).to_string();
        let metadata = json!({ "resource_type": "user", "operation": "create" });

        let request = adapter
            .parse_request(body.as_bytes(), &metadata)
            .await
            .unwrap();

        assert_eq!(request.resource_type, "user");
        assert_eq!(request.operation, "create");
        assert_eq!(request.payload["name"], "test");

        let response = ApiResponse::success(&request.request_id, json!({ "id": "123" }));
        let bytes = adapter.format_response(response).await.unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["status"], "success");
    }

    #[tokio::test]
    async fn test_graphql_adapter() {
        let adapter = GraphQLProtocolAdapter::new();
        assert_eq!(adapter.protocol_type(), ProtocolType::GraphQL);

        let body = json!({
            "query": "{ user(id: 1) { name } }",
            "operationName": "GetUser"
        })
        .to_string();

        let request = adapter
            .parse_request(body.as_bytes(), &json!({}))
            .await
            .unwrap();

        assert_eq!(request.operation, "GetUser");
    }

    #[tokio::test]
    async fn test_websocket_adapter() {
        let adapter = WebSocketProtocolAdapter::new();
        assert_eq!(adapter.protocol_type(), ProtocolType::WebSocket);

        let body = json!({
            "type": "chat",
            "action": "send",
            "data": { "message": "hello" }
        })
        .to_string();

        let request = adapter
            .parse_request(body.as_bytes(), &json!({ "connection_id": "conn123" }))
            .await
            .unwrap();

        assert_eq!(request.resource_type, "chat");
        assert_eq!(request.operation, "send");
        assert_eq!(request.metadata.get("connection_id").unwrap(), "conn123");
    }

    #[test]
    fn test_protocol_registry() {
        let registry = ProtocolRegistry::new();

        assert!(registry.supports(ProtocolType::Rest));
        assert!(registry.supports(ProtocolType::GraphQL));
        assert!(registry.supports(ProtocolType::WebSocket));
        assert!(!registry.supports(ProtocolType::Grpc));

        assert_eq!(registry.list_protocols().len(), 3);
    }

    #[test]
    fn test_arch_error_codes() {
        let err = ArchError::NotFound("resource".to_string());
        assert_eq!(err.code(), "NOT_FOUND");
        assert_eq!(err.http_status(), 404);

        let err = ArchError::PermissionDenied("no access".to_string());
        assert_eq!(err.code(), "PERMISSION_DENIED");
        assert_eq!(err.http_status(), 403);

        let err = ArchError::RateLimited {
            limit: 100,
            retry_after: Some(60),
        };
        assert_eq!(err.code(), "RATE_LIMITED");
        assert_eq!(err.http_status(), 429);
    }
}
