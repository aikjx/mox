// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! AI智能体通用类型定义

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ========== 对话系统类型 ==========

/// 会话消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// 对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub referenced_operators: Vec<String>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            referenced_operators: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            referenced_operators: Vec::new(),
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            referenced_operators: Vec::new(),
        }
    }
}

/// 对话会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub context: SessionContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionContext {
    pub current_workflow: Option<Vec<String>>,
    pub selected_operators: Vec<String>,
    pub intent: Option<UserIntent>,
    pub variables: HashMap<String, serde_json::Value>,
}

/// 用户意图识别
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserIntent {
    /// 询问系统状态
    QueryStatus,
    /// 列出可用算子
    ListOperators,
    /// 执行工作流
    ExecuteWorkflow { operators: Vec<String> },
    /// 分析算法
    AnalyzeAlgorithm { algo_type: String },
    /// 创建自定义算子
    CreateOperator,
    /// 查询资源状态
    QueryResources,
    /// 插件管理
    ManagePlugins,
    /// 查看知识图谱
    ViewGraph,
    /// 获取推荐
    GetRecommendation,
    /// 普通对话
    GeneralChat,
    /// 创建业务流程
    CreateWorkflow,
    /// 其他
    Unknown,
}

/// 对话响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub message: ChatMessage,
    pub suggestions: Vec<String>,
    pub recommended_operators: Vec<String>,
    pub actions: Vec<SuggestedAction>,
    pub workflow_suggestion: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    ExecuteWorkflow,
    ViewOperator,
    CreateWorkflow,
    AnalyzeAlgorithm,
    ShowResources,
    ShowGraph,
}

// ========== 算法分析类型 ==========

/// 算法类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlgorithmType {
    /// 排序算法
    Sorting,
    /// 搜索算法
    Search,
    /// 图算法
    Graph,
    /// 机器学习
    MachineLearning,
    /// 深度学习
    DeepLearning,
    /// 优化算法
    Optimization,
    /// 线性代数
    LinearAlgebra,
    /// 信号处理
    SignalProcessing,
    /// 数据流处理
    DataFlow,
    /// 自定义
    Custom(String),
}

/// 归一化流程图节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: FlowNodeType,
    pub label: String,
    pub description: String,
    pub operator_id: Option<String>,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
    pub parallel_group: Option<String>,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FlowNodeType {
    Start,
    End,
    Process,
    Decision,
    Parallel,
    Merge,
    Input,
    Output,
    SubProcess,
}

/// 归一化流程图边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: Option<String>,
    pub condition: Option<String>,
    pub data_type: Option<String>,
}

/// 算法流程图 - 归一化输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmFlow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub algorithm_type: AlgorithmType,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub operator_mapping: HashMap<String, String>,
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
    pub complexity_analysis: ComplexityAnalysis,
    pub normalized_workflow: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub id: String,
    pub description: String,
    pub impact: OptimizationImpact,
    pub applicable_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OptimizationImpact {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityAnalysis {
    pub time_complexity: String,
    pub space_complexity: String,
    pub parallelizability: f64,
    pub bottlenecks: Vec<String>,
}

// ========== 资源管理类型 ==========

/// 资源类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Cpu,
    Memory,
    Gpu,
    DiskIo,
    Network,
    Plugin,
    Operator,
    Workflow,
    Custom(String),
}

/// 资源分配记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub id: String,
    pub resource_type: ResourceType,
    pub owner_id: String,
    pub owner_type: String,
    pub amount: f64,
    pub allocated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 资源使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageStats {
    pub resource_type: ResourceType,
    pub total: f64,
    pub used: f64,
    pub available: f64,
    pub utilization_percent: f64,
    pub allocations: Vec<ResourceAllocation>,
}

/// 资源全景视图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePanorama {
    pub timestamp: DateTime<Utc>,
    pub resources: HashMap<String, ResourceUsageStats>,
    pub active_plugins: usize,
    pub active_workflows: usize,
    pub cached_operators: usize,
    pub total_allocations: usize,
}

// ========== 插件互通类型 ==========

/// 插件信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub plugin_type: PluginType,
    pub capabilities: Vec<String>,
    pub input_topics: Vec<String>,
    pub output_topics: Vec<String>,
    pub status: PluginStatus,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Wasm,
    Builtin,
    External,
    AiModel,
    DataSource,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PluginStatus {
    Registered,
    Active,
    Paused,
    Error,
}

/// 插件间消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    pub id: String,
    pub source_plugin: String,
    pub target_plugin: Option<String>,
    pub topic: String,
    pub payload: serde_json::Value,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
    pub response_required: bool,
}

impl PluginMessage {
    pub fn new(source: &str, topic: &str, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_plugin: source.to_string(),
            target_plugin: None,
            topic: topic.to_string(),
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
            response_required: false,
        }
    }

    pub fn to_target(mut self, target: &str) -> Self {
        self.target_plugin = Some(target.to_string());
        self
    }

    pub fn with_correlation(mut self, corr_id: &str) -> Self {
        self.correlation_id = Some(corr_id.to_string());
        self
    }

    pub fn need_response(mut self) -> Self {
        self.response_required = true;
        self
    }
}

/// 消息订阅
#[derive(Debug, Clone)]
pub struct MessageSubscription {
    pub id: String,
    pub plugin_id: String,
    pub topic_pattern: String,
    pub handler: tokio::sync::mpsc::Sender<PluginMessage>,
}

// ========== 业务流程引擎类型 ==========

/// 业务工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BusinessWorkflow {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
    pub variables: HashMap<String, serde_json::Value>,
    pub start_node_id: String,
    pub created_at: DateTime<Utc>,
}

/// 工作流节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: WorkflowNodeType,
    pub name: String,
    pub config: WorkflowNodeConfig,
    pub position: Option<NodePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeType {
    Start,
    End,
    Operator,
    Condition,
    Parallel,
    SubWorkflow,
    UserTask,
    Script,
    AiTask,
    PluginCall,
    Delay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WorkflowNodeConfig {
    Start,
    End,
    Operator {
        operator_id: String,
        parameters: HashMap<String, serde_json::Value>,
    },
    Condition {
        expression: String,
        true_path: String,
        false_path: String,
    },
    Parallel {
        branches: Vec<String>,
        merge_strategy: MergeStrategy,
    },
    SubWorkflow {
        workflow_id: String,
    },
    UserTask {
        assignee: Option<String>,
        form: serde_json::Value,
    },
    Script {
        language: String,
        code: String,
    },
    AiTask {
        task_type: String,
        prompt: String,
    },
    PluginCall {
        plugin_id: String,
        method: String,
        parameters: serde_json::Value,
    },
    Delay {
        duration_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    AllComplete,
    AnyComplete,
    FirstSuccess,
    VoteMajority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// 工作流边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
}

/// 工作流执行实例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub id: String,
    pub workflow_id: String,
    pub status: WorkflowStatus,
    pub current_nodes: Vec<String>,
    pub variables: HashMap<String, serde_json::Value>,
    pub node_executions: Vec<NodeExecutionRecord>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkflowStatus {
    Pending,
    Running,
    WaitingUser,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    pub node_id: String,
    pub status: WorkflowStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// 工作流执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub instance: WorkflowInstance,
    pub final_output: Option<serde_json::Value>,
    pub execution_log: Vec<String>,
    pub metrics: WorkflowMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowMetrics {
    pub total_execution_time_ms: u64,
    pub nodes_executed: usize,
    pub operators_called: usize,
    pub plugins_called: usize,
    pub parallel_branches: usize,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub total_duration_ms: u64,
}

/// 工作流连接（与WorkflowEdge兼容的简化结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConnection {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

/// 节点执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// 工作流模板
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub nodes: Vec<WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub variables: HashMap<String, serde_json::Value>,
}

impl WorkflowTemplate {
    pub fn create_workflow(&self, instance_id: &str) -> BusinessWorkflow {
        // 找到start node
        let start_node_id = self
            .nodes
            .iter()
            .find(|n| matches!(n.node_type, WorkflowNodeType::Start))
            .map(|n| n.id.clone())
            .unwrap_or_else(|| "start".to_string());

        // 转换connections为edges
        let edges: Vec<WorkflowEdge> = self
            .connections
            .iter()
            .map(|c| WorkflowEdge {
                id: format!("edge-{}-{}", c.from, c.to),
                source: c.from.clone(),
                target: c.to.clone(),
                condition: c.label.clone(),
            })
            .collect();

        BusinessWorkflow {
            id: format!("wf-{}", instance_id),
            name: self.name.clone(),
            description: self.description.clone(),
            nodes: self.nodes.clone(),
            edges,
            variables: self.variables.clone(),
            start_node_id,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_factories_set_role() {
        let u = ChatMessage::user("hello");
        let a = ChatMessage::assistant("hi");
        let s = ChatMessage::system("sys");
        assert_eq!(u.role, MessageRole::User);
        assert_eq!(a.role, MessageRole::Assistant);
        assert_eq!(s.role, MessageRole::System);
        assert!(!u.id.is_empty() && !a.id.is_empty() && !s.id.is_empty());
        // 内容保留
        assert_eq!(u.content, "hello");
        assert!(a.referenced_operators.is_empty());
    }

    #[test]
    fn test_plugin_message_builder_chain() {
        let msg = PluginMessage::new("src", "topic.a", serde_json::json!({"k": 1}))
            .to_target("dst")
            .with_correlation("corr-1")
            .need_response();
        assert_eq!(msg.source_plugin, "src");
        assert_eq!(msg.target_plugin.as_deref(), Some("dst"));
        assert_eq!(msg.correlation_id.as_deref(), Some("corr-1"));
        assert!(msg.response_required);
        assert_eq!(msg.topic, "topic.a");
    }

    #[test]
    fn test_workflow_template_create_workflow_converts_connections_to_edges() {
        let template = WorkflowTemplate {
            id: "tpl-1".to_string(),
            name: "测试模板".to_string(),
            description: "单测模板".to_string(),
            category: "test".to_string(),
            nodes: vec![
                WorkflowNode {
                    id: "start".to_string(),
                    node_type: WorkflowNodeType::Start,
                    name: "开始".to_string(),
                    config: WorkflowNodeConfig::Start,
                    position: None,
                },
                WorkflowNode {
                    id: "end".to_string(),
                    node_type: WorkflowNodeType::End,
                    name: "结束".to_string(),
                    config: WorkflowNodeConfig::End,
                    position: None,
                },
            ],
            connections: vec![WorkflowConnection {
                from: "start".to_string(),
                to: "end".to_string(),
                label: None,
            }],
            variables: HashMap::new(),
        };

        let wf = template.create_workflow("abc123");
        assert_eq!(wf.id, "wf-abc123");
        assert_eq!(wf.start_node_id, "start");
        assert_eq!(wf.edges.len(), 1);
        assert_eq!(wf.edges[0].source, "start");
        assert_eq!(wf.edges[0].target, "end");
        assert_eq!(wf.nodes.len(), 2);
    }

    #[test]
    fn test_user_intent_serialization() {
        let intent = UserIntent::ExecuteWorkflow {
            operators: vec!["linear".to_string()],
        };
        let json = serde_json::to_string(&intent).unwrap();
        let back: UserIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(intent, back);
    }

    #[test]
    fn test_resource_type_snake_case_serialization() {
        let rt = ResourceType::Cpu;
        assert_eq!(serde_json::to_string(&rt).unwrap(), "\"cpu\"");
        let rt2 = ResourceType::Custom("gpu_cluster".to_string());
        let back: ResourceType =
            serde_json::from_str(&serde_json::to_string(&rt2).unwrap()).unwrap();
        assert_eq!(rt2, back);
    }
}
