//! DAG 执行引擎
//!
//! 整合调度器和执行器，驱动整个 DAG 流程的执行

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::dag::Dag;
use crate::error::FlowResult;
use crate::executor::NodeExecutionResult;
use crate::scheduler::{ScheduleConfig, ExecutionPlan};

/// 流程执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    /// 等待中
    Pending,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已成功完成
    Succeeded,
    /// 已失败
    Failed,
    /// 已取消
    Cancelled,
    /// 超时
    Timeout,
}

/// 流程执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecutionResult {
    /// 流程实例 ID
    pub instance_id: String,
    /// DAG ID
    pub dag_id: String,
    /// 执行状态
    pub status: FlowStatus,
    /// 节点执行结果
    pub node_results: std::collections::HashMap<String, NodeExecutionResult>,
    /// 开始时间（毫秒）
    pub start_time: Option<i64>,
    /// 结束时间（毫秒）
    pub end_time: Option<i64>,
    /// 总耗时（毫秒）
    pub duration_ms: Option<i64>,
    /// 错误信息
    pub error: Option<String>,
}

/// DAG 执行引擎接口
#[async_trait]
pub trait FlowEngine: Send + Sync {
    /// 引擎名称
    fn name(&self) -> &str;

    /// 提交 DAG 执行
    async fn submit(&self, dag: &Dag, config: &ScheduleConfig) -> FlowResult<String>;

    /// 获取执行状态
    async fn get_status(&self, instance_id: &str) -> FlowResult<FlowStatus>;

    /// 获取执行结果
    async fn get_result(&self, instance_id: &str) -> FlowResult<FlowExecutionResult>;

    /// 取消执行
    async fn cancel(&self, instance_id: &str) -> FlowResult<bool>;

    /// 暂停执行
    async fn pause(&self, instance_id: &str) -> FlowResult<bool>;

    /// 恢复执行
    async fn resume(&self, instance_id: &str) -> FlowResult<bool>;
}

/// 默认执行引擎（占位实现）
#[derive(Debug, Default)]
pub struct DefaultFlowEngine;

impl DefaultFlowEngine {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FlowEngine for DefaultFlowEngine {
    fn name(&self) -> &str {
        "default-flow-engine"
    }

    async fn submit(&self, dag: &Dag, _config: &ScheduleConfig) -> FlowResult<String> {
        // TODO: 实现完整的 DAG 执行逻辑
        Ok(format!("flow-{}", dag.id))
    }

    async fn get_status(&self, _instance_id: &str) -> FlowResult<FlowStatus> {
        Ok(FlowStatus::Succeeded)
    }

    async fn get_result(&self, instance_id: &str) -> FlowResult<FlowExecutionResult> {
        Ok(FlowExecutionResult {
            instance_id: instance_id.to_string(),
            dag_id: "".to_string(),
            status: FlowStatus::Succeeded,
            node_results: std::collections::HashMap::new(),
            start_time: None,
            end_time: None,
            duration_ms: None,
            error: None,
        })
    }

    async fn cancel(&self, _instance_id: &str) -> FlowResult<bool> {
        Ok(true)
    }

    async fn pause(&self, _instance_id: &str) -> FlowResult<bool> {
        Ok(true)
    }

    async fn resume(&self, _instance_id: &str) -> FlowResult<bool> {
        Ok(true)
    }
}

// 引入 ExecutionPlan 以避免未使用警告（实际在引擎中会使用）
#[allow(dead_code)]
fn _use_plan(_plan: &ExecutionPlan) {}
