// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 数据节点处理器
//!
//! DataInput / DataOutput / Transform 等数据相关节点的内置实现。

use async_trait::async_trait;
use std::time::Instant;

use crate::error::FlowResult;
use crate::executor::{ExecutionContext, NodeHandler};
use crate::types::*;
use crate::utils::template::apply_template;

// ============================================================================
// DataInput 节点
// ============================================================================

pub struct DataInputHandler;

#[async_trait]
impl NodeHandler for DataInputHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::DataInput
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        let value = match &node.config {
            UnifiedNodeConfig::DataInput { value, source } => {
                // 优先使用静态值
                if let Some(v) = value {
                    Some(v.clone())
                } else if let Some(src) = source {
                    // 从变量中获取
                    context.get_var(src).cloned()
                } else {
                    // 从 last_output 获取
                    context.last_output().cloned()
                }
            }
            _ => None,
        };

        let output = value.unwrap_or(serde_json::Value::Null);

        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_data_input"
    }
}

// ============================================================================
// DataOutput 节点
// ============================================================================

pub struct DataOutputHandler;

#[async_trait]
impl NodeHandler for DataOutputHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::DataOutput
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        // 透传 last_output
        let output = context
            .last_output()
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_data_output"
    }
}

// ============================================================================
// Transform 节点
// ============================================================================

pub struct TransformHandler;

#[async_trait]
impl NodeHandler for TransformHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::Transform
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        let template = match &node.config {
            UnifiedNodeConfig::Transform { template } => template.clone(),
            _ => {
                return Ok(UnifiedNodeResult::failed(
                    node,
                    "Transform 节点配置类型不匹配".into(),
                    start.elapsed().as_millis() as u64,
                ))
            }
        };

        let result = apply_template(&template, context.variables);
        let output = serde_json::json!({ "transformed": result });

        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn name(&self) -> &'static str {
        "builtin_transform"
    }
}

// ============================================================================
// Delay 节点
// ============================================================================

pub struct DelayHandler;

#[async_trait]
impl NodeHandler for DelayHandler {
    fn kind(&self) -> UnifiedNodeKind {
        UnifiedNodeKind::Delay
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        _context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        let start = Instant::now();

        let duration_ms = match &node.config {
            UnifiedNodeConfig::Delay { duration_ms } => *duration_ms,
            _ => 0,
        };

        if duration_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(duration_ms)).await;
        }

        let output = serde_json::json!({
            "delay_completed": true,
            "duration_ms": duration_ms,
        });

        Ok(UnifiedNodeResult::success(
            node,
            output,
            start.elapsed().as_millis() as u64,
        ))
    }

    fn is_parallelizable(&self) -> bool {
        true
    }

    fn name(&self) -> &'static str {
        "builtin_delay"
    }
}
