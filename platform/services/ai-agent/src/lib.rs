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

pub const CRATE_ID: &str = "00374bdd-cc60-55bf-8970-a879afbfe443";
pub const ENGINE_NAME: &str = "xuanji::ai_agent";
pub const CRATE_META: xuanji_common_meta::CrateMeta = xuanji_common_meta::CrateMeta {
    id: CRATE_ID,
    name: env!("CARGO_PKG_NAME"),
    version: env!("CARGO_PKG_VERSION"),
    layer: xuanji_common_meta::AisLayer::L4Services,
    owner: "xuanji-core",
};

pub mod conversation;
pub mod algorithm;
pub mod resource_manager;
pub mod plugin_bus;
pub mod workflow_engine;
pub mod types;
pub mod llm_client;
pub mod browser_automation;
pub mod flow_engine;
pub mod knowledge;
pub mod requirement_compiler;
pub mod dialogue_graph;
pub mod provider;
pub mod engine;
mod util;

pub use conversation::*;
pub use requirement_compiler::*;
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
pub use dialogue_graph::*;

use crate::engine::{AgentRole, Engine, EngineContext, EngineConfig, MultiAgentOrchestrator};
use operator_core::{OperatorError, Result};
use graph_algorithms::KnowledgeGraph;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use xuanji_system::persistence_provider::SqlValue;

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
    /// 草莓多平台：需求编译器（对话→系统蓝图）
    requirement_compiler: Arc<RwLock<RequirementCompiler>>,
    /// 对话→知识图谱自动整理同步器（全自动）
    dialogue_graph: Arc<DialogueGraphSyncer>,
    /// AI Gateway：组件化 Provider 注册表 + 路由（fallback 链）
    router: Arc<provider::LlmRouter>,
    /// AI Agent 引擎：PERCEIVE→PLAN→ACT→OBSERVE→REFLECT→GENERATE→CONSOLIDATE
    engine: Arc<RwLock<engine::Engine>>,
    /// 多 Agent 编排器：支持子 Agent 创建、通信与并行/顺序执行
    multi_agent_orchestrator: Arc<RwLock<MultiAgentOrchestrator>>,
}

impl AIAgent {
    pub fn new() -> Self {
        let mut flow_engine = FlowEngine::new();
        // 加载预置模板
        for template in create_default_templates() {
            let _ = flow_engine.create_flow(template);
        }
        let llm_client = Arc::new(RwLock::new(LLMClient::new(LLMConfig::default())));
        let knowledge_graph = Arc::new(RwLock::new(KnowledgeGraph::new()));
        let dialogue_graph = {
            let dg = DialogueGraphSyncer::new(
                "operator_dialogue.db",
                knowledge_graph.clone(),
                llm_client.clone(),
            );
            match dg {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    tracing::warn!("对话图谱同步器初始化失败，降级为禁用的空同步器: {e}");
                    Arc::new(DialogueGraphSyncer::new_in_memory(
                        knowledge_graph.clone(),
                        llm_client.clone(),
                    ))
                }
            }
        };

        // —— AI Gateway 初始化：从环境变量注册所有可用 Provider，并构建 fallback 链 ——
        // 同步完成（基于 std::sync::RwLock），无需 async。
        let router = provider::LlmRouter::init_from_env();

        Self {
            conversation: Arc::new(RwLock::new(ConversationEngine::new())),
            algorithm_analyzer: Arc::new(RwLock::new(AlgorithmAnalyzer::new())),
            resource_manager: Arc::new(RwLock::new(ResourceManager::new())),
            plugin_bus: Arc::new(RwLock::new(PluginBus::new())),
            workflow_engine: Arc::new(RwLock::new(WorkflowEngine::new_with_llm(Some(llm_client.clone())))),
            llm_client,
            browser: Arc::new(RwLock::new(BrowserAutomationEngine::new())),
            flow_engine: Arc::new(RwLock::new(flow_engine)),
            requirement_compiler: Arc::new(RwLock::new(RequirementCompiler::new())),
            dialogue_graph,
            router,
            engine: Arc::new(RwLock::new(Engine::with_config(EngineConfig::default()))),
            multi_agent_orchestrator: Arc::new(RwLock::new(MultiAgentOrchestrator::new())),
        }
    }

    /// 处理用户消息 - 智能对话入口（优先使用真实LLM，降级到规则引擎）
    pub async fn chat(&self, session_id: &str, message: &str) -> Result<ChatResponse> {
        // 先把用户消息写入会话历史，保证两条调用路径都具备多轮对话记忆
        {
            let mut conv = self.conversation.write().await;
            conv.add_user_message(session_id, message);
        }

        // 全自动：对话内容自动落库并同步进知识图谱（优化布局）
        // 会话首次出现时自动建表级记录
        let _ = self.ensure_session(session_id).await;
        let _ = self
            .dialogue_graph
            .append_message(session_id, "user", message)
            .await;

        // 检测是否需要浏览器自动化
        let needs_browser = self.detect_browser_intent(message);

        if needs_browser {
            return self.handle_browser_automation(session_id, message).await;
        }

        // 尝试使用真实LLM
        let llm = self.llm_client.read().await;
        if llm.is_enabled() {
            drop(llm);
            return self.chat_with_llm(session_id, message).await;
        }
        drop(llm);

        // 降级到内置规则引擎（用户消息已写入，process_message 只追加助手回复）
        let mut conv = self.conversation.write().await;
        conv.process_message(session_id, message).await
    }

    /// 确保会话在对话库中存在（不存在则按 id 建立会话记录）
    async fn ensure_session(&self, session_id: &str) -> Result<()> {
        // 若会话不存在则创建（标题取截断 id，便于后续检索）
        let exists = self
            .dialogue_graph
            .db
            .query_one(
                "SELECT 1 FROM dialogue_sessions WHERE id = ?1",
                &[SqlValue::Text(session_id.to_string())],
            )
            .ok()
            .flatten()
            .is_some();
        if !exists {
            self.dialogue_graph
                .create_session(&format!("会话 {}", &session_id[..8.min(session_id.len())]))
                .await?;
        }
        Ok(())
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
        drop(conv);

        // history 已包含本次用户消息（由 chat 入口写入），直接映射为 LLM 上下文
        let messages: Vec<LLMChatMessage> = history.iter().map(|m| LLMChatMessage {
            role: match m.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
                _ => "system".to_string(),
            },
            content: m.content.clone(),
        }).collect();

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
                let mut conv = self.conversation.write().await;
                conv.process_message(session_id, message).await
            }
        }
    }

    async fn handle_browser_automation(&self, session_id: &str, message: &str) -> Result<ChatResponse> {
        let (url, steps) = {
            let _browser = self.browser.read().await;
            BrowserAutomationEngine::parse_natural_language(message)
        };

        if steps.is_empty() {
            let mut conv = self.conversation.write().await;
            return conv.process_message(session_id, "请提供要访问的URL或具体的浏览器操作指令").await;
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

        // 持久化浏览器回复到会话历史，保持多轮对话记忆一致
        let mut conv = self.conversation.write().await;
        let response = conv.add_assistant_message(session_id, &response_text);

        Ok(ChatResponse {
            message: response.message,
            actions: vec![],
            recommended_operators: vec![],
            suggestions: vec!["访问其他网站".to_string(), "截图网页".to_string(), "搜索内容".to_string()],
            workflow_suggestion: None,
        })
    }

    /// 配置LLM：同步更新底层 `LLMClient`，并把该配置作为一个 "custom" Provider 注册进
    /// AI Gateway 路由器（若启用），保证全系统的对话 / 需求编译 / 流程图 LLM 节点统一走最新配置。
    pub async fn configure_llm(&self, config: LLMConfig) {
        {
            let mut llm = self.llm_client.write().await;
            llm.update_config(config);
        }
        {
            let cfg = self.llm_client.read().await.get_config().clone();
            if cfg.enabled && !cfg.api_key.trim().is_empty() {
                let provider = provider::make_openai_compatible("custom", &cfg.api_base, &cfg.api_key, &cfg.model);
                self.router.register_provider(provider);
                // 把 custom 提到 fallback 链首位（用户显式配置优先）
                let mut chain = self.router.chain();
                chain.retain(|n| n != "custom");
                chain.insert(0, "custom".to_string());
                self.router.set_chain(chain);
                tracing::info!(target: "ai_gateway", model = %cfg.model, "已把最新 LLM 配置注册为 Gateway provider 'custom'（优先）");
            }
        }
    }

    pub async fn test_llm_connection(&self) -> Result<serde_json::Value> {
        let llm = self.llm_client.read().await;
        llm.test_connection().await.map_err(OperatorError::Other)
    }

    pub fn llm_client(&self) -> Arc<RwLock<LLMClient>> {
        self.llm_client.clone()
    }

    /// 返回 AI Gateway 路由器（组件化 Provider + fallback 链）
    pub fn router(&self) -> Arc<provider::LlmRouter> {
        self.router.clone()
    }

    pub fn browser(&self) -> Arc<RwLock<BrowserAutomationEngine>> {
        self.browser.clone()
    }

    /// 对话→知识图谱同步器（全自动对话整理）
    pub fn dialogue_graph(&self) -> Arc<DialogueGraphSyncer> {
        self.dialogue_graph.clone()
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

    // ============ 草莓多平台：对话驱动系统生成 ============

    /// 对话入口：把一句话需求编译成系统蓝图（功能点 + 关联关系 + 流程图）
    pub async fn compile_requirement(
        &self,
        requirement: &str,
        name: &str,
        tags: Vec<String>,
    ) -> Result<SystemBlueprint> {
        let mut rc = self.requirement_compiler.write().await;
        rc.compile(requirement, name, tags)
    }

    /// 继续对话迭代：在已有蓝图基础上增量追加功能
    pub async fn refine_blueprint(
        &self,
        blueprint_id: &str,
        addition: &str,
    ) -> Result<SystemBlueprint> {
        let mut rc = self.requirement_compiler.write().await;
        rc.refine(blueprint_id, addition)
    }

    /// 对话入口（接入真实 LLM 版）：把一句话需求编译成更细的系统蓝图。
    /// 内部把 `LLMClient` 适配成 `LlmFn` 喂给需求编译器；未配置 API key 时自动降级到规则抽取。
    pub async fn compile_requirement_with_llm(
        &self,
        requirement: &str,
        name: &str,
        tags: Vec<String>,
    ) -> Result<SystemBlueprint> {
        let llm = self.llm_client.read().await;
        let llm_fn: Option<crate::requirement_compiler::LlmFn> = if llm.is_enabled() {
            let client = (*llm).clone();
            Some(Arc::new(move |msgs: Vec<crate::requirement_compiler::LlmMsg>| {
                let client = client.clone();
                Box::pin(async move {
                    let chat_msgs: Vec<_> = msgs
                        .into_iter()
                        .map(|m| crate::llm_client::LLMChatMessage {
                            role: m.role,
                            content: m.content,
                        })
                        .collect();
                    client.chat(chat_msgs).await
                })
            }))
        } else {
            None
        };
        drop(llm);

        let mut rc = self.requirement_compiler.write().await;
        rc.compile_with_llm(requirement, name, tags, llm_fn.as_ref()).await
    }

    /// 把蓝图直接注册为可执行的 FlowDefinition（供 execute_flow 运行）
    pub async fn blueprint_to_flow(&self, bp: &SystemBlueprint) -> Result<()> {
        self.create_flow(bp.flow.clone()).await?;
        Ok(())
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

    /// 更新流程图（目标须已存在，更新后须通过结构校验）
    pub async fn update_flow(&self, flow: FlowDefinition) -> Result<FlowDefinition> {
        let mut engine = self.flow_engine.write().await;
        engine.update_flow(flow).map_err(|e| OperatorError::Other(anyhow::anyhow!(e.to_string())))
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
                    .unwrap_or("deepseek-chat");
                // 智能 AI 节点：支持在节点配置里指定 provider（如 "deepseek"/"openai"/"qwen"），
                // 不指定则走 AI Gateway 默认 fallback 链（deepseek→openai→…）。
                let preferred_provider = node.config.get("provider").and_then(|p| p.as_str()).map(|s| s.to_string());
                let temperature = node.config.get("temperature").and_then(|t| t.as_f64()).unwrap_or(0.7) as f32;

                let messages = vec![LLMChatMessage {
                    role: "user".into(),
                    content: prompt.clone(),
                }];
                let req = provider::ChatRequest {
                    messages,
                    temperature,
                    max_tokens: 2048,
                    tenant: None,
                    user: None,
                    trace_id: Some(node_id.clone()),
                };

                // 1) 优先走 AI Gateway（组件化 Provider + fallback 链 / 指定 provider）
                let gateway_result = {
                    let router = self.router();
                    match &preferred_provider {
                        Some(p) => router.chat_with_provider(p, req.clone()).await,
                        None => router.chat(req.clone()).await,
                    }
                };

                if let Ok(resp) = gateway_result {
                    NodeExecutionResult {
                        node_id, node_name, node_type: "llm".into(),
                        status: "success".into(),
                        input: Some(serde_json::json!({
                            "prompt": prompt,
                            "model": model,
                            "provider": preferred_provider.clone().unwrap_or_else(|| "gateway-fallback".to_string())
                        })),
                        output: Some(serde_json::json!({"response": resp.content, "provider": resp.provider, "model": resp.model})),
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    }
                } else {
                    // 2) 降级到直接 LLMClient（保持既有兜底）
                    let llm = self.llm_client.read().await;
                    if llm.is_enabled() {
                        match llm.chat(vec![LLMChatMessage { role: "user".into(), content: prompt.clone() }]).await {
                            Ok(response) => NodeExecutionResult {
                                node_id, node_name, node_type: "llm".into(),
                                status: "success".into(),
                                input: Some(serde_json::json!({"prompt": prompt, "model": model})),
                                output: Some(serde_json::json!({"response": response, "provider": "llm_client"})),
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
                        // 3) 模拟 LLM 响应（离线兜底）
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

    /// 执行 AI Agent 引擎任务
    pub async fn run_engine_task(&self, task: String) -> Result<engine::EngineResult> {
        let ctx = EngineContext {
            task,
            ..Default::default()
        };
        let mut engine = self.engine.write().await;
        *engine = Engine::with_config(EngineConfig::default())
            .with_context(ctx)
            .with_executors(Some(self.llm_client.clone()), Some(self.browser.clone()));
        Ok(engine.run().await)
    }

    // ============ 多 Agent 编排 API ============

    /// 生成子 Agent
    pub async fn spawn_agent(&self, role: AgentRole) -> String {
        let mut orchestrator = self.multi_agent_orchestrator.write().await;
        orchestrator.spawn_agent(role)
    }

    /// 运行多 Agent 任务
    pub async fn run_multi_agent_task(
        &self,
        tasks: Vec<(String, AgentRole, String)>,
        parallel: bool,
    ) -> Result<HashMap<String, engine::EngineResult>> {
        let mut orchestrator = self.multi_agent_orchestrator.write().await;
        let mut agent_ids = Vec::new();

        for (agent_id, role, task) in tasks {
            let id = if agent_id.is_empty() {
                orchestrator.spawn_agent(role)
            } else {
                orchestrator.spawn_agent_with_id(agent_id.clone(), role)
            };
            orchestrator.send_message("coordinator", &id, AgentRole::Coordinator, task);
            agent_ids.push(id);
        }

        let results = if parallel {
            orchestrator.run_parallel(&agent_ids).await
        } else {
            orchestrator.run_sequential(&agent_ids).await
        };

        Ok(results)
    }

    /// Agent 间通信：发送消息给指定 Agent
    pub async fn agent_communicate(
        &self,
        from: &str,
        to: &str,
        role: AgentRole,
        content: impl Into<String>,
    ) {
        let mut orchestrator = self.multi_agent_orchestrator.write().await;
        orchestrator.send_message(from, to, role, content);
    }

    /// 获取多 Agent 编排器引用
    pub fn multi_agent_orchestrator(&self) -> Arc<RwLock<MultiAgentOrchestrator>> {
        self.multi_agent_orchestrator.clone()
    }
}

impl Default for AIAgent {
    fn default() -> Self {
        Self::new()
    }
}
