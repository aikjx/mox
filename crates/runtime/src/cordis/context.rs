//! 插件上下文：会话日志、算子注册表、Agent注册表

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use operator_core::Operator;
use async_trait::async_trait;

/// 插件上下文树
///
/// 设计原则：
/// - Session Log 是唯一事实源
/// - 所有注册表都是只读快照的引用
/// - 支持并发读取，写入通过追加日志
pub struct PluginContext {
    /// 会话日志（追加式）
    pub sessions: Arc<SessionLog>,
    /// 算子注册表
    pub operators: Arc<OperatorRegistry>,
    /// Agent注册表
    pub agents: Arc<AgentRegistry>,
    /// LLM适配器
    pub llm: Arc<RwLock<Option<LlmAdapter>>>,
    /// 系统提示组装器
    pub system_prompt: Arc<SystemPromptBuilder>,
}

impl PluginContext {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(SessionLog::new()),
            operators: Arc::new(OperatorRegistry::new()),
            agents: Arc::new(AgentRegistry::new()),
            llm: Arc::new(RwLock::new(None)),
            system_prompt: Arc::new(SystemPromptBuilder::new()),
        }
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 会话日志（追加式源）
///
/// 核心特性：
/// - 只追加，不修改
/// - 可重放，可审计
/// - 支持快照与回滚
pub struct SessionLog {
    /// 日志条目（追加式）
    entries: RwLock<Vec<SessionEntry>>,
    /// 索引（加速查询）
    index: RwLock<HashMap<String, Vec<usize>>>,
    /// 快照管理器
    snapshots: SnapshotManager,
}

impl SessionLog {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            index: RwLock::new(HashMap::new()),
            snapshots: SnapshotManager::new(),
        }
    }

    /// 追加条目
    pub async fn append(&self, entry: SessionEntry) -> Result<(), String> {
        let mut entries = self.entries.write();
        let mut index = self.index.write();

        let idx = entries.len();
        entries.push(entry.clone());

        // 更新索引
        match &entry {
            SessionEntry::TurnStart { turn_id, .. } => {
                index.entry(turn_id.as_str().to_string())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
            SessionEntry::StepStart { turn_id, .. } => {
                index.entry(turn_id.as_str().to_string())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
            _ => {}
        }

        Ok(())
    }

    /// 获取Turn所有条目
    pub fn get_turn_entries(&self, turn_id: &str) -> Vec<SessionEntry> {
        let entries = self.entries.read();
        let index = self.index.read();

        if let Some(indices) = index.get(turn_id) {
            indices.iter().map(|&i| entries[i].clone()).collect()
        } else {
            Vec::new()
        }
    }

    /// 创建快照
    pub async fn create_snapshot(&self) -> Result<String, String> {
        let entries = self.entries.read();
        self.snapshots.create(&entries)
    }

    /// 重放日志
    pub async fn replay(&self, from_idx: usize) -> Vec<SessionEntry> {
        let entries = self.entries.read();
        entries[from_idx..].to_vec()
    }

    /// 获取所有条目
    pub fn all_entries(&self) -> Vec<SessionEntry> {
        self.entries.read().clone()
    }
}

impl Default for SessionLog {
    fn default() -> Self {
        Self::new()
    }
}

/// 快照管理器
struct SnapshotManager {
    snapshots: RwLock<HashMap<String, Vec<SessionEntry>>>,
}

impl SnapshotManager {
    fn new() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
        }
    }

    fn create(&self, entries: &[SessionEntry]) -> Result<String, String> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let mut snapshots = self.snapshots.write();
        snapshots.insert(snapshot_id.clone(), entries.to_vec());
        Ok(snapshot_id)
    }

    fn restore(&self, snapshot_id: &str) -> Option<Vec<SessionEntry>> {
        let snapshots = self.snapshots.read();
        snapshots.get(snapshot_id).cloned()
    }
}

/// 算子注册表
pub struct OperatorRegistry {
    /// 算子映射
    operators: RwLock<HashMap<String, Arc<dyn Operator>>>,
    /// 工具映射
    tools: RwLock<HashMap<String, ToolDefinition>>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self {
            operators: RwLock::new(HashMap::new()),
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// 注册算子
    pub fn register(&self, operator: Arc<dyn Operator>) -> Result<(), String> {
        let mut operators = self.operators.write();
        let name = operator.metadata().name.clone();
        operators.insert(name.clone(), operator);
        Ok(())
    }

    /// 获取算子
    pub fn get(&self, name: &str) -> Option<Arc<dyn Operator>> {
        let operators = self.operators.read();
        operators.get(name).cloned()
    }

    /// 注册工具
    pub fn register_tool(&self, tool: ToolDefinition) -> Result<(), String> {
        let mut tools = self.tools.write();
        tools.insert(tool.name.clone(), tool);
        Ok(())
    }

    /// 获取所有工具
    pub fn all_tools(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read();
        tools.values().cloned().collect()
    }
}

impl Default for OperatorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 工具定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
}

/// Agent注册表
pub struct AgentRegistry {
    /// Agent映射
    agents: RwLock<HashMap<String, AgentDefinition>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: RwLock::new(HashMap::new()),
        }
    }

    /// 注册Agent
    pub fn register(&self, agent: AgentDefinition) -> Result<(), String> {
        let mut agents = self.agents.write();
        agents.insert(agent.name.clone(), agent);
        Ok(())
    }

    /// 获取Agent
    pub fn get(&self, name: &str) -> Option<AgentDefinition> {
        let agents = self.agents.read();
        agents.get(name).cloned()
    }

    /// 配置Agent
    pub fn configure(&self, config: AgentConfig) -> Result<(), String> {
        let mut agents = self.agents.write();
        if let Some(agent) = agents.get_mut(&config.name) {
            agent.config = Some(config);
            Ok(())
        } else {
            Err(format!("Agent not found: {}", config.name))
        }
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub tools: Vec<String>,
    pub config: Option<AgentConfig>,
}

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<String>>,
}

/// LLM适配器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAdapter {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl LlmAdapter {
    pub fn configure(&self, _config: LlmConfig) -> Result<(), String> {
        // TODO: 实现配置逻辑
        Ok(())
    }
}

/// LLM配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

/// 系统提示组装器
pub struct SystemPromptBuilder {
    templates: RwLock<HashMap<String, String>>,
}

impl SystemPromptBuilder {
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// 构建系统提示
    pub fn build(&self, agent: &AgentDefinition, tools: &[ToolDefinition]) -> String {
        let mut prompt = agent.system_prompt.clone();

        // 添加工具描述
        if !tools.is_empty() {
            prompt.push_str("\n\n可用工具：\n");
            for tool in tools {
                prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
            }
        }

        prompt
    }

    /// 注册模板
    pub fn register_template(&self, name: String, template: String) {
        let mut templates = self.templates.write();
        templates.insert(name, template);
    }
}

impl Default for SystemPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// 导入必要的类型
use crate::cordis::SessionEntry;
