// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器专用类型

use serde::{Deserialize, Serialize};

use mox_alliance_common_proto::{AllianceMode, FusionStrategy, TaskPriority};

/// 调度器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// 最大并发任务数
    pub max_concurrent_tasks: usize,
    /// 任务队列容量
    pub queue_capacity: usize,
    /// 默认任务优先级
    pub default_priority: TaskPriority,
    /// 默认协作模式
    pub default_mode: AllianceMode,
    /// 默认融合策略
    pub default_fusion_strategy: FusionStrategy,
    /// 计划生成超时（毫秒）
    pub plan_generation_timeout_ms: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 100,
            queue_capacity: 1000,
            default_priority: TaskPriority::Normal,
            default_mode: AllianceMode::Parallel,
            default_fusion_strategy: FusionStrategy::Weighted,
            plan_generation_timeout_ms: 30_000,
        }
    }
}

/// 计划生成请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGenerationRequest {
    pub task_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub task_description: String,
    pub preferred_mode: Option<AllianceMode>,
    pub preferred_experts: Vec<String>,
    pub constraints: serde_json::Value,
    /// 任务级融合策略（不再被计划生成器硬编码覆盖）
    pub fusion_strategy: FusionStrategy,
}

/// 计划生成响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGenerationResponse {
    pub task_id: uuid::Uuid,
    pub plan: mox_alliance_common_proto::CollaborationPlan,
    pub matched_experts: Vec<MatchedExpertSummary>,
    pub generation_time_ms: u64,
}

/// 匹配专家摘要（用于计划生成响应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedExpertSummary {
    pub expert_id: String,
    pub name: String,
    pub match_score: f64,
    pub reason: String,
}
