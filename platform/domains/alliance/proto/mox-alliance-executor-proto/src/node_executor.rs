// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 节点执行器 trait 抽象

use async_trait::async_trait;
use mox_alliance_common_proto::{AllianceResult, Node};
use serde::{Deserialize, Serialize};

/// 节点执行请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionRequest {
    pub task_id: uuid::Uuid,
    pub node: Node,
    pub input_data: Option<serde_json::Value>,
    pub context: Option<serde_json::Value>,
    pub tenant_id: String,
}

/// 节点执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub task_id: uuid::Uuid,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub duration_ms: u64,
    pub retry_count: u32,
}

/// 节点执行器 trait
///
/// 负责执行单个 DAG 节点（调用专家 + 工具执行）。
/// 这是一个可插拔的抽象，可以有多种实现：
/// - 本地执行（进程内调用专家）
/// - 远程执行（gRPC 调用 agent-svc）
/// - Mock 执行（测试用）
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// 执行节点
    async fn execute_node(&self, request: NodeExecutionRequest) -> AllianceResult<NodeExecutionResult>;

    /// 执行器名称（用于日志和监控）
    fn executor_name(&self) -> &str;

    /// 检查执行器是否健康
    async fn is_healthy(&self) -> bool {
        true
    }
}
