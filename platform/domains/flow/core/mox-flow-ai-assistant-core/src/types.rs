// Copyright (c) 2026 璇玑 RelGraph · AI对话mox 模块化系统架构自动化核心 (AI Assistant Core)
// Licensed under the MIT License.

//! 核心类型定义

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 意图类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentType {
    /// 知识图谱查询
    GraphQuery,
    /// 知识库检索
    KnowledgeSearch,
    /// 数据分析
    DataAnalysis,
    /// 算法执行
    AlgorithmRun,
    /// 流程启动
    WorkflowStart,
    /// 实体创建
    EntityCreate,
    /// 关系创建
    RelationCreate,
    /// 文件操作
    FileOperation,
    /// 系统设置
    SystemSetting,
    /// 闲聊
    ChitChat,
    /// 任务管理
    TaskManagement,
    /// 报表生成
    ReportGenerate,
    /// 未知意图
    Unknown,
}

impl IntentType {
    pub fn description(&self) -> &'static str {
        match self {
            IntentType::GraphQuery => "知识图谱查询",
            IntentType::KnowledgeSearch => "知识库检索",
            IntentType::DataAnalysis => "数据分析",
            IntentType::AlgorithmRun => "算法执行",
            IntentType::WorkflowStart => "启动流程",
            IntentType::EntityCreate => "创建实体",
            IntentType::RelationCreate => "创建关系",
            IntentType::FileOperation => "文件操作",
            IntentType::SystemSetting => "系统设置",
            IntentType::ChitChat => "闲聊",
            IntentType::TaskManagement => "任务管理",
            IntentType::ReportGenerate => "生成报表",
            IntentType::Unknown => "未知意图",
        }
    }
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待处理
    Pending,
    /// 已分解
    Decomposed,
    /// 执行中
    Running,
    /// 等待中（需要用户输入）
    Waiting,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 智能体角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// 协调者
    Coordinator,
    /// 知识图谱专家
    GraphExpert,
    /// 知识库专家
    KnowledgeExpert,
    /// 数据分析师
    DataAnalyst,
    /// 算法工程师
    AlgorithmEngineer,
    /// 流程工程师
    WorkflowEngineer,
    /// 系统管理员
    SystemAdmin,
    /// 代码专家
    CodeExpert,
    /// 通用助手
    GeneralAssistant,
}

impl AgentRole {
    pub fn description(&self) -> &'static str {
        match self {
            AgentRole::Coordinator => "任务协调者",
            AgentRole::GraphExpert => "知识图谱专家",
            AgentRole::KnowledgeExpert => "知识库专家",
            AgentRole::DataAnalyst => "数据分析师",
            AgentRole::AlgorithmEngineer => "算法工程师",
            AgentRole::WorkflowEngineer => "流程工程师",
            AgentRole::SystemAdmin => "系统管理员",
            AgentRole::CodeExpert => "代码专家",
            AgentRole::GeneralAssistant => "通用助手",
        }
    }
}

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// 用户消息
    User,
    /// 助手消息
    Assistant,
    /// 系统消息
    System,
    /// 工具调用
    ToolCall,
    /// 工具结果
    ToolResult,
    /// 智能体间消息
    AgentMessage,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: String,
    pub message_type: MessageType,
    pub content: String,
    pub sender: String,
    pub timestamp: u64,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ConversationMessage {
    pub fn new(message_type: MessageType, content: &str, sender: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            message_type,
            content: content.to_string(),
            sender: sender.to_string(),
            timestamp: now_ms(),
            metadata: HashMap::new(),
        }
    }

    pub fn user(content: &str) -> Self {
        Self::new(MessageType::User, content, "user")
    }

    pub fn assistant(content: &str) -> Self {
        Self::new(MessageType::Assistant, content, "assistant")
    }
}

/// 智能体能力
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapability {
    /// 能力名称
    pub name: String,
    /// 能力描述
    pub description: String,
    /// 关联的工具列表
    pub tools: Vec<String>,
    /// 可处理的意图类型
    pub intents: Vec<IntentType>,
    /// 置信度 (0-1)
    pub confidence: f64,
}

/// 任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务 ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 任务类型（关联意图）
    pub intent: IntentType,
    /// 优先级
    pub priority: TaskPriority,
    /// 状态
    pub status: TaskStatus,
    /// 子任务列表
    pub subtasks: Vec<SubTask>,
    /// 分配的智能体
    pub assigned_agent: Option<String>,
    /// 任务参数
    pub params: HashMap<String, serde_json::Value>,
    /// 任务结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 父任务 ID（如果是子任务）
    pub parent_task_id: Option<String>,
    /// 创建时间
    pub created_at: u64,
    /// 完成时间
    pub completed_at: Option<u64>,
    /// 对话 ID
    pub conversation_id: Option<String>,
}

impl Task {
    /// 创建任务
    pub fn new(name: &str, intent: IntentType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            intent,
            priority: TaskPriority::Normal,
            status: TaskStatus::Pending,
            subtasks: Vec::new(),
            assigned_agent: None,
            params: HashMap::new(),
            result: None,
            error: None,
            parent_task_id: None,
            created_at: now_ms(),
            completed_at: None,
            conversation_id: None,
        }
    }

    /// 添加子任务
    pub fn add_subtask(&mut self, subtask: SubTask) {
        self.subtasks.push(subtask);
    }

    /// 进度（基于子任务完成比例）
    pub fn progress(&self) -> f64 {
        if self.subtasks.is_empty() {
            return match self.status {
                TaskStatus::Completed => 1.0,
                TaskStatus::Running => 0.5,
                _ => 0.0,
            };
        }
        let completed = self
            .subtasks
            .iter()
            .filter(|s| s.status == TaskStatus::Completed)
            .count() as f64;
        completed / self.subtasks.len() as f64
    }
}

/// 子任务
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    /// 子任务 ID
    pub id: String,
    /// 子任务名称
    pub name: String,
    /// 子任务描述
    pub description: String,
    /// 状态
    pub status: TaskStatus,
    /// 分配的智能体角色
    pub agent_role: AgentRole,
    /// 实际分配的智能体 ID
    pub assigned_agent: Option<String>,
    /// 依赖的子任务 ID 列表
    pub dependencies: Vec<String>,
    /// 子任务顺序
    pub order: u32,
    /// 结果
    pub result: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
}

impl SubTask {
    pub fn new(name: &str, agent_role: AgentRole, order: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: String::new(),
            status: TaskStatus::Pending,
            agent_role,
            assigned_agent: None,
            dependencies: Vec::new(),
            order,
            result: None,
            error: None,
        }
    }
}

/// 获取当前时间戳（毫秒）
pub fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new("分析图谱", IntentType::GraphQuery);
        assert_eq!(task.name, "分析图谱");
        assert_eq!(task.intent, IntentType::GraphQuery);
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, TaskPriority::Normal);
    }

    #[test]
    fn test_subtask_creation() {
        let subtask = SubTask::new("数据采集", AgentRole::DataAnalyst, 1);
        assert_eq!(subtask.name, "数据采集");
        assert_eq!(subtask.agent_role, AgentRole::DataAnalyst);
        assert_eq!(subtask.order, 1);
    }

    #[test]
    fn test_task_progress() {
        let mut task = Task::new("test", IntentType::DataAnalysis);
        assert_eq!(task.progress(), 0.0);

        let mut s1 = SubTask::new("step1", AgentRole::DataAnalyst, 1);
        s1.status = TaskStatus::Completed;
        let s2 = SubTask::new("step2", AgentRole::DataAnalyst, 2);

        task.add_subtask(s1);
        task.add_subtask(s2);

        assert_eq!(task.progress(), 0.5);
    }

    #[test]
    fn test_conversation_message() {
        let msg = ConversationMessage::user("你好");
        assert_eq!(msg.message_type, MessageType::User);
        assert_eq!(msg.sender, "user");
        assert_eq!(msg.content, "你好");

        let msg2 = ConversationMessage::assistant("你好！");
        assert_eq!(msg2.message_type, MessageType::Assistant);
    }

    #[test]
    fn test_intent_descriptions() {
        assert_eq!(IntentType::GraphQuery.description(), "知识图谱查询");
        assert_eq!(IntentType::Unknown.description(), "未知意图");
    }

    #[test]
    fn test_agent_role_descriptions() {
        assert_eq!(AgentRole::Coordinator.description(), "任务协调者");
        assert_eq!(AgentRole::GraphExpert.description(), "知识图谱专家");
    }
}
