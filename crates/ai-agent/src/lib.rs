//! # AI智能体模块 v3.2
//!
//! 实现八大核心能力：
//! 1. AI智能对话 - 支持真实LLM API对接 + 自然语言理解
//! 2. 算法分析与归一化 - 最强开发算法流程图生成与标准化
//! 3. 全资源管理 - 统一资源调度与监控
//! 4. 插件互通总线 - 跨插件消息路由与协作
//! 5. 业务流程驱动自动化 - BPMN风格工作流引擎
//! 6. 真实LLM大模型对接 - OpenAI兼容API
//! 7. 浏览器自动化引擎 - AI驱动网页操作自动化
//! 8. 流程图引擎 - 可视化AI流程编排与执行

pub mod conversation;
pub mod algorithm;
pub mod resource_manager;
pub mod plugin_bus;
pub mod workflow_engine;
pub mod types;
pub mod llm_client;
pub mod browser_automation;
pub mod flow_engine;

pub use conversation::*;
pub use algorithm::*;
pub use resource_manager::*;
pub use plugin_bus::*;
pub use workflow_engine::*;
pub use types::{
    MessageRole, ChatMessage, ChatSession, SessionContext, UserIntent, ChatResponse,
    SuggestedAction, ActionType, AlgorithmType, AlgorithmFlow, OptimizationSuggestion,
    OptimizationImpact, ComplexityAnalysis, ResourceType, ResourceAllocation,
    ResourceUsageStats, ResourcePanorama, PluginInfo, PluginType, PluginStatus,
    PluginMessage, MessageSubscription, BusinessWorkflow, WorkflowNode, WorkflowNodeType,
    WorkflowNodeConfig, MergeStrategy, NodePosition, WorkflowEdge, WorkflowInstance,
    WorkflowStatus, NodeExecutionRecord, WorkflowResult, WorkflowMetrics,
    WorkflowConnection, NodeStatus, WorkflowTemplate,
};
pub use llm_client::*;
pub use browser_automation::*;
pub use flow_engine::*;

use operator_core::{OperatorError, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// AI智能体主结构 - 统一系统大脑
pub struct AIAgent {
    /// 对话引擎
    conversation: Arc<RwLock<ConversationEngine>>,
    /// 算法分析器
    algorithm_analyzer: Arc<RwLock<AlgorithmAnalyzer>>,
    /// 资源管理器
    resource_manager: Arc<RwLock<ResourceManager>>,
    /// 插件消息总线
    plugin_bus: Arc<RwLock<PluginBus>>,
    /// 工作流引擎
    workflow_engine: Arc<RwLock<WorkflowEngine>>,
    /// 真实LLM客户端
    llm_client: Arc<RwLock<LLMClient>>,
    /// 浏览器自动化引擎
    browser: Arc<RwLock<BrowserAutomationEngine>>,
    /// 流程图引擎
    flow_engine: Arc<RwLock<FlowEngine>>,
}

impl AIAgent {
    pub fn new() -> Self {
        let mut flow_engine = FlowEngine::new();
        // 加载预置模板
        for template in create_default_templates() {
            let _ = flow_engine.create_flow(template);
        }
        Self {
            conversation: Arc::new(RwLock::new(ConversationEngine::new())),
            algorithm_analyzer: Arc::new(RwLock::new(AlgorithmAnalyzer::new())),
            resource_manager: Arc::new(RwLock::new(ResourceManager::new())),
            plugin_bus: Arc::new(RwLock::new(PluginBus::new())),
            workflow_engine: Arc::new(RwLock::new(WorkflowEngine::new())),
            llm_client: Arc::new(RwLock::new(LLMClient::new(LLMConfig::default()))),
            browser: Arc::new(RwLock::new(BrowserAutomationEngine::new())),
            flow_engine: Arc::new(RwLock::new(flow_engine)),
        }
    }

    /// 处理用户消息 - 智能对话入口（优先使用真实LLM，降级到规则引擎）
    pub async fn chat(&self, session_id: &str, message: &str) -> Result<ChatResponse> {
        // 检测是否需要浏览器自动化
        let needs_browser = self.detect_browser_intent(message);
        
        if needs_browser {
            return self.handle_browser_automation(message).await;
        }

        // 尝试使用真实LLM
        let llm = self.llm_client.read().await;
        if llm.is_enabled() {
            drop(llm);
            return self.chat_with_llm(session_id, message).await;
        }
        drop(llm);

        // 降级到内置规则引擎
        let conv = self.conversation.read().await;
        conv.process_message(session_id, message).await
    }

    fn detect_browser_intent(&self, message: &str) -> bool {
        let p = message.to_lowercase();
        p.contains("http://") || p.contains("https://") || 
        p.contains("www.") || p.contains(".com") || p.contains(".cn") ||
        p.contains("浏览器") || p.contains("打开网页") || p.contains("访问") ||
        p.contains("截图") && (p.contains("网页") || p.contains("页面") || p.contains("网站")) ||
        p.contains("搜索") && !p.contains("算子") && !p.contains("算法") ||
        p.contains("浏览") || p.contains("爬取") || p.contains("抓取")
    }

    async fn chat_with_llm(&self, session_id: &str, message: &str) -> Result<ChatResponse> {
        let mut conv = self.conversation.write().await;
        let history = conv.get_session_history(session_id);
        
        let mut messages: Vec<LLMChatMessage> = history.iter().map(|m| LLMChatMessage {
            role: match m.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                _ => "system".to_string(),
            },
            content: m.content.clone(),
        }).collect();
        messages.push(LLMChatMessage { role: "user".to_string(), content: message.to_string() });
        drop(conv);

        let llm = self.llm_client.read().await;
        match llm.chat(messages).await {
            Ok(response_text) => {
                drop(llm);
                let mut conv = self.conversation.write().await;
                let response = conv.add_assistant_message(session_id, &response_text);
                Ok(response)
            }
            Err(e) => {
                tracing::warn!("LLM调用失败，降级到规则引擎: {}", e);
                drop(llm);
                let conv = self.conversation.read().await;
                conv.process_message(session_id, message).await
            }
        }
    }

    async fn handle_browser_automation(&self, message: &str) -> Result<ChatResponse> {
        let (url, steps) = {
            let _browser = self.browser.read().await;
            BrowserAutomationEngine::parse_natural_language(message)
        };

        if steps.is_empty() {
            let conv = self.conversation.read().await;
            return conv.process_message("browser-session", "请提供要访问的URL或具体的浏览器操作指令").await;
        }

        let mut browser = self.browser.write().await;
        let result = browser.execute_custom_steps(steps, url).await;

        let mut response_text = String::new();
        match result {
            Ok(task_result) => {
                response_text.push_str("🌐 **浏览器自动化任务执行完成**\n\n");
                response_text.push_str(&format!("任务: {} | 状态: {}\n", 
                    task_result.task_name,
                    if task_result.success { "✅ 成功" } else { "❌ 失败" }));
                response_text.push_str(&format!("会话: {} | 耗时: {}ms\n", 
                    task_result.session_id, task_result.total_duration_ms));
                if !task_result.final_url.is_empty() {
                    response_text.push_str(&format!("最终URL: {}\n", task_result.final_url));
                }
                response_text.push_str("\n**执行步骤:**\n");
                for (i, step) in task_result.steps_results.iter().enumerate() {
                    let icon = if step.success { "✅" } else { "❌" };
                    response_text.push_str(&format!("{}. {} {} ({}ms)\n", i+1, icon, step.action_type, step.duration_ms));
                    if let Some(data) = &step.data {
                        if let Some(text) = data.get("text").and_then(|v| v.as_str()) {
                            response_text.push_str(&format!("   📄 提取: {}\n", text));
                        }
                        if let Some(title) = data.get("title").and_then(|v| v.as_str()) {
                            response_text.push_str(&format!("   📌 标题: {}\n", title));
                        }
                    }
                }
                if let Some(err) = task_result.error {
                    response_text.push_str(&format!("\n❌ 错误: {}\n", err));
                }
            }
            Err(e) => {
                response_text.push_str(&format!("❌ 浏览器自动化失败: {}\n", e));
            }
        }

        Ok(ChatResponse {
            message: ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                role: MessageRole::Assistant,
                content: response_text,
                timestamp: chrono::Utc::now(),
                metadata: HashMap::from([("type".into(), serde_json::json!("browser_automation"))]),
                referenced_operators: vec![],
            },
            actions: vec![],
            recommended_operators: vec![],
            suggestions: vec!["访问其他网站".to_string(), "截图网页".to_string(), "搜索内容".to_string()],
            workflow_suggestion: None,
        })
    }

    /// 配置LLM
    pub async fn configure_llm(&self, config: LLMConfig) {
        let mut llm = self.llm_client.write().await;
        llm.update_config(config);
    }

    pub async fn test_llm_connection(&self) -> Result<serde_json::Value> {
        let llm = self.llm_client.read().await;
        llm.test_connection().await.map_err(OperatorError::Other)
    }

    pub fn llm_client(&self) -> Arc<RwLock<LLMClient>> {
        self.llm_client.clone()
    }

    pub fn browser(&self) -> Arc<RwLock<BrowserAutomationEngine>> {
        self.browser.clone()
    }

    /// 分析算法并生成归一化流程图
    pub async fn analyze_algorithm(&self, algo_code: &str, algo_type: AlgorithmType) -> Result<AlgorithmFlow> {
        let analyzer = self.algorithm_analyzer.read().await;
        analyzer.analyze(algo_code, algo_type).await
    }

    /// 获取资源使用全景
    pub async fn get_resource_status(&self) -> Result<ResourcePanorama> {
        let rm = self.resource_manager.read().await;
        Ok(rm.get_panorama())
    }

    /// 注册插件到互通总线
    pub async fn register_plugin(&self, plugin: PluginInfo) -> Result<()> {
        let mut bus = self.plugin_bus.write().await;
        bus.register(plugin)
    }

    /// 发送插件间消息
    pub async fn send_plugin_message(&self, msg: PluginMessage) -> Result<Option<PluginMessage>> {
        let bus = self.plugin_bus.read().await;
        bus.route_message(msg).await
    }

    /// 执行业务流程
    pub async fn execute_workflow(&self, workflow: BusinessWorkflow) -> Result<WorkflowResult> {
        let mut engine = self.workflow_engine.write().await;
        engine.execute_business_workflow(workflow).await
    }

    pub fn conversation(&self) -> Arc<RwLock<ConversationEngine>> {
        self.conversation.clone()
    }

    pub fn algorithm_analyzer(&self) -> Arc<RwLock<AlgorithmAnalyzer>> {
        self.algorithm_analyzer.clone()
    }

    pub fn resource_manager(&self) -> Arc<RwLock<ResourceManager>> {
        self.resource_manager.clone()
    }

    pub fn plugin_bus(&self) -> Arc<RwLock<PluginBus>> {
        self.plugin_bus.clone()
    }

    pub fn workflow_engine(&self) -> Arc<RwLock<WorkflowEngine>> {
        self.workflow_engine.clone()
    }

    pub fn flow_engine(&self) -> Arc<RwLock<FlowEngine>> {
        self.flow_engine.clone()
    }

    // ============ 流程图引擎API ============

    /// 创建流程图
    pub async fn create_flow(&self, flow: FlowDefinition) -> Result<FlowDefinition> {
        let mut engine = self.flow_engine.write().await;
        engine.create_flow(flow).map_err(|e| OperatorError::Other(anyhow::anyhow!(e.to_string())))
    }

    /// 获取流程图
    pub async fn get_flow(&self, id: &str) -> Result<Option<FlowDefinition>> {
        let engine = self.flow_engine.read().await;
        Ok(engine.get_flow(id).cloned())
    }

    /// 列出所有流程图
    pub async fn list_flows(&self) -> Result<Vec<FlowDefinition>> {
        let engine = self.flow_engine.read().await;
        Ok(engine.list_flows().into_iter().cloned().collect())
    }

    /// 删除流程图
    pub async fn delete_flow(&self, id: &str) -> Result<bool> {
        let mut engine = self.flow_engine.write().await;
        Ok(engine.delete_flow(id))
    }

    /// 验证流程图
    pub fn validate_flow(flow: &FlowDefinition) -> Result<()> {
        FlowEngine::validate_flow(flow).map_err(|e| OperatorError::Other(anyhow::anyhow!(e.to_string())))
    }

    /// 执行流程图
    pub async fn execute_flow(
        &self,
        flow_id: &str,
        input: HashMap<String, serde_json::Value>,
    ) -> Result<FlowExecutionResult> {
        // 先获取流程图定义
        let flow_def = {
            let engine = self.flow_engine.read().await;
            engine.get_flow(flow_id).cloned()
        }.ok_or_else(|| OperatorError::Other(anyhow::anyhow!("流程图不存在: {}", flow_id)))?;

        // 执行节点（支持真实的LLM、浏览器、HTTP请求）
        let mut results = Vec::new();
        let mut variables = flow_def.variables.clone();
        variables.extend(input.clone());

        let start_node = flow_def.nodes.iter()
            .find(|n| matches!(n.node_type, NodeType::Start))
            .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("缺少Start节点")))?;

        let mut current_node_id = start_node.id.clone();
        let mut max_steps = 1000;
        let start_time = std::time::Instant::now();

        loop {
            if max_steps == 0 {
                return Err(OperatorError::Other(anyhow::anyhow!("执行步数超限")));
            }
            max_steps -= 1;

            let node = flow_def.nodes.iter()
                .find(|n| n.id == current_node_id)
                .ok_or_else(|| OperatorError::Other(anyhow::anyhow!("节点不存在: {}", current_node_id)))?
                .clone();

            let result = self.execute_flow_node(&node, &variables).await;
            let had_error = result.error.is_some();

            // 保存输出到变量
            if let Some(ref out) = result.output {
                variables.insert(format!("node_{}", node.id), out.clone());
                variables.insert("last_output".into(), out.clone());
            }
            results.push(result);

            if had_error {
                return Ok(FlowExecutionResult {
                    flow_id: flow_def.id.clone(),
                    flow_name: flow_def.name.clone(),
                    success: false,
                    node_results: results,
                    output: None,
                    variables,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: Some("节点执行失败".into()),
                });
            }

            if matches!(node.node_type, NodeType::End) {
                return Ok(FlowExecutionResult {
                    flow_id: flow_def.id.clone(),
                    flow_name: flow_def.name.clone(),
                    success: true,
                    node_results: results,
                    output: variables.get("last_output").cloned(),
                    variables,
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    error: None,
                });
            }

            // 条件节点
            if matches!(node.node_type, NodeType::Condition) {
                let condition = node.config.get("condition")
                    .and_then(|c| c.as_str())
                    .unwrap_or("true");
                let should_take_true = flow_engine::evaluate_condition(condition, &variables);
                let condition_match = if should_take_true { "true" } else { "false" };

                let next_edge = flow_def.edges.iter()
                    .find(|e| e.source == node.id &&
                        (e.condition.as_deref() == Some(condition_match) || e.condition.is_none()))
                    .or_else(|| flow_def.edges.iter()
                        .find(|e| e.source == node.id))
                    .cloned();

                if let Some(edge) = next_edge {
                    current_node_id = edge.target;
                } else {
                    break;
                }
                continue;
            }

            // 普通节点
            let next_edge = flow_def.edges.iter()
                .find(|e| e.source == node.id)
                .cloned();

            if let Some(edge) = next_edge {
                current_node_id = edge.target;
            } else {
                break;
            }
        }

        Ok(FlowExecutionResult {
            flow_id: flow_def.id.clone(),
            flow_name: flow_def.name.clone(),
            success: true,
            node_results: results,
            output: variables.get("last_output").cloned(),
            variables,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            error: None,
        })
    }

    async fn execute_flow_node(
        &self,
        node: &FlowNode,
        variables: &HashMap<String, serde_json::Value>,
    ) -> NodeExecutionResult {
        let start = std::time::Instant::now();
        let node_id = node.id.clone();
        let node_name = node.name.clone();
        let node_type_str = format!("{:?}", node.node_type);

        match &node.node_type {
            NodeType::LLM => {
                let prompt_template = node.config.get("prompt").and_then(|p| p.as_str()).unwrap_or("");
                let prompt = flow_engine::apply_template(prompt_template, variables);
                let model = node.config.get("model").and_then(|m| m.as_str())
                    .unwrap_or("gpt-3.5-turbo");

                let llm = self.llm_client.read().await;
                if llm.is_enabled() {
                    let messages = vec![LLMChatMessage {
                        role: "user".into(),
                        content: prompt.clone(),
                    }];
                    match llm.chat(messages).await {
                        Ok(response) => NodeExecutionResult {
                            node_id, node_name, node_type: "llm".into(),
                            status: "success".into(),
                            input: Some(serde_json::json!({"prompt": prompt, "model": model})),
                            output: Some(serde_json::json!({"response": response})),
                            error: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        },
                        Err(e) => NodeExecutionResult {
                            node_id, node_name, node_type: "llm".into(),
                            status: "error".into(),
                            input: Some(serde_json::json!({"prompt": prompt})),
                            output: None,
                            error: Some(e.to_string()),
                            duration_ms: start.elapsed().as_millis() as u64,
                        },
                    }
                } else {
                    // 模拟LLM响应
                    NodeExecutionResult {
                        node_id, node_name, node_type: "llm".into(),
                        status: "simulated".into(),
                        input: Some(serde_json::json!({"prompt": prompt})),
                        output: Some(serde_json::json!({"response": format!("[模拟LLM] 收到: {}", prompt)})),
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    }
                }
            }
            NodeType::Browser => {
                let url_template = node.config.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let url = flow_engine::apply_template(url_template, variables);
                let action = node.config.get("action").and_then(|a| a.as_str())
                    .unwrap_or("navigate");

                let mut browser = self.browser.write().await;
                let result = browser.execute_action("default", BrowserAction::Navigate { url: url.clone() }).await;

                match result {
                    Ok(page) => NodeExecutionResult {
                        node_id, node_name, node_type: "browser".into(),
                        status: "success".into(),
                        input: Some(serde_json::json!({"url": url, "action": action})),
                        output: Some(serde_json::json!({
                            "url": page.data.as_ref().and_then(|d| d.get("url")).and_then(|v| v.as_str()).unwrap_or(""),
                            "title": page.data.as_ref().and_then(|d| d.get("title")).and_then(|v| v.as_str()).unwrap_or(""),
                            "html_length": page.data.as_ref().and_then(|d| d.get("html")).and_then(|v| v.as_str()).map(|s| s.len()).unwrap_or(0),
                        })),
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                    Err(e) => NodeExecutionResult {
                        node_id, node_name, node_type: "browser".into(),
                        status: "error".into(),
                        input: Some(serde_json::json!({"url": url})),
                        output: None,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            NodeType::HttpRequest => {
                let url = node.config.get("url").and_then(|u| u.as_str()).unwrap_or("");
                let method = node.config.get("method").and_then(|m| m.as_str()).unwrap_or("GET");
                let body = node.config.get("body").and_then(|b| b.as_str());

                let client = reqwest::Client::new();
                let url_resolved = flow_engine::apply_template(url, variables);

                let response = match method.to_uppercase().as_str() {
                    "GET" => client.get(&url_resolved).send().await,
                    "POST" => {
                        let mut req = client.post(&url_resolved);
                        if let Some(b) = body {
                            req = req.body(flow_engine::apply_template(b, variables));
                        }
                        req.send().await
                    }
                    _ => {
                        return NodeExecutionResult {
                            node_id, node_name, node_type: "http_request".into(),
                            status: "error".into(),
                            input: Some(serde_json::json!({"url": url, "method": method})),
                            output: None,
                            error: Some(format!("不支持的HTTP方法: {}", method)),
                            duration_ms: start.elapsed().as_millis() as u64,
                        };
                    }
                };

                match response {
                    Ok(resp) => {
                        let status = resp.status();
                        let body_text = resp.text().await.unwrap_or_default();
                        NodeExecutionResult {
                            node_id, node_name, node_type: "http_request".into(),
                            status: "success".into(),
                            input: Some(serde_json::json!({"url": url, "method": method})),
                            output: Some(serde_json::json!({
                                "status": status.as_u16(),
                                "body": &body_text[..body_text.len().min(1000)],
                            })),
                            error: None,
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                    Err(e) => NodeExecutionResult {
                        node_id, node_name, node_type: "http_request".into(),
                        status: "error".into(),
                        input: Some(serde_json::json!({"url": url})),
                        output: None,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            _ => {
                // 其他节点类型由FlowEngine基础处理
                let mut temp_engine = FlowEngine::new();
                let temp_flow = FlowDefinition {
                    id: "temp".into(),
                    name: "temp".into(),
                    description: "".into(),
                    nodes: vec![node.clone()],
                    edges: vec![],
                    variables: variables.clone(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                let _ = temp_engine.create_flow(temp_flow);
                let results = temp_engine.execute_flow("temp", HashMap::new()).await;
                match results {
                    Ok(mut r) if !r.node_results.is_empty() => r.node_results.remove(0),
                    _ => NodeExecutionResult {
                        node_id, node_name, node_type: node_type_str,
                        status: "pending".into(),
                        input: None,
                        output: None,
                        error: Some("无法执行节点".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
        }
    }
}

impl Default for AIAgent {
    fn default() -> Self {
        Self::new()
    }
}
