//! 插件上下文：会话日志、算子注册表、Agent注册表

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use operator_core::Operator;

/// 会话日志条目：记录一次 Turn / Step 的生命周期事件，是溯源(SoT)的最小单元。
/// 定义于此供 `cordis` 模块统一 re-export（`cordis::SessionEntry`）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionEntry {
    /// Turn 开始
    TurnStart {
        turn_id: String,
        agent_id: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// Step 开始
    StepStart {
        step_id: String,
        turn_id: String,
        action: String,
    },
    /// Turn 完成（携带摘要）
    TurnComplete {
        turn_id: String,
        summary: String,
    },
}

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
        // agent 编排侧的 LLM 适配器默认从环境变量 DEEPSEEK_API_KEY 接入真实 DeepSeek；
        // 未设置 Key 时为 None，由具体调用方按降级策略处理。
        let llm = std::env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|k| !k.trim().is_empty())
            .map(|_| LlmAdapter::from_env());
        Self {
            sessions: Arc::new(SessionLog::new()),
            operators: Arc::new(OperatorRegistry::new()),
            agents: Arc::new(AgentRegistry::new()),
            llm: Arc::new(RwLock::new(llm)),
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

/// LLM适配器：agent 编排侧统一的真实 LLM 连接抽象（OpenAI 兼容协议）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAdapter {
    pub provider: String,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

impl Default for LlmAdapter {
    fn default() -> Self {
        // 默认指向真实 DeepSeek（OpenAI 兼容协议）
        Self {
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
        }
    }
}

impl LlmAdapter {
    /// 从环境变量构建真实 DeepSeek 适配器；未设置 DEEPSEEK_API_KEY 时返回未配置实例。
    pub fn from_env() -> Self {
        let key = std::env::var("DEEPSEEK_API_KEY").ok().filter(|k| !k.trim().is_empty());
        Self {
            provider: "deepseek".to_string(),
            model: std::env::var("DEEPSEEK_MODEL").ok().unwrap_or_else(|| "deepseek-chat".to_string()),
            api_key: key.clone(),
            base_url: Some(
                std::env::var("DEEPSEEK_BASE_URL")
                    .ok()
                    .unwrap_or_else(|| "https://api.deepseek.com/v1".to_string()),
            ),
        }
    }

    /// 是否已具备真实调用条件（必须存在 API Key）。
    pub fn is_configured(&self) -> bool {
        self.api_key.as_ref().map(|k| !k.trim().is_empty()).unwrap_or(false)
    }

    /// 用显式 LlmConfig 覆盖当前适配器（保留真实连接能力，支持前端/Profile 覆盖）。
    pub fn configure(&self, config: LlmConfig) -> Result<LlmAdapter, String> {
        if config.api_key.as_ref().map(|k| k.trim().is_empty()).unwrap_or(true) {
            return Err("LLM 未配置：缺少 API Key（请设置 DEEPSEEK_API_KEY）".to_string());
        }
        Ok(LlmAdapter {
            provider: if config.provider.trim().is_empty() {
                "deepseek".to_string()
            } else {
                config.provider
            },
            model: if config.model.trim().is_empty() {
                "deepseek-chat".to_string()
            } else {
                config.model
            },
            api_key: config.api_key,
            base_url: config
                .base_url
                .filter(|u| !u.trim().is_empty())
                .or_else(|| self.base_url.clone())
                .or_else(|| Some("https://api.deepseek.com/v1".to_string())),
        })
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

impl Default for LlmConfig {
    fn default() -> Self {
        // 默认读取真实 DeepSeek 配置，未设置 Key 时留空由运行时提示
        Self {
            provider: "deepseek".to_string(),
            model: "deepseek-chat".to_string(),
            api_key: std::env::var("DEEPSEEK_API_KEY").ok(),
            base_url: Some("https://api.deepseek.com/v1".to_string()),
        }
    }
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
