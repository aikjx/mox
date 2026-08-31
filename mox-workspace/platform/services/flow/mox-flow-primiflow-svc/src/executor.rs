//! 节点执行器
//!
//! 负责执行 DAG 中的单个节点，调用对应的算子

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::error::FlowResult;

/// 节点执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 已成功
    Succeeded,
    /// 已失败
    Failed,
    /// 已跳过
    Skipped,
    /// 已取消
    Cancelled,
}

/// 节点执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionResult {
    /// 节点 ID
    pub node_id: String,
    /// 执行状态
    pub status: NodeStatus,
    /// 输出数据
    pub output: Option<serde_json::Value>,
    /// 错误信息
    pub error: Option<String>,
    /// 开始时间（毫秒）
    pub start_time: Option<i64>,
    /// 结束时间（毫秒）
    pub end_time: Option<i64>,
    /// 重试次数
    pub retry_count: u32,
}

/// 算子执行器接口
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// 执行器名称
    fn name(&self) -> &str;

    /// 执行节点
    async fn execute(
        &self,
        node_id: &str,
        operator_type: &str,
        config: Option<&serde_json::Value>,
        inputs: &[serde_json::Value],
    ) -> FlowResult<NodeExecutionResult>;
}

/// 默认执行器（占位实现）
#[derive(Debug, Default)]
pub struct DefaultNodeExecutor;

impl DefaultNodeExecutor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeExecutor for DefaultNodeExecutor {
    fn name(&self) -> &str {
        "default-executor"
    }

    async fn execute(
        &self,
        node_id: &str,
        _operator_type: &str,
        _config: Option<&serde_json::Value>,
        _inputs: &[serde_json::Value],
    ) -> FlowResult<NodeExecutionResult> {
        // TODO: 实现完整的节点执行逻辑
        Ok(NodeExecutionResult {
            node_id: node_id.to_string(),
            status: NodeStatus::Succeeded,
            output: Some(serde_json::Value::Null),
            error: None,
            start_time: None,
            end_time: None,
            retry_count: 0,
        })
    }
}
