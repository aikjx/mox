//! 插件互通消息总线
//!
//! 实现发布-订阅模式的插件间通信，支持：
//! - 点对点消息
//! - 主题广播
//! - 请求-响应模式
//! - 事件驱动协作
//! - 消息路由与过滤

use super::types::*;
use chrono::Utc;
use operator_core::{OperatorError, Result};
use std::collections::HashMap;
use tracing;
use uuid::Uuid;

/// 插件消息总线 - 插件互通核心
pub struct PluginBus {
    /// 已注册插件
    plugins: HashMap<String, PluginInfo>,
    /// 主题订阅: topic -> Vec<Subscription>
    subscriptions: HashMap<String, Vec<SubscriptionEntry>>,
    /// 消息历史（环形记录最近 500 条，供可观测性查询）
    message_log: std::sync::Mutex<Vec<PluginMessage>>,
    /// 内置处理器
    builtin_handlers: HashMap<String, Box<dyn MessageHandler + Send + Sync>>,
}

/// 订阅条目
#[derive(Clone)]
struct SubscriptionEntry {
    plugin_id: String,
    filter: Option<MessageFilter>,
}

/// 消息过滤器（公开 API：`PluginBus::subscribe` 以 `Option<MessageFilter>` 暴露，故需 `pub`）
#[derive(Clone)]
pub enum MessageFilter {
    /// 仅接收来自特定插件的消息
    FromPlugin(String),
    /// 自定义谓词
    Custom(fn(&PluginMessage) -> bool),
}

/// 消息处理器trait
trait MessageHandler: Send + Sync {
    fn handle(&self, msg: &PluginMessage) -> Result<Option<PluginMessage>>;
}

/// 内置AI对话处理器
struct ConversationHandler;

impl MessageHandler for ConversationHandler {
    fn handle(&self, msg: &PluginMessage) -> Result<Option<PluginMessage>> {
        let response_text = format!(
            "[AI-Core] 收到来自{}的消息: topic={}, payload={:?}",
            msg.source_plugin, msg.topic, msg.payload
        );
        Ok(Some(
            PluginMessage::new(
                "ai-core",
                "response",
                serde_json::json!({ "reply": response_text }),
            )
            .with_correlation(&msg.id),
        ))
    }
}

/// 内置算子执行处理器
struct OperatorHandler;

impl MessageHandler for OperatorHandler {
    fn handle(&self, msg: &PluginMessage) -> Result<Option<PluginMessage>> {
        Ok(Some(
            PluginMessage::new(
                "operator-executor",
                "operator-result",
                serde_json::json!({
                    "status": "dispatched",
                    "original_topic": msg.topic,
                    "message": "算子执行请求已派发至执行引擎"
                }),
            )
            .with_correlation(&msg.id),
        ))
    }
}

/// 内置资源查询处理器
struct ResourceHandler;

impl MessageHandler for ResourceHandler {
    fn handle(&self, msg: &PluginMessage) -> Result<Option<PluginMessage>> {
        Ok(Some(
            PluginMessage::new(
                "resource-manager",
                "resource-status",
                serde_json::json!({
                    "cpu_available": true,
                    "memory_available": true,
                    "workflow_slots": 32,
                    "message": "资源状态正常"
                }),
            )
            .with_correlation(&msg.id),
        ))
    }
}

impl PluginBus {
    pub fn new() -> Self {
        let mut builtin_handlers: HashMap<String, Box<dyn MessageHandler + Send + Sync>> =
            HashMap::new();
        builtin_handlers.insert("ai.converse".to_string(), Box::new(ConversationHandler));
        builtin_handlers.insert("operator.execute".to_string(), Box::new(OperatorHandler));
        builtin_handlers.insert("resource.query".to_string(), Box::new(ResourceHandler));

        // 注册内置插件
        let mut plugins = HashMap::new();
        plugins.insert(
            "ai-core".to_string(),
            PluginInfo {
                id: "ai-core".to_string(),
                name: "AI核心对话引擎".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Builtin,
                capabilities: vec![
                    "conversation".to_string(),
                    "intent_recognition".to_string(),
                    "recommendation".to_string(),
                ],
                input_topics: vec!["ai.converse".to_string(), "ai.analyze".to_string()],
                output_topics: vec!["ai.response".to_string()],
                status: PluginStatus::Active,
                metadata: HashMap::new(),
            },
        );
        plugins.insert(
            "operator-executor".to_string(),
            PluginInfo {
                id: "operator-executor".to_string(),
                name: "算子执行引擎".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Builtin,
                capabilities: vec!["execute".to_string(), "workflow".to_string()],
                input_topics: vec!["operator.execute".to_string()],
                output_topics: vec!["operator.result".to_string()],
                status: PluginStatus::Active,
                metadata: HashMap::new(),
            },
        );
        plugins.insert(
            "resource-manager".to_string(),
            PluginInfo {
                id: "resource-manager".to_string(),
                name: "资源管理器".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Builtin,
                capabilities: vec!["allocate".to_string(), "monitor".to_string()],
                input_topics: vec!["resource.query".to_string()],
                output_topics: vec!["resource.status".to_string()],
                status: PluginStatus::Active,
                metadata: HashMap::new(),
            },
        );
        plugins.insert(
            "workflow-engine".to_string(),
            PluginInfo {
                id: "workflow-engine".to_string(),
                name: "工作流引擎".to_string(),
                version: "1.0.0".to_string(),
                plugin_type: PluginType::Builtin,
                capabilities: vec!["orchestrate".to_string(), "automate".to_string()],
                input_topics: vec!["workflow.start".to_string()],
                output_topics: vec!["workflow.status".to_string()],
                status: PluginStatus::Active,
                metadata: HashMap::new(),
            },
        );

        Self {
            plugins,
            subscriptions: HashMap::new(),
            message_log: std::sync::Mutex::new(Vec::new()),
            builtin_handlers,
        }
    }

    /// 注册插件到总线
    pub fn register(&mut self, plugin: PluginInfo) -> Result<()> {
        tracing::info!(
            "插件注册到消息总线: {} (type={:?})",
            plugin.id,
            plugin.plugin_type
        );

        // 自动订阅插件声明的输入主题
        for topic in &plugin.input_topics {
            self.subscribe(&plugin.id, topic, None);
        }

        self.plugins.insert(plugin.id.clone(), plugin);
        Ok(())
    }

    /// 注销插件
    pub fn unregister(&mut self, plugin_id: &str) -> bool {
        tracing::info!("插件从消息总线注销: {}", plugin_id);
        // 移除该插件的所有订阅
        for subs in self.subscriptions.values_mut() {
            subs.retain(|s| s.plugin_id != plugin_id);
        }
        self.plugins.remove(plugin_id).is_some()
    }

    /// 订阅主题
    pub fn subscribe(&mut self, plugin_id: &str, topic: &str, filter: Option<MessageFilter>) {
        self.subscriptions
            .entry(topic.to_string())
            .or_default()
            .push(SubscriptionEntry {
                plugin_id: plugin_id.to_string(),
                filter,
            });
        tracing::debug!("插件 {} 订阅主题 {}", plugin_id, topic);
    }

    /// 取消订阅
    pub fn unsubscribe(&mut self, plugin_id: &str, topic: &str) {
        if let Some(subs) = self.subscriptions.get_mut(topic) {
            subs.retain(|s| s.plugin_id != plugin_id);
        }
    }

    /// 路由消息 - 核心消息分发
    pub async fn route_message(&self, msg: PluginMessage) -> Result<Option<PluginMessage>> {
        tracing::debug!(
            "路由消息: {} -> topic={}, target={:?}",
            msg.source_plugin,
            msg.topic,
            msg.target_plugin
        );

        // 记录消息（环形保留最近 500 条，供可观测性查询）
        {
            let mut log = self.message_log.lock().unwrap();
            log.push(msg.clone());
            let overflow = log.len().saturating_sub(500);
            if overflow > 0 {
                log.drain(0..overflow);
            }
        }

        // 1. 如果有指定目标插件，直接路由
        if let Some(ref target) = msg.target_plugin {
            return self.deliver_to_plugin(&msg, target).await;
        }

        // 2. 检查内置处理器
        if let Some(handler) = self.builtin_handlers.get(&msg.topic) {
            if let Some(response) = handler.handle(&msg)? {
                if msg.response_required {
                    return Ok(Some(response));
                }
            }
        }

        // 3. 主题广播 - 投递到所有订阅者
        let mut any_response = None;
        if let Some(subs) = self.subscriptions.get(&msg.topic) {
            for sub in subs {
                // 应用过滤器
                if !self.matches_filter(&msg, sub) {
                    continue;
                }

                if sub.plugin_id != msg.source_plugin {
                    match self.deliver_to_plugin(&msg, &sub.plugin_id).await {
                        Ok(Some(resp)) if msg.response_required => {
                            any_response = Some(resp);
                        }
                        _ => {}
                    }
                }
            }
        }

        // 4. 通配符订阅 (topic.*)
        let prefix = msg.topic.split('.').next().unwrap_or("");
        let wildcard_topic = format!("{}.*", prefix);
        if let Some(subs) = self.subscriptions.get(&wildcard_topic) {
            for sub in subs {
                if sub.plugin_id != msg.source_plugin {
                    let _ = self.deliver_to_plugin(&msg, &sub.plugin_id).await;
                }
            }
        }

        Ok(any_response)
    }

    /// 投递消息到指定插件
    async fn deliver_to_plugin(
        &self,
        msg: &PluginMessage,
        plugin_id: &str,
    ) -> Result<Option<PluginMessage>> {
        let plugin = self.plugins.get(plugin_id).ok_or_else(|| {
            OperatorError::Other(anyhow::anyhow!("目标插件不存在: {}", plugin_id))
        })?;

        if plugin.status != PluginStatus::Active {
            tracing::warn!("插件 {} 未激活，无法投递消息", plugin_id);
            return Ok(None);
        }

        tracing::trace!("消息投递到插件 {}: topic={}", plugin_id, msg.topic);

        // 对于内置插件，直接处理
        if plugin.plugin_type == PluginType::Builtin {
            if let Some(handler) = self.builtin_handlers.get(&msg.topic) {
                return handler.handle(msg);
            }
            // 内置插件的默认响应
            return Ok(Some(
                PluginMessage::new(
                    plugin_id,
                    "ack",
                    serde_json::json!({
                        "status": "received",
                        "from": msg.source_plugin,
                        "topic": msg.topic
                    }),
                )
                .with_correlation(&msg.id),
            ));
        }

        // WASM/外部插件：在完整实现中会通过WASM调用或HTTP/gRPC投递
        // 这里返回接收确认
        Ok(Some(
            PluginMessage::new(
                plugin_id,
                "ack",
                serde_json::json!({
                    "status": "delivered",
                    "plugin_type": format!("{:?}", plugin.plugin_type)
                }),
            )
            .with_correlation(&msg.id),
        ))
    }

    /// 检查消息是否匹配过滤器
    fn matches_filter(&self, msg: &PluginMessage, sub: &SubscriptionEntry) -> bool {
        match &sub.filter {
            None => true,
            Some(MessageFilter::FromPlugin(expected)) => msg.source_plugin == *expected,
            Some(MessageFilter::Custom(pred)) => pred(msg),
        }
    }

    /// 发送请求并等待响应（请求-响应模式）
    pub async fn request_response(
        &self,
        target: &str,
        topic: &str,
        payload: serde_json::Value,
        _timeout_ms: u64,
    ) -> Result<PluginMessage> {
        let corr_id = Uuid::new_v4().to_string();

        let msg = PluginMessage::new("system", topic, payload)
            .to_target(target)
            .with_correlation(&corr_id)
            .need_response();

        if let Some(response) = self.route_message(msg).await? {
            Ok(response)
        } else {
            Err(OperatorError::Other(anyhow::anyhow!("请求超时或无响应")))
        }
    }

    /// 发布事件（广播模式）：记录消息并同步触发内置处理器响应，
    /// 主题订阅者投递由 `route_message`（异步）消费同一事件。
    pub fn publish(&self, source: &str, topic: &str, payload: serde_json::Value) -> Result<()> {
        tracing::info!("插件 {} 发布事件: {}", source, topic);
        let msg = PluginMessage::new(source, topic, payload);
        // 记录消息（环形保留最近 500 条）
        {
            let mut log = self.message_log.lock().unwrap();
            log.push(msg.clone());
            let overflow = log.len().saturating_sub(500);
            if overflow > 0 {
                log.drain(0..overflow);
            }
        }
        // 内置处理器同步响应（request/response 语义由 route_message 承载）
        if let Some(handler) = self.builtin_handlers.get(&msg.topic) {
            let _ = handler.handle(&msg)?;
        }
        Ok(())
    }

    /// 最近消息历史（可观测性：总线流量审计）
    pub fn message_history(&self, limit: usize) -> Vec<PluginMessage> {
        let log = self.message_log.lock().unwrap();
        log.iter().rev().take(limit).cloned().collect()
    }

    /// 调用插件方法
    pub async fn call_plugin(
        &self,
        plugin_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let topic = format!("plugin.{}.{}", plugin_id, method);
        let response = self
            .request_response(plugin_id, &topic, params, 5000)
            .await?;
        Ok(response.payload)
    }

    /// 获取已注册插件列表
    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins.values().cloned().collect()
    }

    /// 获取插件信息
    pub fn get_plugin(&self, plugin_id: &str) -> Option<&PluginInfo> {
        self.plugins.get(plugin_id)
    }

    /// 获取所有主题
    pub fn list_topics(&self) -> Vec<String> {
        self.subscriptions.keys().cloned().collect()
    }

    /// 获取主题订阅者
    pub fn get_subscribers(&self, topic: &str) -> Vec<String> {
        self.subscriptions
            .get(topic)
            .map(|subs| subs.iter().map(|s| s.plugin_id.clone()).collect())
            .unwrap_or_default()
    }

    /// 更新插件状态
    pub fn set_plugin_status(&mut self, plugin_id: &str, status: PluginStatus) -> Result<()> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("插件不存在: {}", plugin_id)))?;
        plugin.status = status;
        Ok(())
    }

    /// 获取系统互通拓扑
    pub fn get_topology(&self) -> PluginTopology {
        let mut connections = Vec::new();
        for (topic, subs) in &self.subscriptions {
            for sub in subs {
                // 查找发布此主题的插件
                for plugin in self.plugins.values() {
                    if plugin.output_topics.contains(topic)
                        || topic.starts_with(&format!("{}.", plugin.id))
                    {
                        connections.push(PluginConnection {
                            from: plugin.id.clone(),
                            to: sub.plugin_id.clone(),
                            topic: topic.clone(),
                        });
                    }
                }
            }
        }

        PluginTopology {
            plugins: self.plugins.values().cloned().collect(),
            connections,
            topics: self.list_topics(),
        }
    }

    /// 创建标准对话消息
    pub fn create_chat_message(session_id: &str, content: &str) -> PluginMessage {
        PluginMessage::new(
            "user",
            "ai.converse",
            serde_json::json!({
                "session_id": session_id,
                "content": content,
                "timestamp": Utc::now().to_rfc3339()
            }),
        )
        .need_response()
    }

    /// 创建工作流启动消息
    pub fn create_workflow_start(workflow_id: &str, params: serde_json::Value) -> PluginMessage {
        PluginMessage::new(
            "user",
            "workflow.start",
            serde_json::json!({
                "workflow_id": workflow_id,
                "parameters": params
            }),
        )
        .need_response()
    }

    /// 创建算子执行消息
    pub fn create_operator_execute(operator_id: &str, input: Vec<f64>) -> PluginMessage {
        PluginMessage::new(
            "user",
            "operator.execute",
            serde_json::json!({
                "operator_id": operator_id,
                "input": input
            }),
        )
        .need_response()
    }

    /// 创建资源查询消息
    pub fn create_resource_query() -> PluginMessage {
        PluginMessage::new("user", "resource.query", serde_json::json!({})).need_response()
    }
}

/// 插件拓扑结构
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginTopology {
    pub plugins: Vec<PluginInfo>,
    pub connections: Vec<PluginConnection>,
    pub topics: Vec<String>,
}

/// 插件连接
#[derive(Debug, Clone, serde::Serialize)]
pub struct PluginConnection {
    pub from: String,
    pub to: String,
    pub topic: String,
}

impl Default for PluginBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plugin(id: &str, status: PluginStatus, ptype: PluginType) -> PluginInfo {
        PluginInfo {
            id: id.to_string(),
            name: id.to_string(),
            version: "1.0.0".to_string(),
            plugin_type: ptype,
            status,
            capabilities: vec!["cap1".to_string()],
            input_topics: vec!["ai.converse".to_string()],
            output_topics: vec!["ai.response".to_string()],
            metadata: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_register_and_list_plugins() {
        let mut bus = PluginBus::new();
        // bus 预置若干内置插件，注册后应 >= 2
        let before = bus.list_plugins().len();
        bus.register(sample_plugin(
            "p1",
            PluginStatus::Active,
            PluginType::Builtin,
        ))
        .unwrap();
        bus.register(sample_plugin("p2", PluginStatus::Active, PluginType::Wasm))
            .unwrap();

        let plugins = bus.list_plugins();
        assert_eq!(plugins.len(), before + 2);
        assert!(bus.get_plugin("p1").is_some());
        assert!(bus.get_plugin("missing").is_none());
    }

    #[tokio::test]
    async fn test_route_to_target_plugin_returns_ack() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin("dst", PluginStatus::Active, PluginType::Wasm))
            .unwrap();

        let msg = PluginMessage::new("src", "test.topic", serde_json::json!({"x": 1}))
            .to_target("dst")
            .need_response();
        let resp = bus.route_message(msg).await.unwrap();
        assert!(resp.is_some());
        let r = resp.unwrap();
        assert_eq!(r.topic, "ack");
        assert!(r.correlation_id.is_some());
    }

    #[tokio::test]
    async fn test_route_to_paused_plugin_returns_none() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin("dst", PluginStatus::Paused, PluginType::Wasm))
            .unwrap();

        let msg = PluginMessage::new("src", "test.topic", serde_json::json!({"x": 1}))
            .to_target("dst")
            .need_response();
        let resp = bus.route_message(msg).await.unwrap();
        assert!(resp.is_none());
    }

    #[tokio::test]
    async fn test_route_to_missing_plugin_errors() {
        let bus = PluginBus::new();
        let msg = PluginMessage::new("src", "test.topic", serde_json::json!({"x": 1}))
            .to_target("nope")
            .need_response();
        let result = bus.route_message(msg).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_unregister_removes_plugin() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin(
            "p1",
            PluginStatus::Active,
            PluginType::Builtin,
        ))
        .unwrap();
        assert!(bus.get_plugin("p1").is_some());
        assert!(bus.unregister("p1"));
        assert!(bus.get_plugin("p1").is_none());
        assert!(!bus.unregister("p1"));
    }

    #[test]
    fn test_set_plugin_status() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin(
            "p1",
            PluginStatus::Active,
            PluginType::Builtin,
        ))
        .unwrap();
        bus.set_plugin_status("p1", PluginStatus::Paused).unwrap();
        assert_eq!(bus.get_plugin("p1").unwrap().status, PluginStatus::Paused);
        assert!(bus
            .set_plugin_status("ghost", PluginStatus::Active)
            .is_err());
    }

    #[test]
    fn test_get_topology_lists_plugins_and_topics() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin(
            "p1",
            PluginStatus::Active,
            PluginType::Builtin,
        ))
        .unwrap();
        let topo = bus.get_topology();
        assert!(topo.plugins.iter().any(|p| p.id == "p1"));
    }

    #[test]
    fn test_create_helper_messages() {
        let chat = PluginBus::create_chat_message("s1", "hi");
        assert_eq!(chat.topic, "ai.converse");
        assert_eq!(chat.target_plugin, None);
        assert!(chat.response_required);

        let wf = PluginBus::create_workflow_start("wf-1", serde_json::json!({}));
        assert_eq!(wf.topic, "workflow.start");

        let op = PluginBus::create_operator_execute("linear", vec![1.0, 2.0]);
        assert_eq!(op.topic, "operator.execute");

        let _q = PluginBus::create_resource_query();
    }

    #[tokio::test]
    async fn test_request_response_success() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin("svc", PluginStatus::Active, PluginType::Wasm))
            .unwrap();
        let resp = bus
            .request_response("svc", "svc.ping", serde_json::json!({"a":1}), 1000)
            .await
            .unwrap();
        assert_eq!(resp.topic, "ack");
    }

    #[tokio::test]
    async fn test_subscribe_and_route_broadcast() {
        let mut bus = PluginBus::new();
        bus.register(sample_plugin(
            "a",
            PluginStatus::Active,
            PluginType::Builtin,
        ))
        .unwrap();
        bus.register(sample_plugin("b", PluginStatus::Active, PluginType::Wasm))
            .unwrap();
        // a 订阅 ai.converse
        bus.subscribe("a", "ai.converse", None);
        let msg = PluginMessage::new("user", "ai.converse", serde_json::json!({"c":"hi"}));
        // 广播模式，无 target，b 不是订阅者所以不会投递；a 是订阅者但 deliver 给自身跳过
        // 这里验证不会 panic 且返回 None（无 response_required）
        let r = bus.route_message(msg).await.unwrap();
        assert!(r.is_none());
        assert!(bus
            .get_subscribers("ai.converse")
            .contains(&"a".to_string()));
    }
}
