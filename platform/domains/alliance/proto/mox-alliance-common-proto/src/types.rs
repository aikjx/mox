// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家联盟通用领域类型
//!
//! 所有联盟子服务共享的核心数据结构定义于此。
//! 遵循 SSOT 原则：每个领域概念只有一个权威定义。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Task（任务） ───────────────────────────────────────────────────────────

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// 待处理
    Pending,
    /// 规划中（生成协作计划）
    Planning,
    /// 执行中
    Running,
    /// 已暂停
    Paused,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Planning | Self::Running | Self::Paused)
    }
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum TaskPriority {
    Low = 1,
    Normal = 5,
    High = 8,
    Critical = 10,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// 协作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllianceMode {
    /// 串行协作（专家按顺序执行）
    Sequential,
    /// 并行协作（多专家同时执行）
    Parallel,
    /// 辩论模式（专家相互辩论，最终裁决）
    Debate,
    /// 分层协作（分层递进，每层多位专家）
    Hierarchical,
    /// 迭代精炼（多轮迭代，逐步优化结果）
    Iterative,
    /// 投票裁决（多专家投票，多数决）
    Voting,
}

impl Default for AllianceMode {
    fn default() -> Self {
        Self::Parallel
    }
}

/// 融合策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FusionStrategy {
    /// 投票融合（多数决）
    Voting,
    /// 加权融合（按专家权重）
    Weighted,
    /// 拼接融合（结果拼接）
    Concatenation,
    /// 择优融合（选最优结果）
    BestOf,
    /// 辩论融合（对抗辩论）
    Debate,
    /// 迭代精炼融合
    Iterative,
}

impl Default for FusionStrategy {
    fn default() -> Self {
        Self::Weighted
    }
}

/// 任务 — 用户提交的一次协作请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // === 标识 ===
    pub task_id: Uuid,
    pub tenant_id: Uuid,
    pub user_id: Uuid,

    // === 内容 ===
    pub title: String,
    pub description: String,
    pub task_type: String,

    // === 状态 ===
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub progress: f32,
    pub current_node_id: Option<String>,

    // === 协作配置 ===
    pub mode: AllianceMode,
    pub fusion_strategy: FusionStrategy,

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

impl Task {
    pub fn new(tenant_id: Uuid, user_id: Uuid, title: String, description: String) -> Self {
        Self {
            task_id: Uuid::new_v4(),
            tenant_id,
            user_id,
            title,
            description,
            task_type: "custom".to_string(),
            status: TaskStatus::Pending,
            priority: TaskPriority::default(),
            progress: 0.0,
            current_node_id: None,
            mode: AllianceMode::default(),
            fusion_strategy: FusionStrategy::default(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            duration_ms: None,
        }
    }
}

// ─── Node（DAG 节点） ──────────────────────────────────────────────────────

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

impl NodeStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Skipped | Self::Cancelled
        )
    }
}

/// DAG 节点 — 协作计划中的一个执行单元
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub node_id: String,
    pub task_id: Uuid,
    pub expert_id: String,

    pub name: String,
    pub description: Option<String>,

    pub status: NodeStatus,
    pub retry_count: u32,

    /// 依赖的上游节点 ID 列表
    pub dependencies: Vec<String>,

    /// 节点输入数据引用
    pub input_refs: Vec<String>,

    /// 节点输出数据引用
    pub output_ref: Option<String>,

    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,

    pub error_message: Option<String>,
}

// ─── CollaborationPlan（协作计划） ────────────────────────────────────────

/// 协作计划 — 任务的 DAG 执行图
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationPlan {
    pub task_id: Uuid,
    pub mode: AllianceMode,
    pub fusion_strategy: FusionStrategy,
    pub nodes: Vec<Node>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
}

impl CollaborationPlan {
    /// 检查计划是否有效（无环 + 所有依赖存在）
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::{HashMap, HashSet};

        let node_ids: HashSet<&str> = self.nodes.iter().map(|n| n.node_id.as_str()).collect();

        // 检查所有依赖都存在
        for node in &self.nodes {
            for dep in &node.dependencies {
                if !node_ids.contains(dep.as_str()) {
                    return Err(format!("Node {} depends on non-existent node {}", node.node_id, dep));
                }
            }
        }

        // 简单环检测（拓扑排序）
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for node in &self.nodes {
            in_degree.entry(&node.node_id).or_insert(0);
            for dep in &node.dependencies {
                *in_degree.entry(dep).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<&str> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut count = 0;
        while let Some(node_id) = queue.pop() {
            count += 1;
            if let Some(node) = self.nodes.iter().find(|n| n.node_id == node_id) {
                for dep in &node.dependencies {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(dep);
                        }
                    }
                }
            }
        }

        if count != self.nodes.len() {
            return Err("Cycle detected in collaboration plan".to_string());
        }

        Ok(())
    }
}

// ─── Expert（专家） ────────────────────────────────────────────────────────

/// 专家状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpertStatus {
    /// 未激活
    Inactive,
    /// 激活可用
    Active,
    /// 维护中
    Maintenance,
    /// 已下线
    Deprecated,
}

/// 专家健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertHealth {
    pub is_healthy: bool,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub error_count: u64,
}

impl Default for ExpertHealth {
    fn default() -> Self {
        Self {
            is_healthy: true,
            last_heartbeat: None,
            success_rate: 1.0,
            avg_latency_ms: 0.0,
            error_count: 0,
        }
    }
}

/// 能力声明 — 专家可执行的一类操作的抽象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub capability_id: String,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub version: String,
}

/// 工具绑定 — 能力对应的具体可调用方法
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolBinding {
    pub tool_id: String,
    pub name: String,
    pub description: String,
    pub protocol: String, // gRPC / HTTP / MCP
    pub endpoint: String,
    pub input_schema: Option<String>,
    pub output_schema: Option<String>,
}

/// 领域 — 知识/业务的分类范畴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Domain {
    pub domain_id: String,
    pub name: String,
    pub description: String,
    pub parent_id: Option<String>,
}

/// 专家 — 具备领域知识和工具能力的自治 Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expert {
    // === 标识 ===
    pub expert_id: String,
    pub tenant_id: String, // "system" = 系统内置
    pub name: String,
    pub version: String,

    // === 描述 ===
    pub description: String,
    pub domains: Vec<String>,
    pub capabilities: Vec<Capability>,

    // === 工具绑定 ===
    pub tools: Vec<ToolBinding>,

    // === 状态 ===
    pub status: ExpertStatus,
    pub health: ExpertHealth,
    pub priority: u8,

    // === 时间 ===
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Expert {
    pub fn new_system(name: String, description: String) -> Self {
        let now = Utc::now();
        Self {
            expert_id: Uuid::new_v4().to_string(),
            tenant_id: "system".to_string(),
            name,
            version: "0.1.0".to_string(),
            description,
            domains: vec![],
            capabilities: vec![],
            tools: vec![],
            status: ExpertStatus::Active,
            health: ExpertHealth::default(),
            priority: 5,
            created_at: now,
            updated_at: now,
        }
    }
}
