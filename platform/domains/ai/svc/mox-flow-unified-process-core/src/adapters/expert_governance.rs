// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! expert-svc 治理流水线适配层
//!
//! expert-svc 有两套流水线：
//! 1. pipeline.rs - 全维治理流水线（mox_optimize 函数）
//! 2. alliance/gate.rs - 联盟管线 + 质量门禁（run_full_pipeline 函数）
//!
//! 两者都是阶段式瀑布流程，可以重构为统一流程定义 + 治理节点处理器。

use crate::types::*;

// ============================================================================
// 全维治理流水线 → 统一流程映射
// ============================================================================

/*
治理流水线的 8 个阶段可以映射为 8 个流程节点：

| 阶段 | 节点类型 | 节点 ID | 说明 |
|-----|---------|--------|------|
| 归一化 | Task (tool=Compute) | normalize | 维度着色 |
| 专家并行 | ParallelFork | experts_fork | 并行派发 14 位专家 |
| 裁决 | Task (tool=Compute) | reconcile | 归一化裁决 |
| 优化求解 | SubFlow | optimize | 调用 flow-svc 优化 |
| 璇玑验证 | Guard | verify | 最高权限校验 |
| 治理闸门 | Guard | govern | 8 闸门全量门禁 |
| 审计 | Task (tool=Compute) | audit | 审计链记录 |
| 结束 | End | end | 输出报告 |

对应的流程图结构：

  Start → normalize → experts_fork → [14个专家并行] → experts_join
        → reconcile → optimize → verify → govern → audit → End

GovernanceReport 对应 UnifiedExecutionResult 的扩展：
- expert_scores → node_results 中专家节点的输出
- optimization → SubFlow 节点的输出（flow-svc 优化报告）
- algo → verify 节点的输出
- gate → govern 节点的输出
- audit → audit 节点的输出
*/

// ============================================================================
// 联盟管线 → 统一流程映射
// ============================================================================

/*
联盟管线的 7 个阶段：

| 阶段 | 节点类型 | 节点 ID | 说明 |
|-----|---------|--------|------|
| Intent | Task (tool=Llm) | intent | 意图分类 |
| Team | Task (tool=Compute) | team | 专家组队优化 |
| Debate | Task (tool=Llm) | debate | 多专家辩论 |
| Synthesize | Task (tool=Llm) | synthesize | 合成输出 |
| Gate | Guard | gate | 质量门禁 (HC-8) |
| Learn | Task (tool=Compute) | learn | 指标学习 |
| Done | End | done | 完成 |

对应的流程图结构：

  Start → intent → team → debate → synthesize → gate → learn → End

AllianceEvent 对应 UnifiedNodeResult：
- phase → node_kind 扩展字段
- payload → output
- latency_ms → duration_ms
- ts → 由执行器记录

AuditEvent 7 类可以通过 FlowHook 实现：
- 每个节点执行后 emit 一个审计事件
- 由 AuditHook 统一收集和格式化
*/

// ============================================================================
// 治理节点处理器
// ============================================================================

/*
expert-svc 需要注册的节点处理器：

1. ExpertEvaluateHandler
   - 类型: UnifiedNodeKind::Task (tool=Compute + tag="expert")
   - 职责: 调用单个专家插件进行评估
   - 输入: 流程图 + 专家 ID
   - 输出: ExpertOpinion (评分、风险、建议)

2. ReconcileHandler
   - 类型: UnifiedNodeKind::Task (tool=Compute + tag="reconcile")
   - 职责: 归一化裁决，解决专家间冲突
   - 输入: 多个 ExpertOpinion
   - 输出: ReconciledPlan

3. VerifyHandler (璇玑验证)
   - 类型: UnifiedNodeKind::Guard + tag="xuanji_verify"
   - 职责: 最高权限算法验证
   - 输入: 优化报告
   - 输出: AlgoVerification (含 vetoed 标记)
   - 失败则阻断（GuardBlocked）

4. GovernGateHandler
   - 类型: UnifiedNodeKind::Guard + tag="govern_gate"
   - 职责: 治理闸门（8 闸门）
   - 输入: 优化报告 + 租户策略
   - 输出: GateResult (approved/rejected)

5. AuditHandler
   - 类型: 通过 FlowHook 实现（after_node 钩子）
   - 职责: 审计链记录
   - 每个节点执行后追加审计条目
*/

// ============================================================================
// Harness 插件体系与扩展注册表
// ============================================================================

/*
expert-svc 的 HarnessCtx（插件化运行时）可以很好地映射到扩展注册表：

- HarnessCtx → ExtensionRegistry
- ExpertPlugin → NodeHandler (或独立 trait)
- WaterfallEvent → FlowHook
- run_waterfall → 执行器的钩子机制

映射方式：
1. 所有专家插件注册为 NodeHandler（kind=Task, 通过 tag 区分）
2. 瀑布扩展点（PreGate/PostGate 等）注册为 FlowHook
3. HarnessProfile 的配置通过 ExtensionRegistry 传递

这样 expert-svc 既能复用统一执行引擎，
又保留了插件化的灵活性。
*/

// ============================================================================
// 迁移建议
// ============================================================================

/*
expert-svc 是改动最大的，因为它的"流水线"不是 DAG 节点执行模型，
而是瀑布式函数调用。建议最后迁移。

阶段一：类型桥接（无破坏性）
1. GovernanceReport 增加 From<UnifiedExecutionResult> 转换
2. 共享错误类型（UnifiedFlowError 扩展治理相关错误）

阶段二：节点化重构
1. 将每个治理阶段封装为 NodeHandler
2. 构建对应的 UnifiedFlowGraph 定义
3. 用 DagFlowExecutor 替换 mox_optimize 的硬编码顺序

阶段三：联盟管线迁移
1. 6 阶段封装为 NodeHandler
2. SSE 事件流通过 FlowHook 实现
3. 审计事件通过 FlowHook 统一收集

阶段四：Harness 整合
1. ExpertPlugin 适配为 NodeHandler
2. WaterfallEvent 适配为 FlowHook
3. 验证插件化运行时的所有能力都能正常工作
*/
