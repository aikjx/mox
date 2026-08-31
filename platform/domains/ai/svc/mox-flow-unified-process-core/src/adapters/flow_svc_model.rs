// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! flow-svc model 适配层
//!
//! `mox-ai-flow-svc` 的模型是最接近统一核心的，因为统一核心
//! 的二级分类设计（NodeKind + ToolKind）就源自 flow-svc。
//! 因此转换几乎是 1:1 的，主要是命名调整。
//!
//! 映射关系：
//! - NodeKind → UnifiedNodeKind (几乎完全对应)
//! - ToolKind → UnifiedToolKind (几乎完全对应)
//! - FlowGraph → UnifiedFlowGraph (补充 variables/timestamps)
//! - FlowEdge (from/to) → UnifiedFlowEdge (source/target)

use crate::types::*;

// ============================================================================
// NodeKind ↔ UnifiedNodeKind 映射（1:1）
// ============================================================================

/*
| flow_svc NodeKind    | UnifiedNodeKind     | 说明 |
|---------------------|--------------------|------|
| Start               | Start              | 完全一致 |
| End                 | End                | 完全一致 |
| Task                | Task               | 完全一致 |
| Decision            | Decision           | 完全一致 |
| ParallelFork        | ParallelFork       | 完全一致 |
| ParallelJoin        | ParallelJoin       | 完全一致 |
| LoopStart           | LoopStart          | 完全一致 |
| LoopEnd             | LoopEnd            | 完全一致 |
| Guard               | Guard              | 完全一致 |
| SubFlow             | SubFlow            | 完全一致 |
*/

// flow_svc 中没有的节点类型（另外两套引擎有）：
// - Script → 新增
// - DataInput → 新增
// - DataOutput → 新增
// - Transform → 新增
// - UserTask → 新增（可用 Task + ToolKind::Human 近似）
// - Delay → 新增
// - Event → 新增

// ============================================================================
// ToolKind ↔ UnifiedToolKind 映射（1:1）
// ============================================================================

/*
| flow_svc ToolKind   | UnifiedToolKind    | 说明 |
|--------------------|-------------------|------|
| Compute            | Compute           | 完全一致 |
| Llm                | Llm               | 完全一致 |
| File               | File              | 完全一致 |
| Browser            | Browser           | 完全一致 |
| Database           | Database          | 完全一致 |
| Http               | Http              | 完全一致 |
| Shell              | Shell             | 完全一致 |
| Human              | Human             | 完全一致 |
| (无)               | Operator          | 新增（OUS 算子） |
| (无)               | Plugin            | 新增（插件调用） |
*/

// ============================================================================
// FlowGraph ↔ UnifiedFlowGraph 差异
// ============================================================================

/*
| 字段                 | FlowGraph | UnifiedFlowGraph | 处理方式 |
|---------------------|-----------|-----------------|---------|
| id                  | ✓         | ✓               | 直接映射 |
| name                | ✓         | ✓               | 直接映射 |
| description         | ✗         | ✓               | 默认空字符串 |
| nodes               | ✓         | ✓               | 类型转换 |
| edges               | ✓         | ✓               | 字段重命名: from→source, to→target |
| variables           | ✗         | ✓               | 默认空 HashMap |
| pools               | ✓         | ✓               | 直接映射 |
| rules               | ✓         | ✓               | 直接映射 |
| created_at          | ✗         | ✓               | 默认当前时间 |
| updated_at          | ✗         | ✓               | 默认当前时间 |
*/

// ============================================================================
// 优化流水线适配
// ============================================================================

/*
flow-svc 的优化 pipeline 是一个独立的阶段式处理流程，
不是传统意义上的"节点执行"。可以有两种适配方式：

方案 A：保持独立，仅共享 FlowGraph 类型
  - 优化算法（dataflow/conflict/schedule/codegen）继续独立存在
  - 只把 FlowGraph 替换为 UnifiedFlowGraph
  - 优点：改动最小，性能无损
  - 缺点：两套执行体系并存

方案 B：重构为节点处理器 + 流程定义
  - 每个优化阶段变成一个节点（DataflowAnalyze, ConflictDetect, ...）
  - 优化流程变成一个 UnifiedFlowGraph（6 个节点顺序执行）
  - 优点：统一执行模型，可灵活编排优化步骤
  - 缺点：改动较大，可能有性能开销

建议：先采用方案 A，验证类型统一的可行性，
      再评估是否需要方案 B 的深度整合。
*/

// ============================================================================
// 迁移建议（优先级最高，因为最接近）
// ============================================================================

/*
阶段一：类型统一（低风险）
1. FlowGraph 实现 Into<UnifiedFlowGraph> 和 From<UnifiedFlowGraph>
2. NodeKind ↔ UnifiedNodeKind 双向转换
3. ToolKind ↔ UnifiedToolKind 双向转换
4. 所有优化算法签名改为接受 UnifiedFlowGraph
   （内部先转回 FlowGraph，逐步替换）

阶段二：工具函数复用
1. 用核心库的 dag::topo_sort 替换内部拓扑排序
2. 用核心库的 dag::detect_cycle 替换循环检测
3. 验证输出一致性

阶段三：深度整合
1. 评估是否将优化流水线重构为节点处理器模式
2. 考虑将 conflict/schedule/codegen 等注册为扩展
*/
