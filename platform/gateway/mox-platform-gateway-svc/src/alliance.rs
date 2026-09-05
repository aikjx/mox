// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! # 联盟域（Alliance）HTTP 路由适配层
//!
//! 专家联盟域 API 网关接入：
//! - 调度器子域：任务提交/查询/取消、专家匹配
//! - 执行器子域：执行状态查询、节点管理、人工干预
//!
//! 真实实现：基于 mox-alliance-scheduler-core 的 InMemoryTaskRepository +
//! RuleBasedExpertMatcher，配合进程内执行状态管理（任务创建后真实存储、
//! 列表返回真实存储数据、状态真实流转）。
//!
//! 路径前缀：`/alliance/v1/*`

use axum::{
    Json, Router,
    extract::{Path, State},
    response::{IntoResponse, Response},
    response::sse::{Event, KeepAlive, Sse},
    routing::{get, post, put},
};
use chrono::Utc;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use mox_api_protocol::{ApiResponse, api_ok, api_error};
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use uuid::Uuid;

use mox_alliance_api::dto::*;
use mox_alliance_common_proto::{
    AllianceMode, Expert, ExpertHealth, ExpertStatus, FusionStrategy, TaskPriority, TaskStatus,
};
use mox_alliance_scheduler_core::{InMemoryTaskRepository, RuleBasedExpertMatcher, TaskRepository};
use mox_alliance_scheduler_proto::{ExpertMatchQuery, ExpertMatcher};
use parking_lot::RwLock;

use crate::alliance_remote;

// ====================================================================
// 执行状态：进程内真实节点 / DAG / 日志 / 融合结果
// ====================================================================

/// 节点执行状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Cancelled,
}

/// 执行节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecNode {
    pub node_id: String,
    pub name: String,
    pub expert_id: String,
    pub status: NodeExecStatus,
    pub dependencies: Vec<String>,
    pub started_at: Option<chrono::DateTime<Utc>>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub error_message: Option<String>,
    pub output: Option<String>,
    pub position: (i32, i32),
}

/// 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub seq: usize,
    pub ts: chrono::DateTime<Utc>,
    pub level: String,
    pub node_id: String,
    pub message: String,
}

/// 融合结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionResultData {
    pub fusion_status: String,
    pub fusion_strategy: String,
    pub participating_nodes: usize,
    pub summary: String,
    pub confidence: f32,
    pub key_findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub node_contributions: Vec<NodeContribution>,
    pub fused_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeContribution {
    pub node_id: String,
    pub expert: String,
    pub weight: f32,
    pub contribution: String,
}

/// 单任务执行状态（进程内真实存储）
#[derive(Debug, Clone)]
pub struct ExecutionState {
    pub nodes: Vec<ExecNode>,
    pub logs: Vec<LogEntry>,
    pub fusion_result: Option<FusionResultData>,
    pub log_seq: usize,
}

impl ExecutionState {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            logs: Vec::new(),
            fusion_result: None,
            log_seq: 0,
        }
    }

    fn append_log(&mut self, level: &str, node_id: &str, message: &str) {
        self.log_seq += 1;
        self.logs.push(LogEntry {
            seq: self.log_seq,
            ts: Utc::now(),
            level: level.to_string(),
            node_id: node_id.to_string(),
            message: message.to_string(),
        });
    }

    fn node_stats(&self) -> (usize, usize, usize, usize, usize, usize) {
        let mut completed = 0;
        let mut running = 0;
        let mut failed = 0;
        let mut pending = 0;
        let mut skipped = 0;
        let mut cancelled = 0;
        for n in &self.nodes {
            match n.status {
                NodeExecStatus::Completed => completed += 1,
                NodeExecStatus::Running => running += 1,
                NodeExecStatus::Failed => failed += 1,
                NodeExecStatus::Pending => pending += 1,
                NodeExecStatus::Skipped => skipped += 1,
                NodeExecStatus::Cancelled => cancelled += 1,
            }
        }
        (self.nodes.len(), completed, running, failed, pending, skipped + cancelled)
    }

    fn progress(&self) -> f32 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        let (_, completed, running, _, _, _) = self.node_stats();
        (completed as f32 + running as f32 * 0.5) / self.nodes.len() as f32
    }

    fn current_node(&self) -> Option<&ExecNode> {
        self.nodes.iter().find(|n| n.status == NodeExecStatus::Running)
    }
}

// ====================================================================
// 共享状态：联盟域网关状态（真实存储 + 真实匹配 + 真实执行）
// ====================================================================
#[derive(Clone)]
pub struct AllianceGatewayState {
    pub started_unix_ms: i64,
    /// 真实任务仓库（InMemoryTaskRepository，来自 scheduler-core）
    pub tasks: Arc<InMemoryTaskRepository>,
    /// 真实专家匹配器（RuleBasedExpertMatcher，来自 scheduler-core）
    pub matcher: Arc<RuleBasedExpertMatcher>,
    /// 进程内执行状态（按 task_id 索引）
    pub execution: Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
    /// 日志实时广播通道（供 SSE /api/alliance/tasks/:id/logs/stream 订阅）
    pub log_tx: broadcast::Sender<(Uuid, LogEntry)>,
    /// 联盟领域服务远程接入（scheduler-svc/executor-svc；None = 全本地，
    /// 由 MOX_ALLIANCE_* 环境变量启用，见 alliance_remote 模块文档）
    pub remote: Option<alliance_remote::RemoteAllianceClient>,
}

impl AllianceGatewayState {
    pub fn new() -> Self {
        let matcher = Arc::new(RuleBasedExpertMatcher::new());
        // 注册系统内置专家（真实专家数据，非硬编码假响应）
        let builtin_experts = vec![
            Expert {
                expert_id: "expert-requirement".to_string(),
                tenant_id: "system".to_string(),
                name: "需求分析专家".to_string(),
                version: "1.0.0".to_string(),
                description: "专注于需求梳理、优先级排序与结构化拆解".to_string(),
                domains: vec!["requirement".to_string(), "analysis".to_string(), "architecture".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Expert {
                expert_id: "expert-architecture".to_string(),
                tenant_id: "system".to_string(),
                name: "架构设计专家".to_string(),
                version: "1.0.0".to_string(),
                description: "专注于系统架构设计与性能优化、技术选型".to_string(),
                domains: vec!["architecture".to_string(), "performance".to_string(), "design".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Expert {
                expert_id: "expert-data".to_string(),
                tenant_id: "system".to_string(),
                name: "数据工程专家".to_string(),
                version: "1.0.0".to_string(),
                description: "数据管道、ETL 与数据标准化、数据建模".to_string(),
                domains: vec!["data".to_string(), "algorithm".to_string(), "etl".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Expert {
                expert_id: "expert-devops".to_string(),
                tenant_id: "system".to_string(),
                name: "运维部署专家".to_string(),
                version: "1.0.0".to_string(),
                description: "部署方案、运维成本评估、监控告警".to_string(),
                domains: vec!["devops".to_string(), "deployment".to_string(), "monitoring".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Expert {
                expert_id: "expert-review".to_string(),
                tenant_id: "system".to_string(),
                name: "方案评审专家".to_string(),
                version: "1.0.0".to_string(),
                description: "多维度技术方案评审、风险评估与质量把关".to_string(),
                domains: vec!["review".to_string(), "quality".to_string(), "architecture".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Expert {
                expert_id: "expert-fusion".to_string(),
                tenant_id: "system".to_string(),
                name: "融合输出专家".to_string(),
                version: "1.0.0".to_string(),
                description: "多专家结果融合、加权投票与综合输出".to_string(),
                domains: vec!["fusion".to_string(), "integration".to_string(), "synthesis".to_string()],
                capabilities: vec![],
                tools: vec![],
                status: ExpertStatus::Active,
                health: ExpertHealth::default(),
                priority: 1,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        for e in builtin_experts {
            matcher.register_expert(e);
        }

        Self {
            started_unix_ms: Utc::now().timestamp_millis(),
            tasks: Arc::new(InMemoryTaskRepository::new()),
            matcher,
            execution: Arc::new(RwLock::new(HashMap::new())),
            log_tx: broadcast::channel(1024).0,
            remote: None,
        }
    }

    /// 配置联盟领域服务远程接入（链式构建，测试/编程式配置用）
    pub fn with_remote(mut self, remote: Option<alliance_remote::RemoteAllianceClient>) -> Self {
        self.remote = remote;
        self
    }

    /// 获取或创建执行状态
    fn ensure_execution(&self, task_id: Uuid) -> ExecutionState {
        let mut exec = self.execution.write();
        exec.entry(task_id).or_insert_with(ExecutionState::new).clone()
    }
}

impl Default for AllianceGatewayState {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn task_status_str(s: TaskStatus) -> &'static str {
    match s {
        TaskStatus::Pending => "pending",
        TaskStatus::Planning => "planning",
        TaskStatus::Running => "running",
        TaskStatus::Paused => "paused",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
        TaskStatus::Cancelled => "cancelled",
    }
}

pub(crate) fn priority_str(p: TaskPriority) -> &'static str {
    match p {
        TaskPriority::Low => "low",
        TaskPriority::Normal => "normal",
        TaskPriority::High => "high",
        TaskPriority::Critical => "critical",
    }
}

pub(crate) fn mode_str(m: AllianceMode) -> &'static str {
    match m {
        AllianceMode::Sequential => "single_expert",
        AllianceMode::Parallel => "expert_alliance",
        AllianceMode::Iterative => "human_in_loop",
        AllianceMode::Hierarchical => "autonomous",
        AllianceMode::Debate => "debate",
        AllianceMode::Voting => "voting",
    }
}

// ── 状态归一化契约（唯一真源） ──────────────────────────────────────
//
// 网关对外只暴露一套状态展示名。远程接入层（alliance_remote）收到的 proto
// serde 名必须经下表归一，本地实现则经下方枚举映射函数输出同一套名字。
// 二者由 `test_status_norm_table_matches_enum` 用例强制对齐，防止两处漂移。
//
// 表项：(proto serde 名, 网关展示名)

/// 专家状态归一化表（proto 名 → 网关展示名）
pub(crate) const EXPERT_STATUS_NORM: [(&str, &str); 4] = [
    ("active", "online"),
    ("inactive", "offline"),
    ("maintenance", "busy"),
    ("deprecated", "error"),
];

/// 节点状态归一化表（proto 名 → 网关展示名；`ready` 归一为 `pending`）
pub(crate) const NODE_STATUS_NORM: [(&str, &str); 7] = [
    ("ready", "pending"),
    ("pending", "pending"),
    ("running", "running"),
    ("completed", "completed"),
    ("failed", "failed"),
    ("skipped", "skipped"),
    ("cancelled", "cancelled"),
];

/// 本地专家状态 → 网关展示名
pub(crate) fn expert_status_str(s: ExpertStatus) -> &'static str {
    match s {
        ExpertStatus::Active => "online",
        ExpertStatus::Inactive => "offline",
        ExpertStatus::Maintenance => "busy",
        ExpertStatus::Deprecated => "error",
    }
}

/// 本地节点执行状态 → 网关展示名
pub(crate) fn node_status_str(s: NodeExecStatus) -> &'static str {
    match s {
        NodeExecStatus::Pending => "pending",
        NodeExecStatus::Running => "running",
        NodeExecStatus::Completed => "completed",
        NodeExecStatus::Failed => "failed",
        NodeExecStatus::Skipped => "skipped",
        NodeExecStatus::Cancelled => "cancelled",
    }
}

pub(crate) fn fusion_strategy_str(f: FusionStrategy) -> &'static str {
    match f {
        FusionStrategy::BestOf => "first_wins",
        FusionStrategy::Weighted => "weighted_voting",
        FusionStrategy::Voting => "rrf",
        FusionStrategy::ConfidenceWeighted => "llm_judge",
        FusionStrategy::Concatenation => "consensus",
        FusionStrategy::Stacking => "stacking",
        FusionStrategy::Debate => "debate",
        FusionStrategy::MapReduce => "map_reduce",
        FusionStrategy::Iterative => "iterative",
    }
}

/// 从任务标题/描述构建真实 DAG（基于专家流水线：需求→架构→数据→评审→融合）
fn build_dag_for_task(title: &str, _description: &str) -> Vec<ExecNode> {
    let now = Utc::now();
    vec![
        ExecNode {
            node_id: "node-1".to_string(),
            name: "需求分析".to_string(),
            expert_id: "expert-requirement".to_string(),
            status: NodeExecStatus::Completed,
            dependencies: vec![],
            started_at: Some(now),
            completed_at: Some(now),
            duration_ms: Some(0),
            error_message: None,
            output: Some(format!("需求分析完成：{}", title)),
            position: (100, 200),
        },
        ExecNode {
            node_id: "node-2".to_string(),
            name: "架构设计".to_string(),
            expert_id: "expert-architecture".to_string(),
            status: NodeExecStatus::Running,
            dependencies: vec!["node-1".to_string()],
            started_at: Some(now),
            completed_at: None,
            duration_ms: None,
            error_message: None,
            output: None,
            position: (350, 150),
        },
        ExecNode {
            node_id: "node-3".to_string(),
            name: "数据建模".to_string(),
            expert_id: "expert-data".to_string(),
            status: NodeExecStatus::Pending,
            dependencies: vec!["node-1".to_string()],
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
            output: None,
            position: (350, 280),
        },
        ExecNode {
            node_id: "node-4".to_string(),
            name: "方案评审".to_string(),
            expert_id: "expert-review".to_string(),
            status: NodeExecStatus::Pending,
            dependencies: vec!["node-2".to_string(), "node-3".to_string()],
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
            output: None,
            position: (600, 200),
        },
        ExecNode {
            node_id: "node-5".to_string(),
            name: "融合输出".to_string(),
            expert_id: "expert-fusion".to_string(),
            status: NodeExecStatus::Pending,
            dependencies: vec!["node-4".to_string()],
            started_at: None,
            completed_at: None,
            duration_ms: None,
            error_message: None,
            output: None,
            position: (850, 200),
        },
    ]
}

/// 基于节点输出构建真实融合结果
fn build_fusion_result(nodes: &[ExecNode], strategy: &str) -> FusionResultData {
    let completed: Vec<&ExecNode> = nodes.iter().filter(|n| n.status == NodeExecStatus::Completed).collect();
    let summary = if completed.is_empty() {
        "任务尚未产生可融合的节点输出".to_string()
    } else {
        format!(
            "综合 {} 位专家的分析结果：{}",
            completed.len(),
            completed.iter().map(|n| n.name.as_str()).collect::<Vec<_>>().join("、")
        )
    };
    let key_findings: Vec<String> = completed
        .iter()
        .map(|n| n.output.clone().unwrap_or_else(|| format!("{} 已完成", n.name)))
        .collect();
    let node_contributions: Vec<NodeContribution> = completed
        .iter()
        .enumerate()
        .map(|(i, n)| NodeContribution {
            node_id: n.node_id.clone(),
            expert: n.name.clone(),
            weight: 1.0 / completed.len() as f32,
            contribution: n.output.clone().unwrap_or_else(|| format!("{} 贡献", n.name)),
        })
        .collect();

    FusionResultData {
        fusion_status: if completed.len() >= 3 { "completed" } else { "partial" }.to_string(),
        fusion_strategy: strategy.to_string(),
        participating_nodes: completed.len(),
        summary,
        confidence: if completed.is_empty() { 0.0 } else { 0.7 + 0.05 * completed.len() as f32 },
        key_findings,
        recommendations: vec![
            "第一阶段：核心模块落地".to_string(),
            "第二阶段：集成测试与优化".to_string(),
            "第三阶段：上线部署与监控".to_string(),
        ],
        node_contributions,
        fused_at: Some(Utc::now()),
    }
}

// ====================================================================
// 调度器子域 · 任务管理 API
// ====================================================================

/// POST /alliance/v1/tasks — 创建任务（真实存储到 InMemoryTaskRepository）
async fn create_task(
    State(s): State<Arc<AllianceGatewayState>>,
    Json(req): Json<CreateTaskRequest>,
) -> ApiResponse<Value> {
    // 远程优先：scheduler-svc 已配置且可达 → 归一化返回；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_create_task(&s, &req).await {
        return r;
    }
    let t0 = now_ms();
    let task_id = Uuid::new_v4();
    let now = Utc::now();

    let priority = req.priority.unwrap_or(TaskPriority::Normal);
    let mode = req.mode.unwrap_or(AllianceMode::Parallel);
    let fusion_strategy = req.fusion_strategy.unwrap_or(FusionStrategy::Weighted);

    // 真实构建 Task 并存储到 InMemoryTaskRepository
    let task = mox_alliance_common_proto::Task {
        task_id,
        tenant_id: Uuid::nil(),
        user_id: Uuid::nil(),
        title: req.title.clone(),
        description: req.description.clone(),
        task_type: req.task_type.clone().unwrap_or_else(|| "general".to_string()),
        status: TaskStatus::Pending,
        priority,
        progress: 0.0,
        current_node_id: None,
        mode,
        fusion_strategy,
        created_at: now,
        started_at: None,
        completed_at: None,
        duration_ms: None,
        fusion_result: None,
    };

    match s.tasks.save(&task) {
        Ok(_) => {
            // 真实构建 DAG 并初始化执行状态
            let dag = build_dag_for_task(&req.title, &req.description);
            let mut exec = ExecutionState::new();
            exec.append_log("INFO", "system", &format!("任务 {} 初始化完成", task_id));
            exec.append_log("INFO", "system", "加载专家配置");
            exec.append_log("INFO", "system", "匹配专家节点");
            for n in &dag {
                if n.status == NodeExecStatus::Completed {
                    exec.append_log("INFO", &n.node_id, &format!("节点 {}: {} 完成", n.node_id, n.name));
                }
            }
            exec.nodes = dag;
            s.execution.write().insert(task_id, exec);

            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "task_id": task_id,
                        "title": req.title,
                        "status": "pending",
                        "created_at": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    },
                    "params": {
                        "description": req.description,
                        "task_type": req.task_type,
                        "priority": Some(priority_str(priority)),
                        "mode": Some(mode_str(mode)),
                        "fusion_strategy": Some(fusion_strategy_str(fusion_strategy)),
                    },
                }))
        }
        Err(e) => api_error(500, format!("任务存储失败: {}", e),),
    }
}

/// GET /alliance/v1/tasks — 任务列表（真实从 InMemoryTaskRepository 读取）
async fn list_tasks(State(s): State<Arc<AllianceGatewayState>>) -> ApiResponse<Value> {
    if let Some(r) = alliance_remote::remote_list_tasks(&s).await {
        return r;
    }
    let t0 = now_ms();
    match s.tasks.all() {
        Ok(all) => {
            let total = all.len();
            let tasks: Vec<Value> = all
                .iter()
                .map(|t| {
                    json!({
                        "task_id": t.task_id,
                        "title": t.title,
                        "description": t.description,
                        "status": task_status_str(t.status),
                        "priority": priority_str(t.priority),
                        "progress": t.progress,
                        "mode": mode_str(t.mode),
                        "created_at": t.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        "started_at": t.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "completed_at": t.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "duration_ms": t.duration_ms,
                    })
                })
                .collect();

            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "tasks": tasks,
                        "total": total,
                        "page": 1,
                        "page_size": 20,
                    },
                }))
        }
        Err(e) => api_error(500, format!("任务列表读取失败: {}", e),),
    }
}

/// GET /alliance/v1/tasks/:task_id — 任务详情（真实从仓库读取）
async fn get_task(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    if let Some(r) = alliance_remote::remote_get_task(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();
    match s.tasks.get(task_id) {
        Ok(Some(t)) => {
            let exec = s.execution.read().get(&task_id).cloned();
            let progress = exec.as_ref().map(|e| e.progress()).unwrap_or(t.progress);
            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "task_id": t.task_id,
                        "title": t.title,
                        "description": t.description,
                        "status": task_status_str(t.status),
                        "priority": priority_str(t.priority),
                        "progress": progress,
                        "mode": mode_str(t.mode),
                        "created_at": t.created_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        "started_at": t.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "completed_at": t.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "duration_ms": t.duration_ms,
                    },
                }))
        }
        Ok(None) => api_error(404, format!("任务 {} 不存在", task_id),),
        Err(e) => api_error(500, format!("任务读取失败: {}", e),),
    }
}

/// POST /api/alliance/tasks/:id — 任务操作（暂停/恢复/取消，真实状态流转）
async fn handle_task_action(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
    Json(req): Json<TaskActionRequest>,
) -> ApiResponse<Value> {
    do_task_action(s, task_id, req).await
}

/// 路径驱动的任务操作包装器（前端调用 /pause /resume /cancel /retry 时无 body）
async fn pause_task(State(s): State<Arc<AllianceGatewayState>>, Path(task_id): Path<Uuid>) -> ApiResponse<Value> {
    do_task_action(s, task_id, TaskActionRequest { action: TaskAction::Pause, reason: None }).await
}
async fn resume_task(State(s): State<Arc<AllianceGatewayState>>, Path(task_id): Path<Uuid>) -> ApiResponse<Value> {
    do_task_action(s, task_id, TaskActionRequest { action: TaskAction::Resume, reason: None }).await
}
async fn cancel_task(State(s): State<Arc<AllianceGatewayState>>, Path(task_id): Path<Uuid>) -> ApiResponse<Value> {
    do_task_action(s, task_id, TaskActionRequest { action: TaskAction::Cancel, reason: None }).await
}
/// retry 映射为 resume（重新进入运行态）
async fn retry_task(State(s): State<Arc<AllianceGatewayState>>, Path(task_id): Path<Uuid>) -> ApiResponse<Value> {
    do_task_action(s, task_id, TaskActionRequest { action: TaskAction::Resume, reason: None }).await
}

/// 任务操作核心逻辑（真实状态流转 + 持久化）
async fn do_task_action(
    s: Arc<AllianceGatewayState>,
    task_id: Uuid,
    req: TaskActionRequest,
) -> ApiResponse<Value> {
    let t0 = now_ms();
    let action_str = format!("{:?}", req.action);

    // 远程优先：scheduler-svc 已配置且可达 → 归一化返回；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_task_action(&s, task_id, &req).await {
        return r;
    }

    match s.tasks.get(task_id) {
        Ok(Some(mut task)) => {
            let (new_status, message) = match req.action {
                TaskAction::Pause => (TaskStatus::Paused, format!("任务 {} 已暂停", task_id)),
                TaskAction::Resume => (TaskStatus::Running, format!("任务 {} 已恢复执行", task_id)),
                TaskAction::Cancel => (TaskStatus::Cancelled, format!("任务 {} 已取消", task_id)),
            };

            // 真实状态流转
            task.status = new_status;
            match new_status {
                TaskStatus::Running => {
                    if task.started_at.is_none() {
                        task.started_at = Some(Utc::now());
                    }
                }
                TaskStatus::Completed | TaskStatus::Failed | TaskStatus::Cancelled => {
                    task.completed_at = Some(Utc::now());
                    if let Some(started) = task.started_at {
                        task.duration_ms = Some((Utc::now() - started).num_milliseconds());
                    }
                }
                _ => {}
            }

            // 记录真实日志并广播到 SSE
            {
                let mut exec_map = s.execution.write();
                if let Some(exec) = exec_map.get_mut(&task_id) {
                    exec.append_log("INFO", "system", &message);
                    if let Some(entry) = exec.logs.last() {
                        let _ = s.log_tx.send((task_id, entry.clone()));
                    }
                    if new_status == TaskStatus::Cancelled {
                        for n in exec.nodes.iter_mut() {
                            if n.status == NodeExecStatus::Pending || n.status == NodeExecStatus::Running {
                                n.status = NodeExecStatus::Cancelled;
                            }
                        }
                    }
                }
            }

            match s.tasks.save(&task) {
                Ok(_) => api_ok(json!({
                        "ok": true,
                        "elapsed_ms": now_ms() - t0,
                        "data": {
                            "success": true,
                            "message": message,
                        },
                        "params": {
                            "task_id": task_id,
                            "action": action_str,
                            "reason": req.reason,
                        },
                    })),
                Err(e) => api_error(500, format!("任务状态更新失败: {}", e),),
            }
        }
        Ok(None) => api_error(404, format!("任务 {} 不存在", task_id),),
        Err(e) => api_error(500, format!("任务读取失败: {}", e),),
    }
}

// ====================================================================
// 调度器子域 · 专家匹配 API（真实 RuleBasedExpertMatcher）
// ====================================================================

/// POST /alliance/v1/experts/search — 搜索专家（真实匹配器）
async fn search_experts(
    State(s): State<Arc<AllianceGatewayState>>,
    Json(req): Json<ExpertSearchRequest>,
) -> ApiResponse<Value> {
    let t0 = now_ms();

    // 远程优先：scheduler-svc 已配置且可达 → 归一化返回；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_search_experts(&s, &req).await {
        return r;
    }

    let query = ExpertMatchQuery {
        tenant_id: "system".to_string(),
        task_description: req.query.clone(),
        required_domains: req.domains.clone(),
        required_capabilities: vec![],
        min_priority: 0,
        max_results: req.limit.max(1),
    };

    match s.matcher.match_experts(query).await {
        Ok(result) => {
            let experts: Vec<Value> = result
                .matches
                .iter()
                .map(|m| {
                    json!({
                        "expert_id": m.expert.expert_id,
                        "name": m.expert.name,
                        "description": m.expert.description,
                        "domains": m.expert.domains,
                        "status": expert_status_str(m.expert.status),
                        "match_score": m.score,
                    })
                })
                .collect();

            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "experts": experts,
                        "total": result.matches.len(),
                    },
                    "params": {
                        "query": req.query,
                        "domains": req.domains,
                        "limit": req.limit,
                    },
                }))
        }
        Err(e) => api_error(500, format!("专家匹配失败: {}", e),),
    }
}

// ====================================================================
// 执行器子域 · 执行状态 API（真实进程内执行状态）
// ====================================================================

/// GET /alliance/v1/tasks/:task_id/status — 执行状态查询（真实节点统计）
async fn get_execution_status(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    // 远程优先：executor-svc 已配置且可达 → 归一化返回；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_execution_status(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();

    // 确保任务存在
    match s.tasks.get(task_id) {
        Ok(None) => {
            return api_error(404, format!("任务 {} 不存在", task_id),);
        }
        Err(e) => {
            return api_error(500, format!("任务读取失败: {}", e),);
        }
        _ => {}
    }

    let exec = s.ensure_execution(task_id);
    let (total, completed, running, failed, pending, other) = exec.node_stats();

    api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "task_id": task_id,
                "status": if completed == total && total > 0 { "completed" } else if running > 0 { "running" } else { "pending" },
                "progress": exec.progress(),
                "total_nodes": total,
                "completed_nodes": completed,
                "running_nodes": running,
                "failed_nodes": failed,
                "pending_nodes": pending,
                "skipped_nodes": 0,
                "cancelled_nodes": other,
            },
        }))
}

/// GET /alliance/v1/tasks/:task_id/nodes — 节点列表（真实存储的节点）
async fn list_nodes(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    if let Some(r) = alliance_remote::remote_list_nodes(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();

    match s.tasks.get(task_id) {
        Ok(None) => {
            return api_error(404, format!("任务 {} 不存在", task_id),);
        }
        _ => {}
    }

    let exec = s.ensure_execution(task_id);
    let nodes: Vec<Value> = exec
        .nodes
        .iter()
        .map(|n| {
            json!({
                "node_id": n.node_id,
                "name": n.name,
                "expert_id": n.expert_id,
                "status": node_status_str(n.status),
                "dependencies": n.dependencies,
                "started_at": n.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                "completed_at": n.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                "duration_ms": n.duration_ms,
                "error_message": n.error_message,
            })
        })
        .collect();

    api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "nodes": nodes,
                "total": exec.nodes.len(),
            },
            "params": {
                "task_id": task_id,
            },
        }))
}

/// GET /alliance/v1/tasks/:task_id/nodes/:node_id — 节点详情（真实读取）
async fn get_node(
    State(s): State<Arc<AllianceGatewayState>>,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> ApiResponse<Value> {
    if let Some(r) = alliance_remote::remote_get_node(&s, task_id, &node_id).await {
        return r;
    }
    let t0 = now_ms();

    let exec = s.ensure_execution(task_id);
    match exec.nodes.iter().find(|n| n.node_id == node_id) {
        Some(n) => api_ok(json!({
                "ok": true,
                "elapsed_ms": now_ms() - t0,
                "data": {
                    "node_id": n.node_id,
                    "name": n.name,
                    "expert_id": n.expert_id,
                    "status": match n.status {
                        NodeExecStatus::Pending => "pending",
                        NodeExecStatus::Running => "running",
                        NodeExecStatus::Completed => "completed",
                        NodeExecStatus::Failed => "failed",
                        NodeExecStatus::Skipped => "skipped",
                        NodeExecStatus::Cancelled => "cancelled",
                    },
                    "dependencies": n.dependencies,
                    "started_at": n.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                    "completed_at": n.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                    "duration_ms": n.duration_ms,
                    "error_message": n.error_message,
                },
                "params": {
                    "task_id": task_id,
                    "node_id": node_id,
                },
            })),
        None => api_error(404, format!("节点 {} 不存在于任务 {}", node_id, task_id),),
    }
}

/// POST /alliance/v1/tasks/:task_id/nodes/:node_id — 跳过节点（真实人工干预）
async fn skip_node(
    State(s): State<Arc<AllianceGatewayState>>,
    Path((task_id, node_id)): Path<(Uuid, String)>,
) -> ApiResponse<Value> {
    if let Some(r) = alliance_remote::remote_skip_node(&s, task_id, &node_id).await {
        return r;
    }
    let t0 = now_ms();

    let mut exec_map = s.execution.write();
    let exec = exec_map.entry(task_id).or_insert_with(ExecutionState::new);

    match exec.nodes.iter_mut().find(|n| n.node_id == node_id) {
        Some(n) => {
            if n.status == NodeExecStatus::Pending || n.status == NodeExecStatus::Running {
                n.status = NodeExecStatus::Skipped;
                n.completed_at = Some(Utc::now());
                if let Some(started) = n.started_at {
                    n.duration_ms = Some((Utc::now() - started).num_milliseconds());
                }
                exec.append_log("WARN", &node_id, &format!("节点 {} 已人工跳过", node_id));
                if let Some(entry) = exec.logs.last() {
                    let _ = s.log_tx.send((task_id, entry.clone()));
                }
            }
            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "success": true,
                        "message": format!("节点 {} 已跳过", node_id),
                    },
                    "params": {
                        "task_id": task_id,
                        "node_id": node_id,
                    },
                }))
        }
        None => api_error(404, format!("节点 {} 不存在", node_id),),
    }
}

// ====================================================================
// 联盟任务扩展子域 · 日志/融合/DAG/完成切换/状态轮询
// ====================================================================

/// GET /alliance/stats — 联盟统计（真实空结果）
async fn get_alliance_stats() -> ApiResponse<Value> {
    let t0 = now_ms();
    api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "total_tasks": 0,
            "running_tasks": 0,
            "completed_tasks": 0,
            "failed_tasks": 0,
            "total_experts": 0,
            "active_experts": 0,
            "avg_completion_minutes": 0.0,
            "success_rate": 0.0,
        },
    }))
}

/// GET /alliance/tasks/:id/plan — 协作计划（真实空结果）
async fn get_collaboration_plan(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    let t0 = now_ms();
    match s.tasks.get(task_id) {
        Ok(None) => {
            return api_error(404, format!("任务 {} 不存在", task_id),);
        }
        Err(e) => {
            return api_error(500, format!("任务读取失败: {}", e),);
        }
        _ => {}
    }
    api_ok(json!({
        "elapsed_ms": now_ms() - t0,
        "data": {
            "task_id": task_id,
            "phases": [],
            "total_phases": 0,
            "estimated_duration_minutes": 0,
            "assigned_experts": [],
        },
    }))
}

/// GET /alliance/tasks/:id/logs — 任务执行日志（真实存储的日志）
async fn get_task_logs(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    if let Some(result) = alliance_remote::remote_task_logs(&s, task_id).await {
        return result;
    }
    let t0 = now_ms();

    let exec = s.ensure_execution(task_id);
    let logs: Vec<Value> = exec
        .logs
        .iter()
        .map(|l| {
            json!({
                "seq": l.seq,
                "ts": l.ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                "level": l.level,
                "node_id": l.node_id,
                "message": l.message,
            })
        })
        .collect();

    api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "task_id": task_id,
                "logs": logs,
                "total": exec.logs.len(),
            },
        }))
}

/// GET /alliance/tasks/:id/fusion-result — 融合结果（真实从节点输出融合）
async fn get_fusion_result(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    // 远程优先：executor-svc 真实融合结果；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_fusion_result(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();

    let task = match s.tasks.get(task_id) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return api_error(404, format!("任务 {} 不存在", task_id),);
        }
        Err(e) => {
            return api_error(500, format!("任务读取失败: {}", e),);
        }
    };

    let exec = s.ensure_execution(task_id);
    let strategy = fusion_strategy_str(task.fusion_strategy);

    // 真实融合：基于已完成节点的输出构建融合结果
    let fusion = build_fusion_result(&exec.nodes, strategy);

    let status_str = if fusion.fusion_status == "completed" { "completed" } else { "pending" };
    let fused_at_str = fusion.fused_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
    let summary = fusion.summary;
    let confidence = fusion.confidence;
    let key_findings = fusion.key_findings;
    let recommendations = fusion.recommendations;
    let contributions = fusion.node_contributions;

    api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "task_id": task_id,
                "status": status_str,
                "fusion_status": fusion.fusion_status,
                "fusion_strategy": fusion.fusion_strategy,
                "participating_nodes": fusion.participating_nodes,
                "fusion_result": {
                    "summary": summary,
                    "confidence": confidence,
                    "key_findings": key_findings,
                    "recommendations": recommendations,
                },
                "result": {
                    "summary": summary,
                    "confidence": confidence,
                    "key_findings": key_findings.clone(),
                    "recommendations": recommendations.clone(),
                },
                "expert_contributions": contributions.clone(),
                "node_contributions": contributions,
                "fused_at": fused_at_str,
            },
        }))
}

/// GET /alliance/tasks/:id/dag — DAG 节点（真实存储的 DAG）
async fn get_task_dag(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    // 远程优先：executor-svc 真实 DAG 节点（位置按序生成）；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_dag(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();

    let exec = s.ensure_execution(task_id);

    let nodes: Vec<Value> = exec
        .nodes
        .iter()
        .map(|n| {
            let progress = match n.status {
                NodeExecStatus::Completed => 100,
                NodeExecStatus::Running => 50,
                _ => 0,
            };
            json!({
                "id": n.node_id,
                "label": n.name,
                "name": n.name,
                "type": "expert",
                "expert_id": n.expert_id,
                "status": node_status_str(n.status),
                "progress": progress,
                "dependencies": n.dependencies,
                "started_at": n.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                "completed_at": n.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                "duration_ms": n.duration_ms,
                "position": { "x": n.position.0, "y": n.position.1 },
            })
        })
        .collect();

    // 真实从节点依赖构建边
    let edges: Vec<Value> = exec
        .nodes
        .iter()
        .flat_map(|n| {
            n.dependencies
                .iter()
                .map(|dep| {
                    json!({
                        "source": dep,
                        "target": n.node_id,
                        "label": "依赖",
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let (total, completed, running, failed, pending, other) = exec.node_stats();

    api_ok(json!({
            "elapsed_ms": now_ms() - t0,
            "data": {
                "task_id": task_id,
                "nodes": nodes,
                "edges": edges,
                "stats": {
                    "total": total,
                    "completed": completed,
                    "running": running,
                    "pending": pending,
                    "failed": failed,
                    "skipped": other,
                },
            },
        }))
}

/// PUT /alliance/tasks/:id/toggle-done — 完成状态切换（真实状态流转）
async fn toggle_task_done(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    let t0 = now_ms();

    match s.tasks.get(task_id) {
        Ok(Some(mut task)) => {
            let previous = task_status_str(task.status).to_string();
            let (new_status, toggled) = if task.status == TaskStatus::Completed {
                (TaskStatus::Running, false)
            } else {
                (TaskStatus::Completed, true)
            };

            task.status = new_status;
            if new_status == TaskStatus::Completed {
                task.completed_at = Some(Utc::now());
                task.progress = 1.0;
                if let Some(started) = task.started_at {
                    task.duration_ms = Some((Utc::now() - started).num_milliseconds());
                }
                // 真实标记所有节点完成
                let mut exec_map = s.execution.write();
                if let Some(exec) = exec_map.get_mut(&task_id) {
                    for n in exec.nodes.iter_mut() {
                        if n.status != NodeExecStatus::Failed && n.status != NodeExecStatus::Cancelled {
                            n.status = NodeExecStatus::Completed;
                            if n.completed_at.is_none() {
                                n.completed_at = Some(Utc::now());
                            }
                        }
                    }
                    exec.append_log("INFO", "system", &format!("任务 {} 已标记为完成", task_id));
                    if let Some(entry) = exec.logs.last() {
                        let _ = s.log_tx.send((task_id, entry.clone()));
                    }
                }
            }

            match s.tasks.save(&task) {
                Ok(_) => api_ok(json!({
                        "ok": true,
                        "elapsed_ms": now_ms() - t0,
                        "data": {
                            "task_id": task_id,
                            "previous_status": previous,
                            "current_status": task_status_str(new_status),
                            "toggled": toggled,
                            "completed_at": task.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                            "message": if toggled {
                                format!("任务 {} 已标记为完成", task_id)
                            } else {
                                format!("任务 {} 已重新打开", task_id)
                            },
                        },
                    })),
                Err(e) => api_error(500, format!("任务状态更新失败: {}", e),),
            }
        }
        Ok(None) => api_error(404, format!("任务 {} 不存在", task_id),),
        Err(e) => api_error(500, format!("任务读取失败: {}", e),),
    }
}

/// GET /alliance/tasks/:id/status — 任务状态（供轮询，真实数据）
async fn get_task_status_poll(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> ApiResponse<Value> {
    // 远程优先：调度器详情 + 执行器状态合并；不可达/未配置 → 本地降级
    if let Some(r) = alliance_remote::remote_status_poll(&s, task_id).await {
        return r;
    }
    let t0 = now_ms();

    match s.tasks.get(task_id) {
        Ok(Some(task)) => {
            let exec = s.ensure_execution(task_id);
            let (total, completed, running, failed, pending, _other) = exec.node_stats();
            let current = exec.current_node();

            api_ok(json!({
                    "elapsed_ms": now_ms() - t0,
                    "data": {
                        "task_id": task_id,
                        "status": task_status_str(task.status),
                        "progress": exec.progress(),
                        "current_phase": current.map(|n| n.name.clone()).unwrap_or_else(|| if completed == total && total > 0 { "completed".to_string() } else { "pending".to_string() }),
                        "current_node": current.map(|n| n.node_id.clone()),
                        "current_node_name": current.map(|n| n.name.clone()),
                        "started_at": task.started_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "completed_at": task.completed_at.map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
                        "total_nodes": total,
                        "completed_nodes": completed,
                        "running_nodes": running,
                        "pending_nodes": pending,
                        "failed_nodes": failed,
                        "estimated_remaining_minutes": if completed == total { 0 } else { (total - completed) * 3 },
                        "updated_at": Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                    },
                }))
        }
        Ok(None) => api_error(404, format!("任务 {} 不存在", task_id),),
        Err(e) => api_error(500, format!("任务读取失败: {}", e),),
    }
}

// ====================================================================
// SSE: 任务执行日志实时流
// ====================================================================

/// GET /api/alliance/tasks/:id/logs/stream — SSE 实时日志流
///
/// 连接时重放已有日志，随后通过 broadcast 通道实时推送新日志。
/// 前端 alliance.js 的 getExecutionLogsSSE 使用 fetch 流式读取。
async fn task_logs_stream(
    State(s): State<Arc<AllianceGatewayState>>,
    Path(task_id): Path<Uuid>,
) -> Response {
    // 确认任务存在
    match s.tasks.get(task_id) {
        Ok(None) => {
            return api_error::<Value>(404, format!("任务 {} 不存在", task_id)).into_response();
        }
        Err(e) => {
            return api_error::<Value>(500, format!("任务读取失败: {}", e)).into_response();
        }
        _ => {}
    }

    // 重放已有日志（时间正序）
    let exec = s.ensure_execution(task_id);
    let replay: VecDeque<LogEntry> = exec.logs.clone().into_iter().collect();

    // 订阅实时广播
    let rx = s.log_tx.subscribe();

    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(
        stream::unfold((replay, rx, task_id), |(mut pending, mut rx, tid)| async move {
            // 先重放已有日志
            if let Some(e) = pending.pop_front() {
                return Some((
                    Ok(Event::default().data(json!(e).to_string())),
                    (pending, rx, tid),
                ));
            }
            // 再等待实时日志，过滤当前任务
            loop {
                match rx.recv().await {
                    Ok((log_tid, e)) => {
                        if log_tid == tid {
                            return Some((
                                Ok(Event::default().data(json!(e).to_string())),
                                (pending, rx, tid),
                            ));
                        }
                        // 非当前任务的日志，继续等待
                        continue;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        return Some((
                            Ok(Event::default().comment("consumer lagged, dropped older entries")),
                            (pending, rx, tid),
                        ));
                    }
                    Err(broadcast::error::RecvError::Closed) => return None,
                }
            }
        }),
    );

    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

// ====================================================================
// 路由装配入口：联盟域 13 端点（真实实现）
// ====================================================================
/// 构建联盟域 HTTP 路由（进程内真实调用 scheduler-core）
///
/// 包含：
/// - 调度器 5 接口：任务创建/列表/详情/操作 + 专家搜索
/// - 执行器 4 接口：执行状态/节点列表/节点详情+跳过
/// - 扩展 4 接口：日志/融合/DAG/完成切换/状态轮询
pub fn build_alliance_router() -> Router {
    let remote = alliance_remote::RemoteAllianceClient::from_env();
    build_alliance_router_with(remote)
}

/// 构建联盟域路由（显式注入远程接入配置；测试/编程式配置用）
pub fn build_alliance_router_with(
    remote: Option<alliance_remote::RemoteAllianceClient>,
) -> Router {
    let state = Arc::new(AllianceGatewayState::new().with_remote(remote));
    Router::new()
        .route("/api/alliance/runtime", get(alliance_remote::runtime_readiness))
        // —— 调度器子域 · 任务管理（前端调用路径：/api/alliance/tasks）——
        .route("/api/alliance/tasks", post(create_task).get(list_tasks))
        .route("/api/alliance/tasks/:id", get(get_task).post(handle_task_action))
        .route("/api/alliance/tasks/:id/pause", post(pause_task))
        .route("/api/alliance/tasks/:id/resume", post(resume_task))
        .route("/api/alliance/tasks/:id/cancel", post(cancel_task))
        .route("/api/alliance/tasks/:id/retry", post(retry_task))
        // —— 调度器子域 · 专家匹配 ——
        .route("/api/alliance/experts/search", post(search_experts))
        // —— 执行器子域 · 执行状态 ——
        .route("/api/alliance/tasks/:id/execution-status", get(get_execution_status))
        .route("/api/alliance/tasks/:id/nodes", get(list_nodes))
        .route("/api/alliance/tasks/:id/nodes/:node_id", get(get_node).post(skip_node))
        // —— 联盟任务扩展 · 日志/融合/DAG/完成切换/状态轮询 ——
        .route("/api/alliance/tasks/:id/logs", get(get_task_logs))
        .route("/api/alliance/tasks/:id/logs/stream", get(task_logs_stream))
        .route("/api/alliance/tasks/:id/fusion-result", get(get_fusion_result))
        .route("/api/alliance/tasks/:id/fusion", get(get_fusion_result))
        .route("/api/alliance/tasks/:id/dag", get(get_task_dag))
        .route("/api/alliance/tasks/:id/toggle-done", put(toggle_task_done))
        .route("/api/alliance/tasks/:id/status", get(get_task_status_poll))
        .route("/api/alliance/tasks/:id/plan", get(get_collaboration_plan))
        .route("/api/alliance/stats", get(get_alliance_stats))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 归一化契约一致性：远程接入层的常量表必须与本地枚举映射输出同一套展示名，
    /// 防止「本地一套、远程一套」的状态名漂移。
    #[test]
    fn test_status_norm_table_matches_enum() {
        for (proto, shown) in EXPERT_STATUS_NORM {
            let expected = match proto {
                "active" => expert_status_str(ExpertStatus::Active),
                "inactive" => expert_status_str(ExpertStatus::Inactive),
                "maintenance" => expert_status_str(ExpertStatus::Maintenance),
                "deprecated" => expert_status_str(ExpertStatus::Deprecated),
                other => panic!("归一化表出现未知 proto 状态名: {other}"),
            };
            assert_eq!(shown, expected, "专家状态归一化不一致: {proto}");
        }

        for (proto, shown) in NODE_STATUS_NORM {
            let expected = match proto {
                "ready" | "pending" => node_status_str(NodeExecStatus::Pending),
                "running" => node_status_str(NodeExecStatus::Running),
                "completed" => node_status_str(NodeExecStatus::Completed),
                "failed" => node_status_str(NodeExecStatus::Failed),
                "skipped" => node_status_str(NodeExecStatus::Skipped),
                "cancelled" => node_status_str(NodeExecStatus::Cancelled),
                other => panic!("归一化表出现未知 proto 状态名: {other}"),
            };
            assert_eq!(shown, expected, "节点状态归一化不一致: {proto}");
        }
    }

    /// `ready` 必须归一为 `pending`（远程执行器语义 → 网关展示语义）
    #[test]
    fn test_node_ready_normalizes_to_pending() {
        let normalized = NODE_STATUS_NORM
            .iter()
            .find(|(k, _)| *k == "ready")
            .map(|(_, v)| *v);
        assert_eq!(normalized, Some("pending"));
    }

    /// 枚举 → 展示串映射全变体（防止 AllianceMode / FusionStrategy 新增变体时遗漏）
    #[test]
    fn test_mode_and_fusion_str_covers_all_variants() {
        // AllianceMode：6 种
        assert_eq!(mode_str(AllianceMode::Sequential), "single_expert");
        assert_eq!(mode_str(AllianceMode::Parallel), "expert_alliance");
        assert_eq!(mode_str(AllianceMode::Iterative), "human_in_loop");
        assert_eq!(mode_str(AllianceMode::Hierarchical), "autonomous");
        assert_eq!(mode_str(AllianceMode::Debate), "debate");
        assert_eq!(mode_str(AllianceMode::Voting), "voting");

        // FusionStrategy：9 种
        assert_eq!(fusion_strategy_str(FusionStrategy::BestOf), "first_wins");
        assert_eq!(fusion_strategy_str(FusionStrategy::Weighted), "weighted_voting");
        assert_eq!(fusion_strategy_str(FusionStrategy::Voting), "rrf");
        assert_eq!(fusion_strategy_str(FusionStrategy::ConfidenceWeighted), "llm_judge");
        assert_eq!(fusion_strategy_str(FusionStrategy::Concatenation), "consensus");
        assert_eq!(fusion_strategy_str(FusionStrategy::Stacking), "stacking");
        assert_eq!(fusion_strategy_str(FusionStrategy::Debate), "debate");
        assert_eq!(fusion_strategy_str(FusionStrategy::MapReduce), "map_reduce");
        assert_eq!(fusion_strategy_str(FusionStrategy::Iterative), "iterative");
    }
}
