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

use mox_alliance_common_proto::{AllianceResult, Task};
use uuid::Uuid;

/// 执行状态持久化端口
pub trait ExecutionStateSink: Send + Sync {
    /// 持久化任务整行（状态机推进 / 进度更新时调用）
    fn persist_task(&self, task: &Task) -> AllianceResult<()>;

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
}
