//! 统一协议路由器 — Protocol Router
//!
//! 根据协议类型和路径将请求路由到对应的处理器。

use crate::protocol::traits::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// 路由结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingResult {
    /// 是否匹配成功
    pub matched: bool,
    /// 匹配的协议类型
    pub protocol: Option<ProtocolType>,
    /// 匹配的路径
    pub matched_path: Option<String>,
    /// 匹配的处理器名称
    pub handler_name: Option<String>,
    /// 重定向路径（如有）
    pub redirect_path: Option<String>,
    /// 不匹配原因
    pub reason: Option<String>,
}

impl RoutingResult {
    pub fn matched(protocol: ProtocolType, path: impl Into<String>, handler: impl Into<String>) -> Self {
        Self {
            matched: true,
            protocol: Some(protocol),
            matched_path: Some(path.into()),
            handler_name: Some(handler.into()),
            redirect_path: None,
            reason: None,
        }
    }

    pub fn not_found(reason: impl Into<String>) -> Self {
        Self {
            matched: false,
            protocol: None,
            matched_path: None,
            handler_name: None,
            redirect_path: None,
            reason: Some(reason.into()),
        }
    }
}

/// 路由规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRule {
    /// 规则ID
    pub rule_id: String,
    /// 协议类型
    pub protocol: ProtocolType,
    /// 路径前缀（如 /api/v1, /graphql）
    pub path_prefix: String,
    /// 目标处理器名称
    pub handler_name: String,
    /// 重写路径（可选，将匹配的前缀替换为此路径）
    #[serde(default)]
    pub rewrite_path: Option<String>,
    /// 优先级（数字越小越高）
    #[serde(default = "default_priority")]
    pub priority: u32,
    /// 是否启用
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_priority() -> u32 { 100 }
fn default_true() -> bool { true }

/// 统一协议路由器
pub struct ProtocolRouter {
    /// 路由规则（按优先级排序）
    rules: RwLock<Vec<RouteRule>>,
    /// 协议处理器注册表
    handlers: RwLock<HashMap<String, Arc<dyn ProtocolHandler>>>,
}

impl ProtocolRouter {
    pub fn new() -> Self {
        Self {
            rules: RwLock::new(Vec::new()),
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// 添加路由规则
    pub fn add_route(&self, rule: RouteRule) {
        let mut rules = self.rules.write();
        rules.push(rule);
        rules.sort_by_key(|r| r.priority);
    }

    /// 移除路由规则
    pub fn remove_route(&self, rule_id: &str) -> bool {
        let mut rules = self.rules.write();
        let len = rules.len();
        rules.retain(|r| r.rule_id != rule_id);
        rules.len() != len
    }

    /// 注册协议处理器
    pub fn register_handler(&self, name: impl Into<String>, handler: Arc<dyn ProtocolHandler>) {
        let name = name.into();
        tracing::info!("register protocol handler: {} ({})", name, handler.protocol_type().as_str());
        self.handlers.write().insert(name, handler);
    }

    /// 获取处理器
    pub fn get_handler(&self, name: &str) -> Option<Arc<dyn ProtocolHandler>> {
        self.handlers.read().get(name).cloned()
    }

    /// 路由请求
    pub fn route(&self, request: &ProtocolRequest) -> RoutingResult {
        let rules = self.rules.read();
        for rule in rules.iter() {
            if !rule.enabled { continue; }
            if rule.protocol != request.protocol { continue; }
            if request.path.starts_with(&rule.path_prefix) {
                // 检查处理器是否存在
                if self.handlers.read().contains_key(&rule.handler_name) {
                    return RoutingResult::matched(
                        rule.protocol,
                        rule.rewrite_path.clone().unwrap_or_else(|| request.path.clone()),
                        &rule.handler_name,
                    );
                } else {
                    return RoutingResult::not_found(format!("handler '{}' not registered", rule.handler_name));
                }
            }
        }
        RoutingResult::not_found(format!("no route matched for {} {}", request.protocol.as_str(), request.path))
    }

    /// 处理请求（路由 + 执行）
    pub async fn handle(&self, request: ProtocolRequest) -> ProtocolResponse {
        let routing = self.route(&request);
        if !routing.matched {
            return ProtocolResponse::error(
                request.request_id.clone(),
                404,
                routing.reason.unwrap_or_else(|| "not found".into()),
            );
        }

        let handler_name = routing.handler_name.unwrap();
        let handler = match self.get_handler(&handler_name) {
            Some(h) => h,
            None => return ProtocolResponse::error(request.request_id.clone(), 500, "handler not found"),
        };

        // 重写路径
        let mut request = request;
        if let Some(rewrite) = routing.matched_path {
            if !rewrite.is_empty() {
                request.path = rewrite;
            }
        }

        handler.handle(request).await
    }

    /// 列出所有路由规则
    pub fn list_routes(&self) -> Vec<RouteRule> {
        self.rules.read().clone()
    }

    /// 列出所有处理器
    pub fn list_handlers(&self) -> Vec<String> {
        self.handlers.read().keys().cloned().collect()
    }

    /// 路由规则数量
    pub fn route_count(&self) -> usize { self.rules.read().len() }

    /// 处理器数量
    pub fn handler_count(&self) -> usize { self.handlers.read().len() }
}

impl Default for ProtocolRouter {
    fn default() -> Self { Self::new() }
}
