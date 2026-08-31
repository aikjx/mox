// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行器 trait 定义
//!
//! `FlowExecutor`：流程执行器标准接口
//! `NodeHandler`：节点处理器扩展点（各服务注入差异化能力）

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::{FlowResult, UnifiedFlowError};
use crate::types::*;
use super::context::ExecutionContext;

/// 流程执行器 —— 统一所有流程引擎的执行入口
///
/// 各服务可实现此 trait 自定义执行逻辑，
/// 或直接使用内置的 `DagFlowExecutor`。
#[async_trait]
pub trait FlowExecutor: Send + Sync {
    /// 执行流程图
    async fn execute(
        &self,
        graph: &UnifiedFlowGraph,
        input: HashMap<String, serde_json::Value>,
    ) -> FlowResult<UnifiedExecutionResult>;

    /// 校验流程图结构（DAG、节点引用、循环检测等）
    fn validate(&self, graph: &UnifiedFlowGraph) -> FlowResult<()>;

    /// 注册节点处理器
    fn register_handler(&mut self, kind: UnifiedNodeKind, handler: Box<dyn NodeHandler>);

    /// 获取节点处理器
    fn get_handler(&self, kind: &UnifiedNodeKind) -> Option<&dyn NodeHandler>;
}

/// 节点处理器 —— 每个具体节点类型的执行逻辑
///
/// 这是统一核心库的主要扩展点。各服务通过实现此 trait
/// 注入自己的差异化能力：
///
/// - agent-svc: LLM 调用、算子执行、插件调用、浏览器自动化
/// - expert-svc: 专家评估、裁决、治理闸门、审计
/// - flow-svc: 数据流分析、冲突检测、调度、代码生成
///
/// # 示例
/// ```ignore
/// struct LlmNodeHandler { llm_client: Arc<LLMClient> }
///
/// #[async_trait]
/// impl NodeHandler for LlmNodeHandler {
///     fn kind(&self) -> UnifiedNodeKind { UnifiedNodeKind::Task }
///
///     async fn execute(&self, node: &UnifiedFlowNode, ctx: &ExecutionContext<'_>)
///         -> FlowResult<UnifiedNodeResult>
///     {
///         // 调用 LLM ...
///     }
/// }
/// ```
#[async_trait]
pub trait NodeHandler: Send + Sync {
    /// 处理的节点类型
    fn kind(&self) -> UnifiedNodeKind;

    /// 执行节点
    ///
    /// # 参数
    /// - `node`: 待执行的节点
    /// - `context`: 执行上下文（变量、前序输出、扩展注册表等）
    ///
    /// # 返回
    /// 节点执行结果（成功或失败）
    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult>;

    /// 是否可以并行执行（默认 true）
    ///
    /// 返回 false 的节点将被串行执行（即使在并行层中）
    fn is_parallelizable(&self) -> bool {
        true
    }

    /// 预估执行耗时（用于调度优化，默认 0 表示未知）
    fn estimate_duration_ms(&self, _node: &UnifiedFlowNode) -> u64 {
        0
    }

    /// 处理器名称（用于日志和错误信息）
    fn name(&self) -> &'static str {
        "unnamed_handler"
    }
}

/// 节点处理注册表
pub type HandlerRegistry = HashMap<UnifiedNodeKind, Box<dyn NodeHandler>>;

// 为 Box<dyn NodeHandler> 实现 NodeHandler，便于装箱
#[async_trait]
impl<T: NodeHandler + ?Sized> NodeHandler for Box<T> {
    fn kind(&self) -> UnifiedNodeKind {
        (**self).kind()
    }

    async fn execute(
        &self,
        node: &UnifiedFlowNode,
        context: &ExecutionContext<'_>,
    ) -> FlowResult<UnifiedNodeResult> {
        (**self).execute(node, context).await
    }

    fn is_parallelizable(&self) -> bool {
        (**self).is_parallelizable()
    }

    fn estimate_duration_ms(&self, node: &UnifiedFlowNode) -> u64 {
        (**self).estimate_duration_ms(node)
    }

    fn name(&self) -> &'static str {
        (**self).name()
    }
}

/// 扩展点：阶段钩子
///
/// 允许在流程执行的不同阶段注入自定义逻辑。
#[async_trait]
pub trait FlowHook: Send + Sync {
    /// 钩子名称
    fn name(&self) -> &'static str;

    /// 执行前钩子（在整个流程开始前调用）
    async fn before_execute(&self, _graph: &UnifiedFlowGraph) -> FlowResult<()> {
        Ok(())
    }

    /// 节点执行前钩子
    async fn before_node(
        &self,
        _node: &UnifiedFlowNode,
        _context: &ExecutionContext<'_>,
    ) -> FlowResult<()> {
        Ok(())
    }

    /// 节点执行后钩子
    async fn after_node(
        &self,
        _node: &UnifiedFlowNode,
        _result: &mut UnifiedNodeResult,
    ) -> FlowResult<()> {
        Ok(())
    }

    /// 执行后钩子（在整个流程结束后调用）
    async fn after_execute(
        &self,
        _result: &mut UnifiedExecutionResult,
    ) -> FlowResult<()> {
        Ok(())
    }
}
