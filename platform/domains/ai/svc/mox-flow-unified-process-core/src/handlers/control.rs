// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 控制节点处理器
//!
//! Start / End / Decision / Parallel 等控制节点的内置实现。

use async_trait::async_trait;
use std::time::Instant;

use crate::error::FlowResult;
use crate::executor::{ExecutionContext, NodeHandler};
use crate::types::*;
use crate::utils::condition::evaluate_condition;

// ============================================================================
// Start 节点
// ============================================================================

pub struct StartHandler;

#[async_trait]
impl NodeHandler for StartHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::Start
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();
        let output = serde_json::json!({
            "started": true,
            "flow_id": context.flow_id,
            "variables_count": context.variables.len(),
        });
        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_start"
    }
}

// ============================================================================
// End 节点
// ============================================================================

pub struct EndHandler;

#[async_trait]
impl NodeHandler for EndHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::End
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();
        let last_output = context.last_output().cloned().unwrap_or(
            serde_json::json!({"status": "completed"}),
        );
        Ok(UnifiedNodeResult::success(
            node,
            last_output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_end"
    }
}

// ============================================================================
// Decision 节点
// ============================================================================

pub struct DecisionHandler;

#[async_trait]
impl NodeHandler for DecisionHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::Decision
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        let expression = match &node.config {
            UnifiedNodeConfig::Decision { expression } => expression.clone(),
            _ => {
                return Ok(UnifiedNodeResult::failed(
                    node,
                    "Decision 节点配置类型不匹配".into(),
                    start.elapsed().as_millis() as u64,
                ))
            }
        };

        let result = evaluate_condition(&expression, context.variables)
            .unwrap_or(false); // 表达式错误时默认 false（fail-closed）

        let output = serde_json::json!({
            "condition": expression,
            "result": result,
            "branch": if result { "true" } else { "false" },
        });

        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_decision"
    }
}
