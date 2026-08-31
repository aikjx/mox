// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! agent-svc flow_engine 适配层
//!
//! 将 `flow_engine::FlowDefinition` 等类型转换为统一核心类型。
//!
//! 注意：这是参考实现，实际代码应放在 mox-ai-agent-svc crate 中，
//! 通过 `impl From<flow_engine::FlowDefinition> for UnifiedFlowGraph` 实现。
//!
//! 映射关系：
//! - NodeType → UnifiedNodeKind (+ UnifiedToolKind)
//! - FlowDefinition → UnifiedFlowGraph
//! - FlowError → UnifiedFlowError

use crate::error::UnifiedFlowError;
use crate::types::*;

// ============================================================================
// NodeType → UnifiedNodeKind 映射
// ============================================================================

/// flow_engine::NodeType → UnifiedNodeKind 转换
///
/// | flow_engine NodeType | 统一类型 | 说明 |
/// |---------------------|---------|------|
/// | Start               | Start   | 一致 |
/// | End                 | End     | 一致 |
/// | Task                | Task    | 需绑定 tool=None |
/// | Guard               | Guard   | 一致 |
/// | Decision            | Decision| 一致 |
/// | Event               | Event   | 一致 |
/// | LLM                 | Task    | tool=Llm |
/// | Browser             | Task    | tool=Browser |
/// | HttpRequest         | Task    | tool=Http |
/// | Operator            | Task    | tool=Operator |
/// | Condition           | Decision| 语义相同，命名不同 |
/// | Transform           | Transform | 一致 |
/// | Script              | Script  | 一致 |
/// | DataInput           | DataInput | 一致 |
/// | DataOutput          | DataOutput | 一致 |
/// | Parallel            | ParallelFork | 需要拆分为 fork+join |
pub fn map_node_type_to_kind(node_type: &str) -> (UnifiedNodeKind, Option<UnifiedToolKind>) {
    match node_type {
        "Start" => (UnifiedNodeKind::Start, None),
        "End" => (UnifiedNodeKind::End, None),
        "Task" => (UnifiedNodeKind::Task, None),
        "Guard" => (UnifiedNodeKind::Guard, None),
        "Decision" => (UnifiedNodeKind::Decision, None),
        "Event" => (UnifiedNodeKind::Event, None),
        "LLM" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Llm)),
        "Browser" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Browser)),
        "HttpRequest" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Http)),
        "Operator" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Operator)),
        "Condition" => (UnifiedNodeKind::Decision, None),
        "Transform" => (UnifiedNodeKind::Transform, None),
        "Script" => (UnifiedNodeKind::Script, None),
        "DataInput" => (UnifiedNodeKind::DataInput, None),
        "DataOutput" => (UnifiedNodeKind::DataOutput, None),
        "Parallel" => (UnifiedNodeKind::ParallelFork, None),
        _ => (UnifiedNodeKind::Task, None), // 兜底
    }
}

// ============================================================================
// FlowError → UnifiedFlowError 映射
// ============================================================================

/// flow_engine::FlowError → UnifiedFlowError 转换
pub fn map_flow_error(error_msg: &str, error_type: &str) -> UnifiedFlowError {
    match error_type {
        "NodeNotFound" => UnifiedFlowError::NodeNotFound(error_msg.to_string()),
        "CycleDetected" => UnifiedFlowError::CycleDetected(error_msg.to_string()),
        "ExecutionFailed" => UnifiedFlowError::NodeExecutionFailed {
            node_id: "unknown".into(),
            reason: error_msg.to_string(),
        },
        "ConditionError" => UnifiedFlowError::ConditionError(error_msg.to_string()),
        "InvalidConfig" => UnifiedFlowError::InvalidConfig(error_msg.to_string()),
        _ => UnifiedFlowError::Internal(error_msg.to_string()),
    }
}

// ============================================================================
// AI 能力节点处理器示例
// ============================================================================

/// LLM 节点处理器（agent-svc 专属）
///
/// 注入 LLMClient 扩展，执行 AI 任务。
///
/// ```ignore
/// pub struct LlmNodeHandler;
///
/// #[async_trait]
/// impl NodeHandler for LlmNodeHandler {
///     fn kind(&self) -> UnifiedNodeKind { UnifiedNodeKind::Task }
///
///     async fn execute(&self, node: &UnifiedFlowNode, ctx: &ExecutionContext<'_>)
///         -> FlowResult<UnifiedNodeResult>
///     {
///         // 1. 检查是否为 LLM 工具类型
///         if node.tool != Some(UnifiedToolKind::Llm) {
///             // 委托给下一个处理器
///         }
///
///         // 2. 从扩展中获取 LLM 客户端
///         let llm = ctx.extensions.get::<Arc<RwLock<LLMClient>>>()
///             .ok_or_else(|| UnifiedFlowError::ExtensionError { ... })?;
///
///         // 3. 解析 prompt 配置
///         let prompt = match &node.config {
///             UnifiedNodeConfig::Task { tool_config } => {
///                 tool_config.get("prompt").and_then(|p| p.as_str()).unwrap_or("")
///             }
///             _ => "",
///         };
///
///         // 4. 模板替换
///         let rendered = apply_template(prompt, ctx.variables);
///
///         // 5. 调用 LLM
///         let response = llm.read().await.chat(...).await?;
///
///         Ok(UnifiedNodeResult::success(node, response, duration))
///     }
/// }
/// ```

// ============================================================================
// 迁移建议
// ============================================================================

/*
1. 在 agent-svc 的 Cargo.toml 中添加依赖：
   mox-flow-unified-process-core = { path = "../mox-flow-unified-process-core" }

2. 实现 From/Into 转换：
   impl From<FlowDefinition> for UnifiedFlowGraph { ... }
   impl From<NodeType> for (UnifiedNodeKind, Option<UnifiedToolKind>) { ... }

3. 注册 AI 节点处理器：
   let mut executor = DagFlowExecutor::default();
   executor.register_handler(UnifiedNodeKind::Task, Box::new(LlmNodeHandler));
   executor.register_handler(UnifiedNodeKind::Task, Box::new(OperatorNodeHandler));
   // ...

4. 替换 FlowEngine::execute_flow 为 executor.execute()

5. 验证所有内置模板通过测试
*/
