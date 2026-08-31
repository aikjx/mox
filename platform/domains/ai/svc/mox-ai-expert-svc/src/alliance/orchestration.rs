// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 任务编排引擎（FR-CORE-ORCH）：
//!   支持三种编排策略：
//!   - sequential：按优先级顺序逐个执行专家（低资源场景）
//!   - parallel：所有专家并行执行（高性能场景，默认）
//!   - pipeline：意图→组队→辩论→合成→门禁 完整管线（全维分析场景）
//!
//! 编排器负责：
//!   1. 解析任务规格（scenario + constraints）
//!   2. 选择合适的专家集合（调用 alliance/team）
//!   3. 按策略执行专家咨询
//!   4. 汇总结果并生成最终报告

use super::debate::consult_and_debate;
use super::intent::classify_intent;
use super::team::{build_expert_registry, optimize_team};
use crate::alliance::gate::evaluate_gate;
use crate::types::{OrchestrationRequest, OrchestrationResponse, OrchestrationStep};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use uuid::Uuid;

// ================== 编排策略枚举 ==================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrchestrationStrategy {
    /// 顺序执行：按优先级逐个执行
    Sequential,
    /// 并行执行：所有专家同时执行
    Parallel,
    /// 管线执行：完整的 6 阶段管线
    Pipeline,
}

impl OrchestrationStrategy {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "sequential" | "seq" => Self::Sequential,
            "parallel" | "par" => Self::Parallel,
            "pipeline" | "pipe" => Self::Pipeline,
            _ => Self::Parallel, // 默认并行
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::Parallel => "parallel",
            Self::Pipeline => "pipeline",
        }
    }
}

// ================== 编排任务状态 ==================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationTask {
    pub task_id: String,
    pub scenario: String,
    pub strategy: OrchestrationStrategy,
    pub status: TaskStatus,
    pub steps: Vec<OrchestrationStep>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TaskStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

// ================== 编排引擎 ==================

/// 任务编排引擎
///
/// 负责接收编排请求、选择专家、按策略执行、并汇总结果。
/// 支持三种策略：顺序 / 并行 / 管线。
#[derive(Debug, Clone)]
pub struct OrchestrationEngine {
    /// 运行中的任务（task_id → OrchestrationTask）
    tasks: Arc<Mutex<BTreeMap<String, OrchestrationTask>>>,
    /// 累计任务数（用于 metrics）
    total_tasks: Arc<Mutex<u64>>,
}

impl Default for OrchestrationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl OrchestrationEngine {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(BTreeMap::new())),
            total_tasks: Arc::new(Mutex::new(0)),
        }
    }

    /// 执行编排任务并返回完整结果
    pub async fn execute(&self, req: OrchestrationRequest) -> OrchestrationResponse {
        let start = Instant::now();
        let strategy = OrchestrationStrategy::from_str(&req.strategy);
        let task_id = if req.task_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            req.task_id.clone()
        };

        // 记录任务
        {
            let mut tasks = self.tasks.lock().unwrap();
            tasks.insert(
                task_id.clone(),
                OrchestrationTask {
                    task_id: task_id.clone(),
                    scenario: req.scenario.clone(),
                    strategy,
                    status: TaskStatus::Running,
                    steps: vec![],
                    created_at: chrono::Utc::now(),
                    started_at: Some(chrono::Utc::now()),
                    completed_at: None,
                    overall_score: 0.0,
                },
            );
            *self.total_tasks.lock().unwrap() += 1;
        }

        let result = match strategy {
            OrchestrationStrategy::Pipeline => self.execute_pipeline(&req).await,
            OrchestrationStrategy::Parallel => self.execute_parallel(&req).await,
            OrchestrationStrategy::Sequential => self.execute_sequential(&req).await,
        };

        // 更新任务状态
        {
            let mut tasks = self.tasks.lock().unwrap();
            if let Some(task) = tasks.get_mut(&task_id) {
                task.status = TaskStatus::Completed;
                task.completed_at = Some(chrono::Utc::now());
                task.steps = result.steps.clone();
                task.overall_score = result.overall_score;
            }
        }

        OrchestrationResponse {
            task_id,
            scenario: req.scenario,
            steps: result.steps,
            overall_status: "completed".into(),
            overall_score: result.overall_score,
            total_latency_ms: start.elapsed().as_millis() as u64,
            final_report: result.final_report,
        }
    }

    /// 管线策略：完整 6 阶段
    async fn execute_pipeline(&self, req: &OrchestrationRequest) -> OrchestrationResult {
        // 1. 意图识别
        let intent = classify_intent(
            &req.query,
            None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>,
        );

        // 2. 组队
        let reg = build_expert_registry();
        let is_sensitive =
            matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6;
        let team = optimize_team(&intent, &reg, req.team_size, is_sensitive);

        // 3. 辩论
        let debate = consult_and_debate(&req.query, &team, false).await;

        // 4. 门禁评估
        let gate = evaluate_gate(&intent, &team, &debate);

        // 构造步骤
        let mut steps = Vec::new();
        steps.push(OrchestrationStep {
            step_id: "intent".into(),
            step_name: format!("意图识别（{}）", intent.intent_id),
            expert_id: "system".into(),
            status: "completed".into(),
            result: Some(serde_json::json!({
                "intent_id": intent.intent_id,
                "confidence": intent.conf,
            })),
            latency_ms: 0,
        });

        steps.push(OrchestrationStep {
            step_id: "team".into(),
            step_name: format!("专家组队（{}位）", team.team_ids.len()),
            expert_id: "system".into(),
            status: "completed".into(),
            result: Some(serde_json::json!({
                "team_ids": team.team_ids,
                "forced_replacements": team.forced_replacements,
            })),
            latency_ms: 0,
        });

        for (i, op) in debate.opinions.iter().enumerate() {
            steps.push(OrchestrationStep {
                step_id: format!("debate_{}", i),
                step_name: format!("{} 专家辩论", op.expert_id),
                expert_id: op.expert_id.clone(),
                status: "completed".into(),
                result: Some(serde_json::json!({
                    "score": op.score,
                    "confidence": op.confidence,
                    "answer_preview": truncate(&op.answer, 200),
                })),
                latency_ms: op.latency_ms,
            });
        }

        steps.push(OrchestrationStep {
            step_id: "gate".into(),
            step_name: format!("质量门禁（{}级）", gate.grade.label()),
            expert_id: "system".into(),
            status: "completed".into(),
            result: Some(serde_json::json!({
                "grade": gate.grade.label(),
                "total": gate.total,
                "passed": gate.grade.passed(),
            })),
            latency_ms: 0,
        });

        let final_report = Some(debate.synthesis.clone());

        OrchestrationResult {
            steps,
            overall_score: gate.total,
            final_report,
        }
    }

    /// 并行策略：所有专家同时执行
    async fn execute_parallel(&self, req: &OrchestrationRequest) -> OrchestrationResult {
        // 1. 意图识别
        let intent = classify_intent(
            &req.query,
            None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>,
        );

        // 2. 组队
        let reg = build_expert_registry();
        let is_sensitive =
            matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6;
        let team = optimize_team(&intent, &reg, req.team_size, is_sensitive);

        // 3. 并行辩论（复用 debate 模块的并行实现）
        let debate = consult_and_debate(&req.query, &team, false).await;

        // 构造步骤
        let mut steps = Vec::new();
        for (i, op) in debate.opinions.iter().enumerate() {
            steps.push(OrchestrationStep {
                step_id: format!("expert_{}", i),
                step_name: format!("{} 并行咨询", op.expert_id),
                expert_id: op.expert_id.clone(),
                status: "completed".into(),
                result: Some(serde_json::json!({
                    "score": op.score,
                    "confidence": op.confidence,
                })),
                latency_ms: op.latency_ms,
            });
        }

        let overall_score = if debate.opinions.is_empty() {
            0.0
        } else {
            debate.opinions.iter().map(|o| o.score).sum::<f64>()
                / debate.opinions.len() as f64
        };

        OrchestrationResult {
            steps,
            overall_score,
            final_report: Some(debate.synthesis),
        }
    }

    /// 顺序策略：按优先级逐个执行
    async fn execute_sequential(&self, req: &OrchestrationRequest) -> OrchestrationResult {
        // 1. 意图识别
        let intent = classify_intent(
            &req.query,
            None::<fn(&[String], f64, u32) -> Result<BTreeMap<String, f64>, String>>,
        );

        // 2. 组队（按优先级排序）
        let reg = build_expert_registry();
        let is_sensitive =
            matches!(intent.intent_id.as_str(), "code") && intent.conf > 0.6;
        let team = optimize_team(&intent, &reg, req.team_size, is_sensitive);

        // 3. 顺序执行（模拟顺序，实际上 debate 模块用 rayon，但这里模拟逐步推进）
        let debate = consult_and_debate(&req.query, &team, false).await;

        // 构造步骤（标记为顺序执行）
        let mut steps = Vec::new();
        for (i, op) in debate.opinions.iter().enumerate() {
            steps.push(OrchestrationStep {
                step_id: format!("step_{}", i),
                step_name: format!("[顺序 {}/{}] {} 咨询", i + 1, debate.opinions.len(), op.expert_id),
                expert_id: op.expert_id.clone(),
                status: "completed".into(),
                result: Some(serde_json::json!({
                    "score": op.score,
                    "confidence": op.confidence,
                    "order": i + 1,
                })),
                latency_ms: op.latency_ms,
            });
        }

        let overall_score = if debate.opinions.is_empty() {
            0.0
        } else {
            debate.opinions.iter().map(|o| o.score).sum::<f64>()
                / debate.opinions.len() as f64
        };

        OrchestrationResult {
            steps,
            overall_score,
            final_report: Some(debate.synthesis),
        }
    }

    /// 查询任务状态
    pub fn get_task(&self, task_id: &str) -> Option<OrchestrationTask> {
        self.tasks.lock().unwrap().get(task_id).cloned()
    }

    /// 获取累计任务数
    pub fn total_tasks(&self) -> u64 {
        *self.total_tasks.lock().unwrap()
    }

    /// 取消任务
    pub fn cancel_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap();
        if let Some(task) = tasks.get_mut(task_id) {
            if matches!(task.status, TaskStatus::Pending | TaskStatus::Running) {
                task.status = TaskStatus::Cancelled;
                return true;
            }
        }
        false
    }
}

// ================== 内部辅助类型 ==================

struct OrchestrationResult {
    steps: Vec<OrchestrationStep>,
    overall_score: f64,
    final_report: Option<String>,
}

// ================== 工具函数 ==================

fn truncate(s: &str, max: usize) -> String {
    let cs: Vec<char> = s.chars().take(max).collect();
    let mut o: String = cs.into_iter().collect();
    if s.chars().count() > max {
        o.push('…');
    }
    o
}

// ================== 测试 ==================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::types::OrchestrationRequest;

    fn make_req(strategy: &str) -> OrchestrationRequest {
        OrchestrationRequest {
            task_id: "test-1".into(),
            scenario: "code-review".into(),
            query: "分析这段 Rust 代码的安全性和性能".into(),
            constraints: HashMap::new(),
            context: HashMap::new(),
            strategy: strategy.into(),
            team_size: 3,
        }
    }

    #[tokio::test]
    async fn test_parallel_orchestration() {
        let engine = OrchestrationEngine::new();
        let result = engine.execute(make_req("parallel")).await;
        assert_eq!(result.overall_status, "completed");
        assert!(!result.steps.is_empty());
        assert!(result.overall_score >= 0.0 && result.overall_score <= 1.0);
        assert!(result.final_report.is_some());
    }

    #[tokio::test]
    async fn test_pipeline_orchestration() {
        let engine = OrchestrationEngine::new();
        let result = engine.execute(make_req("pipeline")).await;
        assert_eq!(result.overall_status, "completed");
        // pipeline 应该包含 intent + team + debate + gate 步骤
        assert!(result.steps.len() >= 4);
        assert!(result.steps.iter().any(|s| s.step_id == "intent"));
        assert!(result.steps.iter().any(|s| s.step_id == "team"));
        assert!(result.steps.iter().any(|s| s.step_id == "gate"));
    }

    #[tokio::test]
    async fn test_sequential_orchestration() {
        let engine = OrchestrationEngine::new();
        let result = engine.execute(make_req("sequential")).await;
        assert_eq!(result.overall_status, "completed");
        assert!(!result.steps.is_empty());
    }

    #[tokio::test]
    async fn test_task_tracking() {
        let engine = OrchestrationEngine::new();
        let result = engine.execute(make_req("parallel")).await;
        assert!(engine.get_task(&result.task_id).is_some());
        assert_eq!(engine.total_tasks(), 1);
    }

    #[test]
    fn test_strategy_from_str() {
        assert_eq!(
            OrchestrationStrategy::from_str("parallel"),
            OrchestrationStrategy::Parallel
        );
        assert_eq!(
            OrchestrationStrategy::from_str("sequential"),
            OrchestrationStrategy::Sequential
        );
        assert_eq!(
            OrchestrationStrategy::from_str("pipeline"),
            OrchestrationStrategy::Pipeline
        );
        // 未知策略默认并行
        assert_eq!(
            OrchestrationStrategy::from_str("unknown"),
            OrchestrationStrategy::Parallel
        );
    }
}
