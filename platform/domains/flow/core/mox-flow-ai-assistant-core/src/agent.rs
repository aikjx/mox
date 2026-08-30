// Copyright (c) 2026 璇玑 RelGraph · AI对话全维自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 智能体与多智能体协同
//!
//! 支持多种角色的智能体，协同完成复杂任务。
//! 每个智能体有自己的能力、工具和决策逻辑。

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AiError, AiResult};
use crate::tool_registry::ToolRegistry;
use crate::types::*;

/// 智能体消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// 消息 ID
    pub id: String,
    /// 发送者智能体 ID
    pub from_agent: String,
    /// 接收者智能体 ID（"*" 表示广播）
    pub to_agent: String,
    /// 消息类型
    pub message_type: String,
    /// 消息内容
    pub content: String,
    /// 时间戳
    pub timestamp: u64,
    /// 关联任务 ID
    pub task_id: Option<String>,
}

impl AgentMessage {
    pub fn new(from: &str, to: &str, msg_type: &str, content: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from_agent: from.to_string(),
            to_agent: to.to_string(),
            message_type: msg_type.to_string(),
            content: content.to_string(),
            timestamp: now_ms(),
            task_id: None,
        }
    }
}

/// 智能体定义
#[derive(Debug, Clone)]
pub struct Agent {
    /// 智能体 ID
    pub id: String,
    /// 智能体名称
    pub name: String,
    /// 智能体角色
    pub role: AgentRole,
    /// 描述
    pub description: String,
    /// 能力列表
    pub capabilities: Vec<AgentCapability>,
    /// 可使用的工具名称列表
    pub tools: Vec<String>,
    /// 是否在线
    pub online: bool,
    /// 处理的任务数
    pub tasks_completed: u64,
    /// 系统提示词
    pub system_prompt: String,
    /// 温度参数
    pub temperature: f64,
}

impl Agent {
    /// 创建智能体
    pub fn new(name: &str, role: AgentRole) -> Self {
        let system_prompt = match role {
            AgentRole::Coordinator => {
                "你是一个任务协调专家，负责分解复杂任务、分配子任务给合适的专家，并汇总最终结果。"
                    .to_string()
            }
            AgentRole::GraphExpert => {
                "你是知识图谱专家，精通图数据库查询、图算法和知识图谱建模。"
                    .to_string()
            }
            AgentRole::KnowledgeExpert => {
                "你是知识库专家，擅长信息检索、文档管理和知识组织。"
                    .to_string()
            }
            AgentRole::DataAnalyst => {
                "你是数据分析师，擅长数据处理、统计分析和数据可视化。"
                    .to_string()
            }
            AgentRole::AlgorithmEngineer => {
                "你是算法工程师，精通各类算法的选择、调优和执行。"
                    .to_string()
            }
            AgentRole::WorkflowEngineer => {
                "你是流程工程师，擅长业务流程建模和流程自动化。"
                    .to_string()
            }
            AgentRole::SystemAdmin => {
                "你是系统管理员，负责系统配置、用户管理和权限设置。"
                    .to_string()
            }
            AgentRole::CodeExpert => {
                "你是代码专家，精通多种编程语言和软件开发。"
                    .to_string()
            }
            AgentRole::GeneralAssistant => {
                "你是通用助手，负责生成最终回答、整理结果和与用户交互。"
                    .to_string()
            }
        };

        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            role,
            description: role.description().to_string(),
            capabilities: Vec::new(),
            tools: Vec::new(),
            online: true,
            tasks_completed: 0,
            system_prompt,
            temperature: 0.7,
        }
    }

    /// 添加能力
    pub fn add_capability(&mut self, capability: AgentCapability) {
        self.capabilities.push(capability);
    }

    /// 添加工具
    pub fn add_tool(&mut self, tool_name: &str) {
        self.tools.push(tool_name.to_string());
    }

    /// 计算智能体对某意图的匹配度
    pub fn intent_match_score(&self, intent: IntentType) -> f64 {
        for cap in &self.capabilities {
            if cap.intents.contains(&intent) {
                return cap.confidence;
            }
        }
        0.0
    }
}

/// 智能体注册表
pub struct AgentRegistry {
    /// 智能体表
    agents: RwLock<HashMap<String, Agent>>,
    /// 名称索引
    name_index: RwLock<HashMap<String, String>>, // name -> id
    /// 角色索引
    role_index: RwLock<HashMap<AgentRole, Vec<String>>>,
    /// 消息总线（简化：消息队列）
    message_bus: RwLock<Vec<AgentMessage>>,
    /// 工具注册表引用
    tool_registry: Arc<ToolRegistry>,
}

impl AgentRegistry {
    /// 创建智能体注册表（内置默认智能体）
    pub fn new(tool_registry: Arc<ToolRegistry>) -> Self {
        let registry = Self {
            agents: RwLock::new(HashMap::new()),
            name_index: RwLock::new(HashMap::new()),
            role_index: RwLock::new(HashMap::new()),
            message_bus: RwLock::new(Vec::new()),
            tool_registry,
        };
        registry.register_default_agents();
        registry
    }

    /// 注册默认智能体
    fn register_default_agents(&self) {
        // 协调者
        let mut coordinator = Agent::new("coordinator", AgentRole::Coordinator);
        coordinator.add_capability(AgentCapability {
            name: "任务协调".to_string(),
            description: "任务分解、分配和协调".to_string(),
            tools: Vec::new(),
            intents: vec![
                IntentType::GraphQuery,
                IntentType::DataAnalysis,
                IntentType::AlgorithmRun,
                IntentType::ReportGenerate,
            ],
            confidence: 0.9,
        });
        self.register(coordinator).unwrap();

        // 图谱专家
        let mut graph_expert = Agent::new("graph-expert", AgentRole::GraphExpert);
        graph_expert.add_capability(AgentCapability {
            name: "图谱查询".to_string(),
            description: "知识图谱查询与分析".to_string(),
            tools: vec!["graph_query".to_string()],
            intents: vec![IntentType::GraphQuery],
            confidence: 0.95,
        });
        self.register(graph_expert).unwrap();

        // 知识库专家
        let mut kb_expert = Agent::new("knowledge-expert", AgentRole::KnowledgeExpert);
        kb_expert.add_capability(AgentCapability {
            name: "知识检索".to_string(),
            description: "知识库检索与问答".to_string(),
            tools: vec!["knowledge_search".to_string()],
            intents: vec![IntentType::KnowledgeSearch],
            confidence: 0.95,
        });
        self.register(kb_expert).unwrap();

        // 数据分析师
        let mut analyst = Agent::new("data-analyst", AgentRole::DataAnalyst);
        analyst.add_capability(AgentCapability {
            name: "数据分析".to_string(),
            description: "数据处理与分析".to_string(),
            tools: vec!["data_query".to_string()],
            intents: vec![IntentType::DataAnalysis, IntentType::ReportGenerate],
            confidence: 0.95,
        });
        self.register(analyst).unwrap();

        // 算法工程师
        let mut algo_engineer = Agent::new("algo-engineer", AgentRole::AlgorithmEngineer);
        algo_engineer.add_capability(AgentCapability {
            name: "算法执行".to_string(),
            description: "算法选择与执行".to_string(),
            tools: vec!["algorithm_run".to_string()],
            intents: vec![IntentType::AlgorithmRun],
            confidence: 0.95,
        });
        self.register(algo_engineer).unwrap();

        // 通用助手
        let mut assistant = Agent::new("general-assistant", AgentRole::GeneralAssistant);
        assistant.add_capability(AgentCapability {
            name: "通用对话".to_string(),
            description: "日常对话和结果整理".to_string(),
            tools: Vec::new(),
            intents: vec![IntentType::ChitChat],
            confidence: 0.8,
        });
        self.register(assistant).unwrap();
    }

    /// 注册智能体
    pub fn register(&self, agent: Agent) -> AiResult<Agent> {
        if self.name_index.read().contains_key(&agent.name) {
            return Err(AiError::AlreadyExists(format!(
                "agent '{}' already exists",
                agent.name
            )));
        }

        self.name_index
            .write()
            .insert(agent.name.clone(), agent.id.clone());
        self.role_index
            .write()
            .entry(agent.role)
            .or_default()
            .push(agent.id.clone());
        self.agents
            .write()
            .insert(agent.id.clone(), agent.clone());

        Ok(agent)
    }

    /// 按名称获取智能体
    pub fn get_by_name(&self, name: &str) -> Option<Agent> {
        let id = self.name_index.read().get(name)?.clone();
        self.agents.read().get(&id).cloned()
    }

    /// 按 ID 获取
    pub fn get_by_id(&self, id: &str) -> Option<Agent> {
        self.agents.read().get(id).cloned()
    }

    /// 按角色获取智能体列表
    pub fn get_by_role(&self, role: AgentRole) -> Vec<Agent> {
        let ids = self
            .role_index
            .read()
            .get(&role)
            .cloned()
            .unwrap_or_default();
        let agents = self.agents.read();
        ids.iter()
            .filter_map(|id| agents.get(id).cloned())
            .filter(|a| a.online)
            .collect()
    }

    /// 找到最适合处理某意图的智能体
    pub fn find_best_for_intent(&self, intent: IntentType) -> Option<Agent> {
        let agents = self.agents.read();
        let mut best: Option<&Agent> = None;
        let mut best_score = 0.0;

        for agent in agents.values() {
            if !agent.online {
                continue;
            }
            let score = agent.intent_match_score(intent);
            if score > best_score {
                best_score = score;
                best = Some(agent);
            }
        }

        best.cloned()
    }

    /// 发送智能体消息
    pub fn send_message(&self, message: AgentMessage) {
        self.message_bus.write().push(message);
    }

    /// 获取发给某智能体的消息
    pub fn get_messages_for(&self, agent_id: &str) -> Vec<AgentMessage> {
        self.message_bus
            .read()
            .iter()
            .filter(|m| m.to_agent == agent_id || m.to_agent == "*")
            .cloned()
            .collect()
    }

    /// 列出所有在线智能体
    pub fn list_online(&self) -> Vec<Agent> {
        self.agents
            .read()
            .values()
            .filter(|a| a.online)
            .cloned()
            .collect()
    }

    /// 智能体总数
    pub fn count(&self) -> usize {
        self.agents.read().len()
    }

    /// 在线数量
    pub fn online_count(&self) -> usize {
        self.agents.read().values().filter(|a| a.online).count()
    }
}

fn now_ms() -> u64 {
    crate::types::now_ms()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_registry() -> AgentRegistry {
        AgentRegistry::new(Arc::new(ToolRegistry::new()))
    }

    #[test]
    fn test_default_agents() {
        let registry = create_registry();
        assert!(registry.count() >= 5);
        assert_eq!(registry.online_count(), registry.count());
    }

    #[test]
    fn test_get_by_name() {
        let registry = create_registry();
        let agent = registry.get_by_name("coordinator").unwrap();
        assert_eq!(agent.role, AgentRole::Coordinator);
    }

    #[test]
    fn test_get_by_role() {
        let registry = create_registry();
        let graph_experts = registry.get_by_role(AgentRole::GraphExpert);
        assert_eq!(graph_experts.len(), 1);
        assert_eq!(graph_experts[0].name, "graph-expert");
    }

    #[test]
    fn test_find_best_for_intent() {
        let registry = create_registry();

        let best = registry.find_best_for_intent(IntentType::GraphQuery).unwrap();
        assert_eq!(best.role, AgentRole::GraphExpert);

        let best2 = registry
            .find_best_for_intent(IntentType::DataAnalysis)
            .unwrap();
        assert_eq!(best2.role, AgentRole::DataAnalyst);
    }

    #[test]
    fn test_agent_messages() {
        let registry = create_registry();

        let msg = AgentMessage::new("coordinator", "graph-expert", "task", "请查询图谱");
        registry.send_message(msg);

        let messages = registry.get_messages_for("graph-expert");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "请查询图谱");
    }

    #[test]
    fn test_broadcast_message() {
        let registry = create_registry();

        let msg = AgentMessage::new("coordinator", "*", "announcement", "大家好");
        registry.send_message(msg);

        // 每个智能体都能收到广播
        let messages = registry.get_messages_for("graph-expert");
        assert_eq!(messages.len(), 1);
    }

    #[test]
    fn test_intent_match_score() {
        let registry = create_registry();
        let graph_expert = registry.get_by_name("graph-expert").unwrap();
        assert!(graph_expert.intent_match_score(IntentType::GraphQuery) > 0.9);
        assert_eq!(graph_expert.intent_match_score(IntentType::ChitChat), 0.0);
    }

    #[test]
    fn test_list_online() {
        let registry = create_registry();
        let online = registry.list_online();
        assert!(!online.is_empty());
    }
}
