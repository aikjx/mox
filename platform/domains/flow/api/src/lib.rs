// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! MOX Flow Domain API — trait contracts for workflow, operators, execution engine.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum FlowApiError {
    #[error("workflow not found: {0}")]
    NotFound(String),
    #[error("workflow validation failed: {0}")]
    Validation(String),
    #[error("execution error: {0}")]
    Execution(String),
    #[error("operator error: {0}")]
    Operator(String),
    #[error("internal: {0}")]
    Internal(String),
}

pub type FlowApiResult<T> = Result<T, FlowApiError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowStatus { Draft, Published, Running, Completed, Failed, Paused, Cancelled }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus { Pending, Running, Completed, Failed, Skipped, Retrying }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
    pub status: FlowStatus,
    pub created_at: String,
    pub updated_at: String,
}

impl FlowDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            version: 1,
            nodes: vec![],
            edges: vec![],
            status: FlowStatus::Draft,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub config: serde_json::Value,
    pub position: (f64, f64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowExecution {
    pub id: String,
    pub flow_id: String,
    pub flow_version: u32,
    pub status: FlowStatus,
    pub node_statuses: HashMap<String, NodeStatus>,
    pub results: HashMap<String, serde_json::Value>,
    pub errors: HashMap<String, String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub triggered_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorContext {
    pub flow_id: String,
    pub execution_id: String,
    pub node_id: String,
    pub input: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub tenant_id: String,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorResult {
    pub output: serde_json::Value,
    pub variables: HashMap<String, serde_json::Value>,
    pub next_nodes: Vec<String>,
}

#[async_trait]
pub trait FlowOperator: Send + Sync {
    fn operator_type(&self) -> &str;
    async fn execute(&self, ctx: OperatorContext) -> FlowApiResult<OperatorResult>;
    fn validate_config(&self, config: &serde_json::Value) -> FlowApiResult<()>;
}

#[async_trait]
pub trait FlowEngine: Send + Sync {
    async fn create_flow(&self, flow: FlowDefinition) -> FlowApiResult<FlowDefinition>;
    async fn get_flow(&self, id: &str) -> FlowApiResult<Option<FlowDefinition>>;
    async fn update_flow(&self, flow: FlowDefinition) -> FlowApiResult<FlowDefinition>;
    async fn delete_flow(&self, id: &str) -> FlowApiResult<bool>;
    async fn list_flows(&self, tenant_id: &str) -> FlowApiResult<Vec<FlowDefinition>>;
    async fn trigger(&self, flow_id: &str, input: serde_json::Value, triggered_by: &str) -> FlowApiResult<String>;
    async fn get_execution(&self, execution_id: &str) -> FlowApiResult<Option<FlowExecution>>;
    async fn cancel_execution(&self, execution_id: &str) -> FlowApiResult<bool>;
    async fn register_operator(&self, operator: Box<dyn FlowOperator>) -> FlowApiResult<()>;
}
