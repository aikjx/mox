// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! DAG 执行引擎 trait 抽象

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceResult, CollaborationPlan, Node, Task};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::ExecutorConfig;

/// 执行选项
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// 最大重试次数
    pub max_retries: u32,
    /// 节点超时（毫秒）
    pub node_timeout_ms: u64,
    /// 是否在第一个节点失败时终止整个任务
    pub fail_fast: bool,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_retries: 3,
            node_timeout_ms: 300_000,
            fail_fast: false,
        }
    }
}

/// DAG 执行引擎 trait
///
/// 负责执行协作计划（DAG），管理节点调度、依赖解析、状态追踪。
/// 执行引擎是无状态的，所有状态外部化（存储层提供）。
#[async_trait]
pub trait DagEngine: Send + Sync {
    /// 开始执行一个计划
    async fn start_execution(
        &self,
        task: &Task,
        plan: CollaborationPlan,
        options: ExecutionOptions,
    ) -> AllianceResult<()>;

    /// 暂停执行
    async fn pause_execution(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 恢复执行
    async fn resume_execution(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<()>;

    /// 取消执行
    async fn cancel_execution(&self, task_id: Uuid, tenant_id: Uuid, reason: Option<String>) -> AllianceResult<()>;

    /// 获取执行状态
    async fn get_execution_status(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<ExecutionStatus>;

    /// 获取节点列表
    async fn get_nodes(&self, task_id: Uuid, tenant_id: Uuid) -> AllianceResult<Vec<Node>>;

    /// 获取单个节点
    async fn get_node(&self, task_id: Uuid, node_id: &str, tenant_id: Uuid) -> AllianceResult<Node>;

    /// 跳过某个节点（人工干预）
    async fn skip_node(&self, task_id: Uuid, node_id: &str, tenant_id: Uuid, reason: Option<String>) -> AllianceResult<()>;

    /// 获取执行引擎配置
    fn config(&self) -> &ExecutorConfig;
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    pub task_id: Uuid,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub running_nodes: usize,
    pub failed_nodes: usize,
    pub pending_nodes: usize,
    pub skipped_nodes: usize,
    pub progress: f32,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub estimated_remaining_ms: Option<u64>,
}
