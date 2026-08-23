//! 多 Agent 协作框架 - 支持子 Agent 的创建、通信与编排执行
//!
//! 核心设计：
//! - AgentRole: 定义五种 Agent 角色（研究者/分析师/撰写者/协调者/执行者）
//! - SubAgent: 封装独立的 Engine 实例，携带角色与消息队列
//! - MultiAgentOrchestrator: 管理多个 SubAgent，支持并行/顺序/广播等编排模式

use super::engine_loop::{Engine, EngineContext, EngineResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── AgentRole ──────────────────────────────────────────────

/// Agent 角色枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// 研究者：负责信息搜集与调研
    Researcher,
    /// 分析师：负责数据分析与洞察
    Analyst,
    /// 撰写者：负责内容生成与文档编写
    Writer,
    /// 协调者：负责任务分配与结果整合
    Coordinator,
    /// 执行者：负责实际操作与工具调用
    Executor,
}

impl AgentRole {
    pub fn as_str(&self) -> &str {
        match self {
            AgentRole::Researcher => "researcher",
            AgentRole::Analyst => "analyst",
            AgentRole::Writer => "writer",
            AgentRole::Coordinator => "coordinator",
            AgentRole::Executor => "executor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "researcher" => Some(AgentRole::Researcher),
            "analyst" => Some(AgentRole::Analyst),
            "writer" => Some(AgentRole::Writer),
            "coordinator" => Some(AgentRole::Coordinator),
            "executor" => Some(AgentRole::Executor),
            _ => None,
        }
    }

    /// 获取该角色的系统提示词
    pub fn system_prompt(&self) -> &str {
        match self {
            AgentRole::Researcher => {
                "你是一个专业的研究者。你的职责是搜集信息、进行调研，并将发现整理为结构化的研究报告。请注重事实与来源。"
            }
            AgentRole::Analyst => {
                "你是一个专业的分析师。你的职责是分析数据、发现规律、提供洞察。请用数据支撑你的结论。"
            }
            AgentRole::Writer => {
                "你是一个专业的撰写者。你的职责是根据给定素材撰写高质量内容，包括文章、报告、摘要等。"
            }
            AgentRole::Coordinator => {
                "你是一个协调者。你的职责是将任务拆解为子任务，分发给合适的 Agent，并整合各 Agent 的结果。"
            }
            AgentRole::Executor => {
                "你是一个执行者。你的职责是执行具体的操作任务，包括调用工具、处理数据等。"
            }
        }
    }
}

// ── AgentMessage ───────────────────────────────────────────

/// Agent 间通信消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub from_agent: String,
    pub to_agent: Option<String>,
    pub role: AgentRole,
    pub content: String,
    pub message_type: AgentMessageType,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AgentMessage {
    pub fn new(from: &str, role: AgentRole, content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_agent: from.to_string(),
            to_agent: None,
            role,
            content: content.into(),
            message_type: AgentMessageType::Task,
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    pub fn to_agent(mut self, to: &str) -> Self {
        self.to_agent = Some(to.to_string());
        self
    }

    pub fn with_type(mut self, msg_type: AgentMessageType) -> Self {
        self.message_type = msg_type;
        self
    }

    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageType {
    Task,
    Result,
    Query,
    Response,
    Broadcast,
    Acknowledgment,
}

// ── SubAgent ───────────────────────────────────────────────

/// 子 Agent：封装独立的 Engine 实例
pub struct SubAgent {
    pub id: String,
    pub role: AgentRole,
    pub engine: Engine,
    pub message_queue: Vec<AgentMessage>,
    pub status: SubAgentStatus,
    pub last_result: Option<EngineResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SubAgentStatus {
    Idle,
    Running,
    Completed,
    Failed,
}

impl SubAgent {
    pub fn new(id: String, role: AgentRole, engine: Engine) -> Self {
        Self {
            id,
            role,
            engine,
            message_queue: Vec::new(),
            status: SubAgentStatus::Idle,
            last_result: None,
        }
    }

    pub fn with_context(mut self, ctx: EngineContext) -> Self {
        self.engine = self.engine.with_context(ctx);
        self
    }

    pub async fn execute(&mut self, task: String) -> EngineResult {
        self.status = SubAgentStatus::Running;
        let ctx = EngineContext {
            task,
            ..Default::default()
        };
        let engine = std::mem::take(&mut self.engine);
        self.engine = engine.with_context(ctx);
        let result = self.engine.run().await;
        self.status = if result.success {
            SubAgentStatus::Completed
        } else {
            SubAgentStatus::Failed
        };
        self.last_result = Some(result.clone());
        result
    }

    pub fn enqueue_message(&mut self, msg: AgentMessage) {
        self.message_queue.push(msg);
    }

    pub fn drain_messages(&mut self) -> Vec<AgentMessage> {
        std::mem::take(&mut self.message_queue)
    }

    pub fn pending_message_count(&self) -> usize {
        self.message_queue.len()
    }
}

// ── MessageBus ─────────────────────────────────────────────

/// Agent 间消息总线
#[derive(Default)]
pub struct MessageBus {
    subscribers: HashMap<String, Vec<AgentMessage>>,
    broadcast_log: Vec<AgentMessage>,
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new(),
            broadcast_log: Vec::new(),
        }
    }

    pub fn send(&mut self, msg: AgentMessage) {
        if let Some(target) = &msg.to_agent {
            self.subscribers
                .entry(target.clone())
                .or_default()
                .push(msg.clone());
        }
        self.broadcast_log.push(msg);
    }

    pub fn send_to(&mut self, target_id: &str, msg: AgentMessage) {
        self.subscribers
            .entry(target_id.to_string())
            .or_default()
            .push(msg.clone());
        self.broadcast_log.push(msg);
    }

    pub fn receive(&mut self, agent_id: &str) -> Vec<AgentMessage> {
        self.subscribers.remove(agent_id).unwrap_or_default()
    }

    pub fn broadcast(&mut self, msg: AgentMessage) {
        self.broadcast_log.push(msg);
    }

    pub fn message_count(&self) -> usize {
        self.broadcast_log.len()
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscribers.len()
    }
}

// ── MultiAgentOrchestrator ─────────────────────────────────

/// 多 Agent 编排器
pub struct MultiAgentOrchestrator {
    agents: HashMap<String, SubAgent>,
    message_bus: MessageBus,
}

impl MultiAgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            message_bus: MessageBus::new(),
        }
    }

    /// 创建子 Agent
    pub fn spawn_agent(&mut self, role: AgentRole) -> String {
        let id = Uuid::new_v4().to_string();
        let engine = Engine::new();
        let agent = SubAgent::new(id.clone(), role.clone(), engine);
        self.agents.insert(id.clone(), agent);
        tracing::info!(target: "multi_agent", agent_id = %id, role = ?role, "创建子 Agent");
        id
    }

    /// 创建带指定 ID 的子 Agent
    pub fn spawn_agent_with_id(&mut self, id: String, role: AgentRole) -> String {
        let engine = Engine::new();
        let agent = SubAgent::new(id.clone(), role.clone(), engine);
        self.agents.insert(id.clone(), agent);
        tracing::info!(target: "multi_agent", agent_id = %id, role = ?role, "创建子 Agent");
        id
    }

    /// 向指定 Agent 发送消息
    pub fn send_message(&mut self, from: &str, to: &str, role: AgentRole, content: impl Into<String>) {
        let msg = AgentMessage::new(from, role, content).to_agent(to);
        self.message_bus.send_to(to, msg.clone());
        if let Some(agent) = self.agents.get_mut(to) {
            agent.enqueue_message(msg);
        }
    }

    /// 向所有 Agent 广播消息
    pub fn broadcast_task(&mut self, from: &str, role: AgentRole, content: impl Into<String>) {
        let msg = AgentMessage::new(from, role, content)
            .with_type(AgentMessageType::Broadcast);
        let agent_ids: Vec<String> = self.agents.keys().cloned().collect();
        for agent_id in &agent_ids {
            let cloned = msg.clone();
            self.message_bus.send_to(agent_id, cloned);
        }
        tracing::info!(
            target: "multi_agent",
            agent_count = agent_ids.len(),
            "广播任务给所有 Agent"
        );
    }

    /// 并行执行多个 Agent
    pub async fn run_parallel(&mut self, agent_ids: &[String]) -> HashMap<String, EngineResult> {
        let mut results = HashMap::new();
        let mut agents_to_run: Vec<(String, String)> = Vec::new();

        for id in agent_ids {
            if let Some(agent) = self.agents.get(id) {
                let task = agent
                    .message_queue
                    .first()
                    .map(|m| m.content.clone())
                    .unwrap_or_else(|| format!("Agent {} 无待处理任务", id));
                agents_to_run.push((id.clone(), task));
            }
        }

        for (id, task) in &agents_to_run {
            if let Some(agent) = self.agents.get_mut(id) {
                let result = agent.execute(task.clone()).await;
                results.insert(id.clone(), result);
            }
        }

        tracing::info!(
            target: "multi_agent",
            parallel_count = results.len(),
            "并行执行完成"
        );
        results
    }

    /// 顺序执行多个 Agent
    pub async fn run_sequential(&mut self, agent_ids: &[String]) -> HashMap<String, EngineResult> {
        let mut results = HashMap::new();

        for id in agent_ids {
            if let Some(agent) = self.agents.get_mut(id) {
                let task = agent
                    .message_queue
                    .first()
                    .map(|m| m.content.clone())
                    .unwrap_or_else(|| format!("Agent {} 无待处理任务", id));
                let result = agent.execute(task).await;
                tracing::info!(
                    target: "multi_agent",
                    agent_id = %id,
                    success = result.success,
                    "顺序 Agent 执行完成"
                );
                results.insert(id.clone(), result);
            }
        }

        results
    }

    /// 获取指定 Agent 的状态
    pub fn get_agent_status(&self, agent_id: &str) -> Option<&SubAgentStatus> {
        self.agents.get(agent_id).map(|a| &a.status)
    }

    /// 获取所有 Agent ID 列表
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    /// 获取 Agent 数量
    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    /// 移除指定 Agent
    pub fn remove_agent(&mut self, agent_id: &str) -> Option<SubAgent> {
        self.agents.remove(agent_id)
    }

    /// 从消息总线接收指定 Agent 的消息
    pub fn receive_messages(&mut self, agent_id: &str) -> Vec<AgentMessage> {
        self.message_bus.receive(agent_id)
    }
}

impl Default for MultiAgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_serialization() {
        let role = AgentRole::Researcher;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"researcher\"");
        let back: AgentRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, AgentRole::Researcher);
    }

    #[test]
    fn test_agent_role_from_str() {
        assert_eq!(AgentRole::parse("researcher"), Some(AgentRole::Researcher));
        assert_eq!(AgentRole::parse("analyst"), Some(AgentRole::Analyst));
        assert_eq!(AgentRole::parse("writer"), Some(AgentRole::Writer));
        assert_eq!(AgentRole::parse("coordinator"), Some(AgentRole::Coordinator));
        assert_eq!(AgentRole::parse("executor"), Some(AgentRole::Executor));
        assert_eq!(AgentRole::parse("unknown"), None);
    }

    #[test]
    fn test_spawn_agent() {
        let mut orchestrator = MultiAgentOrchestrator::new();
        let id = orchestrator.spawn_agent(AgentRole::Researcher);
        assert!(!id.is_empty());
        assert_eq!(orchestrator.agent_count(), 1);
        assert!(orchestrator.get_agent_status(&id).is_some());
        assert_eq!(*orchestrator.get_agent_status(&id).unwrap(), SubAgentStatus::Idle);
    }

    #[test]
    fn test_message_bus_send_receive() {
        let mut bus = MessageBus::new();
        bus.send(AgentMessage::new("agent1", AgentRole::Researcher, "hello").to_agent("agent2"));
        let msgs = bus.receive("agent2");
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].from_agent, "agent1");
        assert_eq!(msgs[0].content, "hello");
    }

    #[test]
    fn test_broadcast() {
        let mut orchestrator = MultiAgentOrchestrator::new();
        let _id1 = orchestrator.spawn_agent(AgentRole::Researcher);
        let _id2 = orchestrator.spawn_agent(AgentRole::Analyst);
        orchestrator.broadcast_task("coordinator", AgentRole::Coordinator, "全体会议");
        assert_eq!(orchestrator.agent_count(), 2);
    }

    #[test]
    fn test_list_agents() {
        let mut orchestrator = MultiAgentOrchestrator::new();
        orchestrator.spawn_agent(AgentRole::Researcher);
        orchestrator.spawn_agent(AgentRole::Analyst);
        orchestrator.spawn_agent(AgentRole::Writer);
        assert_eq!(orchestrator.list_agents().len(), 3);
    }

    #[test]
    fn test_remove_agent() {
        let mut orchestrator = MultiAgentOrchestrator::new();
        let id = orchestrator.spawn_agent(AgentRole::Researcher);
        assert_eq!(orchestrator.agent_count(), 1);
        let removed = orchestrator.remove_agent(&id);
        assert!(removed.is_some());
        assert_eq!(orchestrator.agent_count(), 0);
    }

    #[test]
    fn test_sub_agent_execute() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let engine = Engine::new();
            let mut agent = SubAgent::new("test-agent".to_string(), AgentRole::Researcher, engine);
            let result = agent.execute("测试任务".to_string()).await;
            assert!(result.success);
            assert_eq!(agent.status, SubAgentStatus::Completed);
        });
    }

    #[test]
    fn test_multi_agent_sequential() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut orchestrator = MultiAgentOrchestrator::new();
            let id1 = orchestrator.spawn_agent(AgentRole::Researcher);
            let id2 = orchestrator.spawn_agent(AgentRole::Writer);

            orchestrator.send_message("coordinator", &id1, AgentRole::Coordinator, "收集数据");
            orchestrator.send_message("coordinator", &id2, AgentRole::Coordinator, "撰写报告");

            let ids = vec![id1.clone(), id2.clone()];
            let results = orchestrator.run_sequential(&ids).await;
            assert_eq!(results.len(), 2);
            assert!(results.contains_key(&id1));
            assert!(results.contains_key(&id2));
        });
    }

    #[test]
    fn test_multi_agent_parallel() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut orchestrator = MultiAgentOrchestrator::new();
            let id1 = orchestrator.spawn_agent(AgentRole::Researcher);
            let id2 = orchestrator.spawn_agent(AgentRole::Analyst);

            orchestrator.send_message("coordinator", &id1, AgentRole::Coordinator, "搜集资料");
            orchestrator.send_message("coordinator", &id2, AgentRole::Coordinator, "分析数据");

            let ids = vec![id1.clone(), id2.clone()];
            let results = orchestrator.run_parallel(&ids).await;
            assert_eq!(results.len(), 2);
        });
    }
}