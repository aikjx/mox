// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 执行状态持久化端口（ExecutionStateSink）
//!
//! # 依赖倒置（端口在消费侧定义）
//!
//! DAG 执行引擎的内存态（任务/节点/输出）此前进程重启即丢失——存储层虽有
//! `SqliteTaskRepository`（含恢复标记与读穿缓存），但引擎侧没有任何回写出口，
//! 「持久化状态如何恢复到执行引擎」断链。本 trait 是引擎侧的持久化出口端口：
//! - **端口（本 trait）**定义在消费侧 executor-core，core 不引入任何 IO 依赖；
//! - **适配器**由 svc 层提供（包装 `SqliteTaskRepository` 等实现），
//!   装配时经 `DagEngineImpl::spawn_with_state_sink` 注入；
//! - 未注入（`None`）时行为与改动前完全一致（纯内存执行）。
//!
//! # 失败语义
//!
//! 持久化失败**不阻断执行**（引擎照常推进内存态），仅由调用方记录告警——
//! 可用性优先于持久化完整性；对账由存储层恢复标记（running→interrupted）兜底。

use mox_alliance_common_proto::{AllianceResult, CollaborationPlan, Node, NodeStatus, Task};
use uuid::Uuid;

/// 恢复期可重建的任务（由适配器从存储层读出）
pub struct RestorableTask {
    /// 任务行（状态为存储层恢复解析后的值：running→interrupted→pending）
    pub task: Task,
    /// 协作计划（DAG），恢复执行的依据
    pub plan: CollaborationPlan,
    /// 已完成节点（node_id, result），恢复后直接置 Completed，不再重复执行
    pub completed_nodes: Vec<(String, Option<serde_json::Value>)>,
}

/// 执行状态持久化端口
pub trait ExecutionStateSink: Send + Sync {
    /// 持久化任务整行（状态机推进 / 进度更新时调用）
    fn persist_task(&self, task: &Task) -> AllianceResult<()>;

    /// 持久化协作计划（DAG）——恢复期重建执行状态的必要输入
    fn persist_plan(&self, task_id: Uuid, plan: &CollaborationPlan) -> AllianceResult<()>;

    /// 启动时恢复扫描：读取「有计划且任务未达终态」的任务及其已完成节点。
    ///
    /// 语义约定（场景②/③）：
    /// - 仅返回**有持久化计划**的任务（无计划无法重建 DAG，交由人工/对账处理）；
    /// - `completed_nodes` 用于跳过已成功节点，避免重复执行与重复副作用；
    /// - 文件底座无 plan / 节点表 → 返回空列表（升级 sqlite 即得完整恢复能力）。
    fn restore_pending(&self) -> AllianceResult<Vec<RestorableTask>>;

    /// 引擎内存 miss 时的降级回读（**多实例水平扩展前提**）。
    ///
    /// 任务可能正由另一实例执行或已在本实例重启前完成——状态查询不能绑定单进程内存。
    /// 返回 `None` = 存储层无此任务，或底座不具备回读能力（file，默认实现）。
    fn read_back(&self, task_id: Uuid) -> AllianceResult<Option<ExecutionView>> {
        let _ = task_id;
        Ok(None)
    }

    /// 持久化单个 DAG 节点行（增量 upsert：status / result）
    ///
    /// `status` 使用与 `TaskStatus` serde 一致的 snake_case 小写
    /// （running / completed / failed / skipped / cancelled / pending），
    /// 与存储层恢复解析（interrupted → pending）对齐。
    fn persist_node(
        &self,
        task_id: Uuid,
        node_id: &str,
        status: &str,
        result: Option<&serde_json::Value>,
    ) -> AllianceResult<()>;

    /// 持久化融合输出（可选能力：sqlite 底座经保留节点行落盘；默认不持久化）
    fn persist_fusion_output(
        &self,
        _task_id: Uuid,
        _output: &serde_json::Value,
    ) -> AllianceResult<()> {
        Ok(())
    }

    /// 回读融合输出（与 [`Self::persist_fusion_output`] 配对；默认无能力）
    fn read_fusion_output(&self, _task_id: Uuid) -> AllianceResult<Option<serde_json::Value>> {
        Ok(None)
    }
}

/// 融合输出的保留节点行 id：sqlite 底座借节点表存储融合 JSON。
///
/// 非真实 DAG 节点——`read_back` / `restore_pending` 必须过滤本 id，
/// 避免污染节点计数与恢复重建。
pub const FUSION_OUTPUT_NODE_ID: &str = "__fusion_output__";

/// 引擎内存 miss 时的降级回读视图
pub struct ExecutionView {
    /// 任务行（存储层权威状态）
    pub task: Task,
    /// 协作计划（重建节点集合与依赖；可能缺失——无计划则无法还原节点明细）
    pub plan: Option<CollaborationPlan>,
    /// 节点行（node_id, status, result）
    pub nodes: Vec<(String, String, Option<serde_json::Value>)>,
}

/// 存储层状态字符串 → NodeStatus（与引擎落盘口径一致；interrupted 视作可重认领的 Pending）
pub fn node_status_from_str(s: &str) -> NodeStatus {
    match s {
        "running" => NodeStatus::Running,
        "completed" => NodeStatus::Completed,
        "failed" => NodeStatus::Failed,
        "skipped" => NodeStatus::Skipped,
        "cancelled" => NodeStatus::Cancelled,
        _ => NodeStatus::Pending,
    }
}

/// 由持久化计划 + 节点行合成节点集合（回读视图 → 引擎可计算的状态）
///
/// 节点骨架来自计划（DAG 权威），状态以节点行为准；无行的节点视为 Pending。
pub fn synthesize_nodes(
    plan: &CollaborationPlan,
    rows: &[(String, String, Option<serde_json::Value>)],
) -> Vec<Node> {
    plan.nodes
        .iter()
        .map(|n| {
            let mut node = n.clone();
            if let Some((_, status, _)) = rows.iter().find(|(id, _, _)| id == &node.node_id) {
                node.status = node_status_from_str(status);
            }
            node
        })
        .collect()
}
