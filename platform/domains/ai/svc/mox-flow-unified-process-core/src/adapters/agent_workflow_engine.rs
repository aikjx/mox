// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! agent-svc workflow_engine 适配层
//!
//! 将 BPMN 风格的 `BusinessWorkflow` 转换为统一核心类型。
//!
//! 映射关系：
//! - WorkflowNodeType → UnifiedNodeKind
//! - WorkflowNodeConfig → UnifiedNodeConfig
//! - BusinessWorkflow → UnifiedFlowGraph

use crate::types::*;

// ============================================================================
// WorkflowNodeType → UnifiedNodeKind 映射
// ============================================================================

/// WorkflowNodeType → UnifiedNodeKind 转换
///
/// | WorkflowNodeType | UnifiedNodeKind | 说明 |
/// |-----------------|----------------|------|
/// | Start           | Start          | 一致 |
/// | End             | End            | 一致 |
/// | Operator        | Task           | tool=Operator |
/// | Condition       | Decision       | 语义相同 |
/// | Parallel        | ParallelFork   | 需要 fork+join 一对 |
/// | SubWorkflow     | SubFlow        | 语义相同 |
/// | UserTask        | UserTask       | 一致 |
/// | Script          | Script         | 一致 |
/// | AiTask          | Task           | tool=Llm |
/// | PluginCall      | Task           | tool=Plugin |
/// | Delay           | Delay          | 一致 |
pub fn map_workflow_type_to_kind(
    node_type: &str,
) -> (UnifiedNodeKind, Option<UnifiedToolKind>) {
    match node_type {
        "Start" => (UnifiedNodeKind::Start, None),
        "End" => (UnifiedNodeKind::End, None),
        "Operator" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Operator)),
        "Condition" => (UnifiedNodeKind::Decision, None),
        "Parallel" => (UnifiedNodeKind::ParallelFork, None),
        "SubWorkflow" => (UnifiedNodeKind::SubFlow, None),
        "UserTask" => (UnifiedNodeKind::UserTask, None),
        "Script" => (UnifiedNodeKind::Script, None),
        "AiTask" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Llm)),
        "PluginCall" => (UnifiedNodeKind::Task, Some(UnifiedToolKind::Plugin)),
        "Delay" => (UnifiedNodeKind::Delay, None),
        _ => (UnifiedNodeKind::Task, None),
    }
}

// ============================================================================
// WorkflowNodeConfig → UnifiedNodeConfig 映射
// ============================================================================

/// WorkflowNodeConfig 字段映射到 UnifiedNodeConfig
///
/// Start → Start
/// End → End
/// Operator { operator_id, parameters } → Task { tool_config: {operator_id, parameters} }
/// Condition { expression, true_path, false_path } → Decision { expression }
///   注意：true_path/false_path 在 workflow_engine 中是节点 ID，
///   统一核心通过 Conditional 边的 condition 字段表达分支选择
/// Parallel { branches, merge_strategy } → Parallel { merge_strategy }
/// SubWorkflow { workflow_id } → SubFlow { flow_id, ... }
/// UserTask { assignee, form } → UserTask { assignee, form }
/// Script { language, code } → Script { language, code }
/// AiTask { task_type, prompt } → Task { tool_config: {task_type, prompt} }
/// PluginCall { plugin_id, method, parameters } → Task { tool_config: {plugin_id, method, parameters} }
/// Delay { duration_ms } → Delay { duration_ms }

// ============================================================================
// BPMN 特性说明
// ============================================================================

/*
workflow_engine 中的 BPMN 特性在统一核心中的对应实现：

1. 并行分支 (Parallel)
   - workflow_engine: 单个 Parallel 节点 + branches 列表
   - 统一核心: ParallelFork + ParallelJoin 两个节点 + 多条边
   - 转换时需自动插入 Join 节点

2. 子流程 (SubWorkflow)
   - workflow_engine: SubWorkflow 节点 + workflow_id
   - 统一核心: SubFlow 节点 + flow_id + input/output mapping
   - 执行时通过子执行器递归调用

3. 用户任务 (UserTask)
   - workflow_engine: UserTask 节点，标记为 pending
   - 统一核心: UserTask 节点 + status=Waiting
   - 需要外部系统（人工审批）回调后继续执行

4. 执行模式
   - workflow_engine: BFS 队列
   - 统一核心: 拓扑排序分层执行
   - 两者语义等价，但分层执行更便于并行优化
*/

// ============================================================================
// 迁移建议
// ============================================================================

/*
1. 实现 BusinessWorkflow → UnifiedFlowGraph 转换
   - 处理 Parallel 节点：拆分为 Fork + Join 对
   - 处理 Condition 节点：true/false_path 转为 Conditional 边

2. 注册 BPMN 特定处理器
   - SubFlowHandler: 子流程调用（递归执行器）
   - UserTaskHandler: 用户任务（等待外部回调）
   - OperatorHandler: 算子执行（HTTP 调用）
   - PluginCallHandler: 插件调用（HTTP 调用）

3. 替换 WorkflowEngine::execute 为统一执行器
   - 保留 WorkflowInstance 概念（可通过 FlowHook 实现）
   - 保留 execution_log（可通过 FlowHook 实现）

4. 逐步迁移内置模板
   - data-pipeline → 最易迁移
   - nn-training → 需处理循环（Condition + 回边 → LoopStart/LoopEnd）
   - algorithm-analysis → 中等复杂度
*/
