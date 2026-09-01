// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 端到端集成测试：专家匹配 → 计划生成 → DAG 执行 → 结果验证

use std::sync::Arc;
use std::time::Duration;

use mox_alliance_common_proto::{
    AllianceMode, AllianceResult, Expert, FusionStrategy, NodeStatus, TaskPriority,
    TaskStatus,
};
use mox_alliance_executor_core::{DagEngineImpl, MockNodeExecutor, MockExecutorConfig};
use mox_alliance_executor_proto::{DagEngine, ExecutionOptions, NodeExecutor};
use mox_alliance_scheduler_core::{RuleBasedExpertMatcher, SimplePlanGenerator, TaskSchedulerImpl};
use mox_alliance_scheduler_proto::{
    ExpertMatchQuery, ExpertMatcher, PlanGenerationRequest, TaskScheduler, TaskSubmitRequest,
};
use tokio::sync::mpsc;
use uuid::Uuid;

/// 创建测试用的专家列表
fn create_test_experts() -> Vec<Expert> {
    let mut e1 = Expert::new_system(
        "研究专家 Alpha".to_string(),
        "擅长深度研究和文献综述".to_string(),
    );
    e1.domains = vec!["research".to_string(), "analysis".to_string()];
    e1.priority = 1;

    let mut e2 = Expert::new_system(
        "数据专家 Beta".to_string(),
        "擅长数据分析和统计建模".to_string(),
    );
    e2.domains = vec!["data".to_string(), "analysis".to_string()];
    e2.priority = 2;

    let mut e3 = Expert::new_system(
        "写作专家 Gamma".to_string(),
        "擅长报告撰写和内容编辑".to_string(),
    );
    e3.domains = vec!["writing".to_string(), "editing".to_string()];
    e3.priority = 3;

    vec![e1, e2, e3]
}

/// 测试1：任务提交与查询
#[tokio::test]
async fn test_e2e_task_submission() -> AllianceResult<()> {
    let matcher = Arc::new(RuleBasedExpertMatcher::new());
    let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel();
    let config = mox_alliance_scheduler_proto::types::SchedulerConfig::default();
    let scheduler = Arc::new(TaskSchedulerImpl::new(config, matcher.clone(), dispatch_tx));

    let title = "E2E 测试任务".to_string();
    let resp = scheduler
        .submit_task(TaskSubmitRequest {
            tenant_id: Uuid::nil(),
            user_id: Uuid::nil(),
            title: title.clone(),
            description: "测试描述".to_string(),
            task_type: Some("research".to_string()),
            priority: Some(TaskPriority::Normal),
            mode: Some(AllianceMode::Parallel),
            fusion_strategy: Some(FusionStrategy::Weighted),
        })
        .await?;

    assert_eq!(resp.task.title, title);
    assert!(matches!(resp.task.status, TaskStatus::Running | TaskStatus::Planning),
        "Task should be in Running or Planning state after submission, got {:?}", resp.task.status);

    let task = scheduler.get_task(resp.task.task_id, Uuid::nil()).await?;
    assert_eq!(task.title, title);

    Ok(())
}

/// 测试2：专家匹配
#[tokio::test]
async fn test_e2e_expert_matching() -> AllianceResult<()> {
    let matcher = RuleBasedExpertMatcher::new();
    matcher.register_experts(create_test_experts());

    let result = matcher
        .match_experts(ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "数据分析与可视化".to_string(),
            required_domains: vec!["data".to_string()],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 5,
        })
        .await?;

    assert!(result.total_available > 0);
    assert!(!result.matches.is_empty());
    for m in &result.matches {
        assert!(m.score > 0.0);
    }
    Ok(())
}

/// 测试3：计划生成
#[tokio::test]
async fn test_e2e_plan_generation() -> AllianceResult<()> {
    let matcher = RuleBasedExpertMatcher::new();
    matcher.register_experts(create_test_experts());
    let planner = SimplePlanGenerator::new();

    let match_result = matcher
        .match_experts(ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "研究分析".to_string(),
            required_domains: vec!["research".to_string()],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 3,
        })
        .await?;

    let task_id = Uuid::new_v4();
    let req = PlanGenerationRequest {
        task_id,
        tenant_id: Uuid::nil(),
        task_description: "测试计划描述".to_string(),
        preferred_mode: Some(AllianceMode::Parallel),
        preferred_experts: vec![],
        constraints: serde_json::json!({}),
        fusion_strategy: FusionStrategy::Weighted,
    };

    let plan = planner.generate(&req, &match_result.matches)?;
    assert_eq!(plan.mode, AllianceMode::Parallel);
    assert_eq!(plan.nodes.len(), match_result.matches.len());
    assert!(plan.validate().is_ok());

    Ok(())
}

/// 测试4：DAG 执行引擎（2节点串行）
#[tokio::test]
async fn test_e2e_dag_execution_sequential() -> AllianceResult<()> {
    let mock_config = MockExecutorConfig {
        delay_ms: 10,
        success_rate: 1.0,
        generate_output: true,
    };
    let node_executor: Arc<dyn NodeExecutor> = Arc::new(MockNodeExecutor::new(mock_config));
    let exec_config = mox_alliance_executor_proto::types::ExecutorConfig::default();
    let engine = DagEngineImpl::spawn(exec_config, node_executor);

    let matcher = RuleBasedExpertMatcher::new();
    matcher.register_experts(create_test_experts());
    let planner = SimplePlanGenerator::new();
    let task_id = Uuid::new_v4();

    let match_result = matcher
        .match_experts(ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "研究分析".to_string(),
            required_domains: vec!["research".to_string(), "data".to_string()],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 2,
        })
        .await?;

    let req = PlanGenerationRequest {
        task_id,
        tenant_id: Uuid::nil(),
        task_description: "串行模式测试".to_string(),
        preferred_mode: Some(AllianceMode::Sequential),
        preferred_experts: vec![],
        constraints: serde_json::json!({}),
        fusion_strategy: FusionStrategy::Weighted,
    };
    let plan = planner.generate(&req, &match_result.matches)?;
    assert_eq!(plan.nodes.len(), 2);

    let task = mox_alliance_common_proto::Task {
        task_id,
        tenant_id: Uuid::nil(),
        user_id: Uuid::nil(),
        title: "DAG执行测试".to_string(),
        description: "测试".to_string(),
        task_type: "test".to_string(),
        status: TaskStatus::Pending,
        priority: TaskPriority::Normal,
        progress: 0.0,
        current_node_id: None,
        mode: AllianceMode::Sequential,
        fusion_strategy: FusionStrategy::Weighted,
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        fusion_result: None,
    };

    engine
        .start_execution(&task, plan.clone(), ExecutionOptions::default())
        .await?;

    // 等待完成（最多 3 秒）
    let mut completed = false;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let status = engine.get_execution_status(task_id, Uuid::nil()).await?;
        if status.completed_nodes == status.total_nodes && status.total_nodes > 0 {
            completed = true;
            break;
        }
    }
    assert!(completed, "串行 DAG 应在 3 秒内完成");

    let status = engine.get_execution_status(task_id, Uuid::nil()).await?;
    assert_eq!(status.total_nodes, 2);
    assert_eq!(status.completed_nodes, 2);
    assert_eq!(status.failed_nodes, 0);
    assert!(status.progress >= 0.99);

    let nodes = engine.get_nodes(task_id, Uuid::nil()).await?;
    for node in &nodes {
        assert_eq!(node.status, NodeStatus::Completed);
        assert!(node.completed_at.is_some());
        assert!(node.duration_ms.is_some());
    }

    Ok(())
}

/// 测试5：完整链路（匹配 → 计划 → 执行 → 验证）
#[tokio::test]
async fn test_e2e_full_pipeline() -> AllianceResult<()> {
    let matcher = Arc::new(RuleBasedExpertMatcher::new());
    matcher.register_experts(create_test_experts());
    let planner = Arc::new(SimplePlanGenerator::new());
    let (dispatch_tx, _dispatch_rx) = mpsc::unbounded_channel();
    let sched_config = mox_alliance_scheduler_proto::types::SchedulerConfig::default();
    let scheduler = Arc::new(TaskSchedulerImpl::new(
        sched_config,
        matcher.clone(),
        dispatch_tx,
    ));

    let mock_config = MockExecutorConfig {
        delay_ms: 10,
        success_rate: 1.0,
        generate_output: true,
    };
    let node_executor: Arc<dyn NodeExecutor> = Arc::new(MockNodeExecutor::new(mock_config));
    let exec_config = mox_alliance_executor_proto::types::ExecutorConfig::default();
    let engine = DagEngineImpl::spawn(exec_config, node_executor);

    let submit_resp = scheduler
        .submit_task(TaskSubmitRequest {
            tenant_id: Uuid::nil(),
            user_id: Uuid::nil(),
            title: "E2E 完整链路".to_string(),
            description: "从匹配到执行的完整链路测试".to_string(),
            task_type: Some("research".to_string()),
            priority: Some(TaskPriority::High),
            mode: Some(AllianceMode::Parallel),
            fusion_strategy: Some(FusionStrategy::BestOf),
        })
        .await?;
    let task_id = submit_resp.task.task_id;

    let match_result = matcher
        .match_experts(ExpertMatchQuery {
            tenant_id: "system".to_string(),
            task_description: "需要研究、数据分析能力".to_string(),
            required_domains: vec!["research".to_string(), "data".to_string()],
            required_capabilities: vec![],
            min_priority: 1,
            max_results: 3,
        })
        .await?;
    assert!(match_result.matches.len() >= 2);

    let plan_req = PlanGenerationRequest {
        task_id,
        tenant_id: Uuid::nil(),
        task_description: "完整链路测试计划".to_string(),
        preferred_mode: Some(AllianceMode::Parallel),
        preferred_experts: vec![],
        constraints: serde_json::json!({}),
        fusion_strategy: FusionStrategy::Weighted,
    };
    let plan = planner.generate(&plan_req, &match_result.matches)?;
    assert!(plan.validate().is_ok());
    assert_eq!(plan.nodes.len(), match_result.matches.len());

    let task = scheduler.get_task(task_id, Uuid::nil()).await?;
    engine
        .start_execution(&task, plan, ExecutionOptions::default())
        .await?;

    let mut completed = false;
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let status = engine.get_execution_status(task_id, Uuid::nil()).await?;
        if status.completed_nodes == status.total_nodes && status.total_nodes > 0 {
            completed = true;
            break;
        }
    }
    assert!(completed, "完整链路应在 3 秒内完成");

    let status = engine.get_execution_status(task_id, Uuid::nil()).await?;
    assert_eq!(status.total_nodes, match_result.matches.len());
    assert_eq!(status.completed_nodes, match_result.matches.len());
    assert_eq!(status.failed_nodes, 0);
    assert_eq!(status.running_nodes, 0);
    assert!(status.progress > 0.99);

    let nodes = engine.get_nodes(task_id, Uuid::nil()).await?;
    assert_eq!(nodes.len(), match_result.matches.len());
    for node in &nodes {
        assert_eq!(node.status, NodeStatus::Completed);
    }

    Ok(())
}
