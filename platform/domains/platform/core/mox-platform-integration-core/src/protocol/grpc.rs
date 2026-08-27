//! gRPC服务注册 — gRPC Service Registry

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// gRPC服务描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcService {
    /// 服务全名（如 mox.platform.v1.UserService）
    pub full_name: String,
    /// 服务包名
    pub package: String,
    /// 服务版本
    pub version: String,
    /// 方法列表
    pub methods: Vec<GrpcMethod>,
    /// 服务端点（host:port）
    pub endpoint: String,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 元数据
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

fn default_true() -> bool { true }

/// gRPC方法描述
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcMethod {
    /// 方法名
    pub name: String,
    /// 请求类型（Protobuf消息名）
    pub request_type: String,
    /// 响应类型（Protobuf消息名）
    pub response_type: String,
    /// 是否流式响应
    #[serde(default)]
    pub server_streaming: bool,
    /// 是否流式请求
    #[serde(default)]
    pub client_streaming: bool,
}

/// gRPC服务注册表
pub struct GrpcServiceRegistry {
    services: RwLock<HashMap<String, Arc<GrpcService>>>,
}

impl GrpcServiceRegistry {
    pub fn new() -> Self {
        Self { services: RwLock::new(HashMap::new()) }
    }

    /// 注册服务
    pub fn register(&self, service: GrpcService) {
        let name = service.full_name.clone();
        tracing::info!("register gRPC service: {} at {}", name, service.endpoint);
        self.services.write().insert(name, Arc::new(service));
    }

    /// 注销服务
    pub fn unregister(&self, full_name: &str) -> Option<Arc<GrpcService>> {
        self.services.write().remove(full_name)
    }

    /// 获取服务
    pub fn get(&self, full_name: &str) -> Option<Arc<GrpcService>> {
        self.services.read().get(full_name).cloned()
    }

    /// 列出所有服务
    pub fn list(&self) -> Vec<Arc<GrpcService>> {
        self.services.read().values().cloned().collect()
    }

    /// 按包名筛选
    pub fn list_by_package(&self, package: &str) -> Vec<Arc<GrpcService>> {
        self.services.read()
            .values()
            .filter(|s| s.package == package)
            .cloned()
            .collect()
    }

    /// 检查服务是否存在
    pub fn contains(&self, full_name: &str) -> bool {
        self.services.read().contains_key(full_name)
    }

    /// 服务数量
    pub fn len(&self) -> usize { self.services.read().len() }
    pub fn is_empty(&self) -> bool { self.services.read().is_empty() }

    /// 生成服务发现响应（用于gRPC reflection）
    pub fn reflection_response(&self) -> serde_json::Value {
        let services: Vec<serde_json::Value> = self.list().iter()
            .map(|s| serde_json::json!({
                "name": s.full_name,
                "methods": s.methods.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
            }))
            .collect();
        serde_json::json!({ "services": services })
    }
}

impl Default for GrpcServiceRegistry {
    fn default() -> Self { Self::new() }
}
