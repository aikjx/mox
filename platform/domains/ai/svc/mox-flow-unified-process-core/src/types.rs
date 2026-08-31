// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 统一流程类型定义
//!
//! 融合三套引擎的类型系统：
//! - agent-svc/flow_engine::NodeType (16种)
//! - agent-svc/types::WorkflowNodeType (11种)
//! - flow-svc/model::NodeKind + ToolKind (10+8种)
//!
//! 采用「控制节点 + 工具类型」二级分类设计（源自 flow-svc model），
//! 扩展以覆盖另外两套引擎的全部语义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

// ============================================================================
// 节点类型
// ============================================================================

/// 节点语义类型
///
/// 采用 flow-svc 的二级分类设计：
/// - 控制节点：仅约束拓扑结构，零耗时
/// - 工作节点：实际执行业务逻辑
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedNodeKind {
    // === 控制节点 (is_control = true) ===
    /// 流程起点
    Start,
    /// 流程终点
    End,
    /// 排他判断分支（if / match）
    Decision,
    /// 并行网关：分叉
    ParallelFork,
    /// 并行网关：汇合
    ParallelJoin,
    /// 循环入口
    LoopStart,
    /// 循环出口
    LoopEnd,
    /// 守卫/门禁节点（前置拦截器）
    Guard,
    /// 子流程引用（可复用模板）
    SubFlow,

    // === 工作节点 (is_control = false) ===
    /// 通用任务（绑定 ToolKind）
    Task,
    /// 自定义脚本
    Script,
    /// 数据输入
    DataInput,
    /// 数据输出
    DataOutput,
    /// 数据转换
    Transform,
    /// 人工任务
    UserTask,
    /// 延迟等待
    Delay,
    /// 事件触发
    Event,
}

impl UnifiedNodeKind {
    /// 是否为可执行的实体工作节点（参与调度与关键路径计算）
    pub fn is_executable(&self) -> bool {
        matches!(
            self,
            UnifiedNodeKind::Task
                | UnifiedNodeKind::Script
                | UnifiedNodeKind::Transform
                | UnifiedNodeKind::UserTask
                | UnifiedNodeKind::Delay
                | UnifiedNodeKind::SubFlow
                | UnifiedNodeKind::Guard
        )
    }

    /// 是否为纯控制节点（零耗时，仅约束拓扑）
    pub fn is_control(&self) -> bool {
        !self.is_executable()
    }

    /// 节点分类标签
    pub fn category(&self) -> &'static str {
        if self.is_control() { "control" } else { "work" }
    }
}

// ============================================================================
// 工具类型
// ============================================================================

/// 工具类别 —— Task 节点的具体执行器类型
///
/// 决定互斥资源与冲突规则，映射到资源池
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedToolKind {
    /// 纯计算 / LLM 推理，无外部副作用
    Compute,
    /// 大模型调用（计费、限流）
    Llm,
    /// 浏览器 RPA
    Browser,
    /// 数据库
    Database,
    /// HTTP / 三方接口
    Http,
    /// 文件读写（Excel / CSV / 文档）
    File,
    /// 桌面自动化 / 系统命令
    Shell,
    /// 算子（OUS 算子统一系统）
    Operator,
    /// 插件调用
    Plugin,
    /// 人工审批
    Human,
}

impl UnifiedToolKind {
    /// 该工具默认独占的资源池名（用于资源受限调度）
    pub fn resource_pool(&self) -> &'static str {
        match self {
            UnifiedToolKind::Compute => "cpu",
            UnifiedToolKind::Llm => "llm",
            UnifiedToolKind::Browser => "browser",
            UnifiedToolKind::Database => "db",
            UnifiedToolKind::Http => "net",
            UnifiedToolKind::File => "io",
            UnifiedToolKind::Shell => "shell",
            UnifiedToolKind::Operator => "operator",
            UnifiedToolKind::Plugin => "plugin",
            UnifiedToolKind::Human => "human",
        }
    }
}

// ============================================================================
// 数据访问
// ============================================================================

/// 数据访问模式 —— 数据流依赖推断的基础
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedAccessMode {
    Read,
    Write,
    /// 读改写（等价于 Read + Write，且要求事务原子性）
    ReadWrite,
}

impl UnifiedAccessMode {
    pub fn reads(&self) -> bool {
        matches!(self, UnifiedAccessMode::Read | UnifiedAccessMode::ReadWrite)
    }
    pub fn writes(&self) -> bool {
        matches!(self, UnifiedAccessMode::Write | UnifiedAccessMode::ReadWrite)
    }
}

/// 一次具体的资源访问声明
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnifiedAccess {
    /// 资源标识：表名 / 文件路径 / URL / 变量名
    pub resource: String,
    pub mode: UnifiedAccessMode,
}

impl UnifiedAccess {
    pub fn read(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: UnifiedAccessMode::Read }
    }
    pub fn write(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: UnifiedAccessMode::Write }
    }
    pub fn rw(r: impl Into<String>) -> Self {
        Self { resource: r.into(), mode: UnifiedAccessMode::ReadWrite }
    }
}

// ============================================================================
// 节点配置
// ============================================================================

/// 节点配置 —— 类型化的 tagged enum
///
/// 替代 flow_engine 的无类型 JSON config，
/// 借鉴 workflow_engine 的 WorkflowNodeConfig 设计。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum UnifiedNodeConfig {
    Start,
    End,
    Task {
        /// 工具特定配置（透传给 NodeHandler）
        tool_config: serde_json::Value,
    },
    Decision {
        /// 条件表达式（支持 {{var}} 变量引用）
        expression: String,
    },
    Parallel {
        /// 合并策略
        merge_strategy: UnifiedMergeStrategy,
    },
    Loop {
        /// 循环条件
        condition: String,
        /// 最大迭代次数（防死循环）
        max_iterations: u32,
    },
    Guard {
        /// 守卫类型标识
        guard_type: String,
        /// 关联的规则 ID
        rule_id: Option<String>,
    },
    SubFlow {
        /// 子流程 ID
        flow_id: String,
        /// 输入变量映射 (子流程变量名 → 父流程变量名)
        input_mapping: HashMap<String, String>,
        /// 输出变量映射 (父流程变量名 → 子流程变量名)
        output_mapping: HashMap<String, String>,
    },
    Script {
        language: String,
        code: String,
    },
    DataInput {
        /// 静态值（优先使用）
        value: Option<serde_json::Value>,
        /// 动态来源（变量名）
        source: Option<String>,
    },
    DataOutput {
        /// 输出目标（变量名或外部资源）
        target: Option<String>,
    },
    Transform {
        /// 转换模板（支持 {{var}}）
        template: String,
    },
    UserTask {
        assignee: Option<String>,
        form: serde_json::Value,
    },
    Delay {
        duration_ms: u64,
    },
    Event {
        event_type: String,
        payload: serde_json::Value,
    },
}

/// 并行合并策略
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedMergeStrategy {
    /// 等待所有分支完成
    AllComplete,
    /// 任意分支完成即继续
    AnyComplete,
    /// 第一个成功的分支
    FirstSuccess,
    /// 多数投票
    VoteMajority,
}

// ============================================================================
// 节点
// ============================================================================

/// 流程节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFlowNode {
    pub id: String,
    pub name: String,
    pub kind: UnifiedNodeKind,
    /// 绑定的工具（Task 节点必填，其他可为 None）
    #[serde(default)]
    pub tool: Option<UnifiedToolKind>,
    /// 类型化配置
    pub config: UnifiedNodeConfig,
    /// 预估耗时（毫秒），用于关键路径与调度
    #[serde(default)]
    pub duration_ms: u64,
    /// 数据访问声明
    #[serde(default)]
    pub accesses: Vec<UnifiedAccess>,
    /// 语义标签（Guard 用 tag 声明自己校验了什么）
    #[serde(default)]
    pub tags: Vec<String>,
    /// 画布位置
    pub position: Option<UnifiedPosition>,
    /// 是否事务性节点（数据库事务边界）
    #[serde(default)]
    pub transactional: bool,
    /// 是否可重试（幂等）
    #[serde(default)]
    pub idempotent: bool,
    /// 任意扩展属性
    #[serde(default)]
    pub props: BTreeMap<String, String>,
}

impl UnifiedFlowNode {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: UnifiedNodeKind) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            tool: None,
            config: match kind {
                UnifiedNodeKind::Start => UnifiedNodeConfig::Start,
                UnifiedNodeKind::End => UnifiedNodeConfig::End,
                UnifiedNodeKind::Decision => UnifiedNodeConfig::Decision {
                    expression: String::new(),
                },
                UnifiedNodeKind::ParallelFork | UnifiedNodeKind::ParallelJoin => {
                    UnifiedNodeConfig::Parallel {
                        merge_strategy: UnifiedMergeStrategy::AllComplete,
                    }
                }
                _ => UnifiedNodeConfig::Task {
                    tool_config: serde_json::Value::Null,
                },
            },
            duration_ms: 0,
            accesses: Vec::new(),
            tags: Vec::new(),
            position: None,
            transactional: false,
            idempotent: false,
            props: BTreeMap::new(),
        }
    }

    pub fn task(
        id: impl Into<String>,
        name: impl Into<String>,
        tool: UnifiedToolKind,
        duration_ms: u64,
    ) -> Self {
        let mut n = Self::new(id, name, UnifiedNodeKind::Task);
        n.tool = Some(tool);
        n.duration_ms = duration_ms;
        n.config = UnifiedNodeConfig::Task {
            tool_config: serde_json::json!({}),
        };
        n
    }

    pub fn with_access(mut self, a: UnifiedAccess) -> Self {
        self.accesses.push(a);
        self
    }

    pub fn with_tag(mut self, t: impl Into<String>) -> Self {
        self.tags.push(t.into());
        self
    }

    pub fn with_config(mut self, config: UnifiedNodeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn transactional(mut self, v: bool) -> Self {
        self.transactional = v;
        self
    }

    pub fn idempotent(mut self, v: bool) -> Self {
        self.idempotent = v;
        self
    }

    /// 读集合
    pub fn read_set(&self) -> BTreeSet<&str> {
        self.accesses
            .iter()
            .filter(|a| a.mode.reads())
            .map(|a| a.resource.as_str())
            .collect()
    }

    /// 写集合
    pub fn write_set(&self) -> BTreeSet<&str> {
        self.accesses
            .iter()
            .filter(|a| a.mode.writes())
            .map(|a| a.resource.as_str())
            .collect()
    }
}

/// 画布位置
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnifiedPosition {
    pub x: f64,
    pub y: f64,
}

// ============================================================================
// 边
// ============================================================================

/// 边的语义
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedEdgeKind {
    /// 顺序控制流
    Sequence,
    /// 条件分支（带条件表达式）
    Conditional,
    /// 异常流（catch 边）
    Exception,
    /// 由数据流分析自动推断出的隐式依赖
    InferredData,
    /// 资源互斥序：由冲突修复注入的硬约束，数据流分析不得剪除
    Mutex,
}

impl UnifiedEdgeKind {
    /// 该边是否为不可剪除的硬约束
    pub fn is_hard(&self) -> bool {
        matches!(
            self,
            UnifiedEdgeKind::Conditional | UnifiedEdgeKind::Exception | UnifiedEdgeKind::Mutex
        )
    }
}

/// 流程边
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default = "default_edge_kind")]
    pub kind: UnifiedEdgeKind,
    /// 条件表达式（Conditional 边）
    #[serde(default)]
    pub condition: Option<String>,
}

fn default_edge_kind() -> UnifiedEdgeKind {
    UnifiedEdgeKind::Sequence
}

impl UnifiedFlowEdge {
    pub fn seq(id: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: from.into(),
            target: to.into(),
            kind: UnifiedEdgeKind::Sequence,
            condition: None,
        }
    }
    pub fn cond(
        id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        expr: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: from.into(),
            target: to.into(),
            kind: UnifiedEdgeKind::Conditional,
            condition: Some(expr.into()),
        }
    }
    pub fn exception(id: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: from.into(),
            target: to.into(),
            kind: UnifiedEdgeKind::Exception,
            condition: None,
        }
    }
    /// 资源互斥边：强制 from 先于 to，冲突修复专用
    pub fn mutex(id: impl Into<String>, from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: from.into(),
            target: to.into(),
            kind: UnifiedEdgeKind::Mutex,
            condition: None,
        }
    }
}

// ============================================================================
// 资源与规则
// ============================================================================

/// 资源池容量（并发上限）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedResourcePool {
    pub name: String,
    pub capacity: u32,
}

/// 合规 / 业务规则等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedSeverity {
    Info,
    Warning,
    /// 阻断级：必须在生成代码前修复
    Blocking,
}

/// 业务专家规则（政务 / 等保等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedExpertRule {
    pub id: String,
    pub description: String,
    pub severity: UnifiedSeverity,
    /// 命中该规则的资源前缀
    #[serde(default)]
    pub resource_prefixes: Vec<String>,
    /// 命中该规则的工具类别
    #[serde(default)]
    pub tool_kinds: Vec<UnifiedToolKind>,
    /// 满足规则所必须存在的前置 Guard 标签
    #[serde(default)]
    pub required_guard_tags: Vec<String>,
}

// ============================================================================
// 流程图
// ============================================================================

/// 统一流程图定义
///
/// 融合三套引擎的全部字段：
/// - flow_engine::FlowDefinition (id/name/description/nodes/edges/variables/timestamps)
/// - types::BusinessWorkflow (start_node_id)
/// - model::FlowGraph (pools/rules)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFlowGraph {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: Vec<UnifiedFlowNode>,
    pub edges: Vec<UnifiedFlowEdge>,
    pub variables: HashMap<String, serde_json::Value>,
    /// 资源池容量配置
    #[serde(default)]
    pub pools: Vec<UnifiedResourcePool>,
    /// 绑定的业务专家规则
    #[serde(default)]
    pub rules: Vec<UnifiedExpertRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UnifiedFlowGraph {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            variables: HashMap::new(),
            pools: Vec::new(),
            rules: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_node(&mut self, n: UnifiedFlowNode) -> &mut Self {
        self.nodes.push(n);
        self
    }

    pub fn add_edge(&mut self, e: UnifiedFlowEdge) -> &mut Self {
        self.edges.push(e);
        self
    }

    pub fn node(&self, id: &str) -> Option<&UnifiedFlowNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: &str) -> Option<&mut UnifiedFlowNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// 资源池容量，未显式配置时给出安全缺省
    pub fn capacity_of(&self, pool: &str) -> u32 {
        if let Some(p) = self.pools.iter().find(|p| p.name == pool) {
            return p.capacity.max(1);
        }
        // 安全缺省：无配置时容量为 1（保守策略）
        1
    }

    /// 获取 Start 节点
    pub fn start_node(&self) -> Option<&UnifiedFlowNode> {
        self.nodes
            .iter()
            .find(|n| matches!(n.kind, UnifiedNodeKind::Start))
    }

    /// 获取所有 End 节点
    pub fn end_nodes(&self) -> Vec<&UnifiedFlowNode> {
        self.nodes
            .iter()
            .filter(|n| matches!(n.kind, UnifiedNodeKind::End))
            .collect()
    }

    /// 获取某节点的所有出边
    pub fn outgoing_edges(&self, node_id: &str) -> Vec<&UnifiedFlowEdge> {
        self.edges.iter().filter(|e| e.source == node_id).collect()
    }

    /// 获取某节点的所有入边
    pub fn incoming_edges(&self, node_id: &str) -> Vec<&UnifiedFlowEdge> {
        self.edges.iter().filter(|e| e.target == node_id).collect()
    }
}

// ============================================================================
// 执行结果
// ============================================================================

/// 节点执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnifiedNodeStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    /// 被 Guard 阻断
    Blocked,
    /// 等待用户/外部事件
    Waiting,
}

/// 单个节点执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedNodeResult {
    pub node_id: String,
    pub node_name: String,
    pub node_kind: String,
    pub status: UnifiedNodeStatus,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl UnifiedNodeResult {
    pub fn success(node: &UnifiedFlowNode, output: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            node_kind: format!("{:?}", node.kind).to_lowercase(),
            status: UnifiedNodeStatus::Completed,
            input: None,
            output: Some(output),
            error: None,
            duration_ms,
        }
    }

    pub fn failed(node: &UnifiedFlowNode, error: String, duration_ms: u64) -> Self {
        Self {
            node_id: node.id.clone(),
            node_name: node.name.clone(),
            node_kind: format!("{:?}", node.kind).to_lowercase(),
            status: UnifiedNodeStatus::Failed,
            input: None,
            output: None,
            error: Some(error),
            duration_ms,
        }
    }

    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = Some(input);
        self
    }
}

/// 完整执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedExecutionResult {
    pub flow_id: String,
    pub flow_name: String,
    pub success: bool,
    pub node_results: Vec<UnifiedNodeResult>,
    pub final_output: Option<serde_json::Value>,
    pub variables: HashMap<String, serde_json::Value>,
    pub execution_time_ms: u64,
    pub error: Option<String>,
}

impl UnifiedExecutionResult {
    /// 成功的执行结果
    pub fn ok(
        graph: &UnifiedFlowGraph,
        node_results: Vec<UnifiedNodeResult>,
        final_output: Option<serde_json::Value>,
        variables: HashMap<String, serde_json::Value>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            flow_id: graph.id.clone(),
            flow_name: graph.name.clone(),
            success: true,
            node_results,
            final_output,
            variables,
            execution_time_ms,
            error: None,
        }
    }

    /// 失败的执行结果
    pub fn err(
        graph: &UnifiedFlowGraph,
        node_results: Vec<UnifiedNodeResult>,
        error: String,
        variables: HashMap<String, serde_json::Value>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            flow_id: graph.id.clone(),
            flow_name: graph.name.clone(),
            success: false,
            node_results,
            final_output: None,
            variables,
            execution_time_ms,
            error: Some(error),
        }
    }
}
