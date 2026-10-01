// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 人审 Approval Gate（一等公民的「节点挂起等审批再续跑」纯逻辑）。
//!
//! 背景：既有调度/执行链路只有 `skip_node`（人工跳过）与人工 `complete_task`（人工终态），
//! 缺一条「DAG 节点执行到一半挂起、等人审批、再放行/驳回」的 first-class 路径。本模块
//! 把这条路径的**纯状态机**抽出来：不接 svc 路由、不碰 proto、不修改既有调度函数签名，
//! 仅作为可被 scheduler 装配时复用的内存审批门。
//!
//! 设计约束（对齐本次「最小安全增量」铁律）：
//! - 只用 std + chrono + 本 crate 已依赖的 `mox-alliance-common-proto`；不新增依赖。
//! - 不平行重定义任务/节点状态机：复用 common-proto 的 [`NodeStatus`] 做终态映射；
//!   common-proto `NodeStatus` 当前没有 `Blocked` 变体，故 gate 侧用一个极小的
//!   [`GateNodeEffect`] 投影表达「挂起/放行/驳回」，由调度器在 Hold 期间不派发该节点，
//!   而不是反向去改权威枚举。
//! - 时钟不注入 trait：所有时间判定以调用方传入的 `now: DateTime<Utc>` 为准，
//!   保证单测确定性（对齐本 crate 其他纯逻辑模块的约定）。

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use mox_alliance_common_proto::types::NodeStatus;

/// 审批请求状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// 已提交、等待审批（尚未过期）。
    Pending,
    /// 审批通过，节点可继续。
    Approved,
    /// 审批拒绝，节点失败。
    Rejected,
    /// 超过 TTL 仍未决，自动挂起（需重新提交或人工续期）。
    Expired,
}

/// 一条人审请求。
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovalRequest {
    pub task_id: String,
    pub node_id: String,
    /// 挂起原因（为什么这个节点需要人审）。
    pub reason: String,
    pub requested_at: DateTime<Utc>,
    pub status: ApprovalStatus,
    /// 决策时刻；Pending/Expired 为 None。
    pub decided_at: Option<DateTime<Utc>>,
}

impl ApprovalRequest {
    /// 是否仍处于「等待人决策」的 Pending 态（未过期）。
    pub fn is_live_pending(&self, now: DateTime<Utc>, ttl: Duration) -> bool {
        self.status == ApprovalStatus::Pending && !self.is_expired(now, ttl)
    }

    /// 是否已超过 TTL（仅对 Pending 有意义）。
    pub fn is_expired(&self, now: DateTime<Utc>, ttl: Duration) -> bool {
        self.status == ApprovalStatus::Pending && now - self.requested_at > ttl
    }
}

/// 审批门对 DAG 节点的作用推导（gate 侧投影）。
///
/// 为什么不直接返回 `NodeStatus`：common-proto 的 `NodeStatus` 是节点在 DAG 引擎里的
/// 权威状态机（Pending/Ready/Running/Completed/Failed/Skipped/Cancelled），没有「Blocked」
/// 变体。挂起语义由调度器在 Hold 期间「不派发、不推进」来实现，而非占用一个节点状态位；
/// 只有拒绝是终态，可以无损映射到 [`NodeStatus::Failed`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateNodeEffect {
    /// 挂起：审批未决（含过期未续），调度器不应派发该节点。
    Hold,
    /// 放行：审批通过，节点可进入 Ready 并被派发。
    Release,
    /// 驳回：审批拒绝，节点应转入失败。
    Fail,
}

impl GateNodeEffect {
    /// 若本 effect 需要改写权威节点状态，给出目标 `NodeStatus`（Hold 不改写，返回 None）。
    pub fn to_node_status(self) -> Option<NodeStatus> {
        match self {
            GateNodeEffect::Hold => None,
            GateNodeEffect::Release => Some(NodeStatus::Ready),
            GateNodeEffect::Fail => Some(NodeStatus::Failed),
        }
    }
}

/// 根据一条审批请求推导当前对节点的作用。
///
/// 纯函数：只读取状态、不做副作用，因此**不在此把过期的 Pending 翻成 Expired**。
/// Pending（无论是否已超过 TTL）与 Expired 对当拍派发的作用相同——都是 Hold（不派发、
/// 不推进）；过期状态的惰性翻转由持有 `&mut self` 的 [`ApprovalGate::list_pending`]
/// 与 [`ApprovalGate::decide`] 负责。
pub fn derive_effect(
    req: &ApprovalRequest,
    _now: DateTime<Utc>,
    _ttl: Duration,
) -> GateNodeEffect {
    match req.status {
        ApprovalStatus::Approved => GateNodeEffect::Release,
        ApprovalStatus::Rejected => GateNodeEffect::Fail,
        // Pending（含已过期但尚未被惰性翻转的那一拍）与 Expired 都挂起，等待决策/续期。
        ApprovalStatus::Pending | ApprovalStatus::Expired => GateNodeEffect::Hold,
    }
}

/// DAG 派发循环对一个「依赖已满足、本可执行」节点应采取的动作（派发前包装判定）。
///
/// 这是审批门接入执行器的**纯接缝判定**：执行器的 `schedule_ready_nodes` 在把就绪节点
/// 标 Running 之前，应先问一次 [`ApprovalGate::dispatch_decision`]：
/// - [`DispatchDecision::Dispatch`] → 维持现有路径，标 Running 并派发；
/// - [`DispatchDecision::Hold`] → 本 tick 跳过该节点（沿用现有「不派发即挂起」语义，
///   不新增 NodeStatus 变体、不改节点状态），下一轮再问；
/// - [`DispatchDecision::Fail`] → 把节点标记 Failed（映射 `NodeStatus::Failed`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchDecision {
    Dispatch,
    Hold,
    Fail,
}

/// 审批决策失败原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeciseError {
    /// 该 (task, node) 没有挂起的审批请求。
    NotFound,
    /// 已决策过（幂等：不报错、不改写，由调用方决定是否接受）。
    AlreadyDecided,
}

/// 内存审批门：submit / list_pending / decide / reap_expired。
///
/// 非线程安全——调用方负责互斥（对齐本 crate `RackAggregator` 等纯逻辑模块的约定）。
pub struct ApprovalGate {
    ttl: Duration,
    /// key = `(task_id, node_id)` 拼接，同一节点同时只挂一份有效审批。
    items: BTreeMap<String, ApprovalRequest>,
}

impl Default for ApprovalGate {
    fn default() -> Self {
        Self::new(Duration::hours(24))
    }
}

impl ApprovalGate {
    /// `ttl`：一条 Pending 审批超过多久未决即视为 Expired（仍 Hold，需重提）。
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            items: BTreeMap::new(),
        }
    }

    fn key(task_id: &str, node_id: &str) -> String {
        format!("{task_id}\u{1}{node_id}")
    }

    /// 提交一条人审请求。若同一 (task, node) 已有未决 Pending，重复提交视为幂等：
    /// 返回既有请求（不刷新 requested_at、不产生第二条）。
    pub fn submit(
        &mut self,
        task_id: impl Into<String>,
        node_id: impl Into<String>,
        reason: impl Into<String>,
        now: DateTime<Utc>,
    ) -> &ApprovalRequest {
        let task_id = task_id.into();
        let node_id = node_id.into();
        let key = Self::key(&task_id, &node_id);
        // 幂等：已存在 Pending 就不重复插入（先结束不可变借用再决定是否写入）。
        let already_pending = self
            .items
            .get(&key)
            .map(|r| r.status == ApprovalStatus::Pending)
            .unwrap_or(false);
        if !already_pending {
            self.items.insert(
                key.clone(),
                ApprovalRequest {
                    task_id,
                    node_id,
                    reason: reason.into(),
                    requested_at: now,
                    status: ApprovalStatus::Pending,
                    decided_at: None,
                },
            );
        }
        self.items.get(&key).expect("just inserted")
    }

    /// 列出所有「仍在等待人决策」的审批（Pending 且未过期）。
    /// 此调用会顺手把已过期的 Pending 翻成 Expired（惰性过期）。
    pub fn list_pending(&mut self, now: DateTime<Utc>) -> Vec<&ApprovalRequest> {
        for req in self.items.values_mut() {
            if req.status == ApprovalStatus::Pending && req.is_expired(now, self.ttl) {
                req.status = ApprovalStatus::Expired;
            }
        }
        self.items
            .values()
            .filter(|r| r.status == ApprovalStatus::Pending)
            .collect()
    }

    /// 批准 / 拒绝一条审批。幂等：若该请求已决策（Approved/Rejected/Expired），
    /// 不报错、不改写，返回 [`DeciseError::AlreadyDecided`]。
    pub fn decide(
        &mut self,
        task_id: &str,
        node_id: &str,
        approve: bool,
        now: DateTime<Utc>,
    ) -> Result<&ApprovalRequest, DeciseError> {
        let key = Self::key(task_id, node_id);
        let req = self.items.get_mut(&key).ok_or(DeciseError::NotFound)?;
        match req.status {
            ApprovalStatus::Approved | ApprovalStatus::Rejected | ApprovalStatus::Expired => {
                return Err(DeciseError::AlreadyDecided);
            }
            ApprovalStatus::Pending => {}
        }
        // 过期了不允许在事后追认批准——调用方应重新提交。
        if req.is_expired(now, self.ttl) {
            req.status = ApprovalStatus::Expired;
            return Err(DeciseError::AlreadyDecided);
        }
        req.status = if approve {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Rejected
        };
        req.decided_at = Some(now);
        Ok(self.items.get(&key).expect("just decided"))
    }

    /// 推导某节点当前应被调度器如何对待（Hold/Release/Fail）。
    pub fn effect_of(&self, task_id: &str, node_id: &str, now: DateTime<Utc>) -> GateNodeEffect {
        match self.items.get(&Self::key(task_id, node_id)) {
            Some(req) => derive_effect(req, now, self.ttl),
            None => GateNodeEffect::Release, // 没人审 → 不阻塞，默认放行
        }
    }

    /// 派发前包装判定：执行器 `schedule_ready_nodes` 在把就绪节点标 Running 前调用。
    /// 无审批挂起 → Dispatch（不阻塞既有 DAG）；pending/过期 → Hold（本 tick 跳过）；
    /// 拒绝 → Fail（标 NodeStatus::Failed）。
    pub fn dispatch_decision(
        &self,
        task_id: &str,
        node_id: &str,
        now: DateTime<Utc>,
    ) -> DispatchDecision {
        match self.effect_of(task_id, node_id, now) {
            GateNodeEffect::Release => DispatchDecision::Dispatch,
            GateNodeEffect::Hold => DispatchDecision::Hold,
            GateNodeEffect::Fail => DispatchDecision::Fail,
        }
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// 读取一条审批请求（不存在返回 None）。
    pub fn get(&self, task_id: &str, node_id: &str) -> Option<&ApprovalRequest> {
        self.items.get(&Self::key(task_id, node_id))
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn t(minutes: i64) -> DateTime<Utc> {
        Utc::now() - Duration::minutes(minutes)
    }

    #[test]
    fn submit_then_lists_pending() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        gate.submit("task-1", "node-a", "需人审", now);
        let pending = gate.list_pending(now);
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].node_id, "node-a");
        assert_eq!(pending[0].status, ApprovalStatus::Pending);
    }

    #[test]
    fn approve_releases_node() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        gate.submit("task-1", "node-a", "需人审", now);
        gate.decide("task-1", "node-a", true, now).unwrap();
        // 批准后 effect = Release，且映射到 NodeStatus::Ready
        assert_eq!(gate.effect_of("task-1", "node-a", now), GateNodeEffect::Release);
        assert_eq!(GateNodeEffect::Release.to_node_status(), Some(NodeStatus::Ready));
        // 已决策后不再出现在 pending 列表
        assert!(gate.list_pending(now).is_empty());
    }

    #[test]
    fn reject_fails_node() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        gate.submit("task-1", "node-a", "需人审", now);
        gate.decide("task-1", "node-a", false, now).unwrap();
        assert_eq!(gate.effect_of("task-1", "node-a", now), GateNodeEffect::Fail);
        assert_eq!(GateNodeEffect::Fail.to_node_status(), Some(NodeStatus::Failed));
    }

    #[test]
    fn expired_request_holds_node() {
        let submitted_at = t(120); // 2 小时前
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1)); // TTL 1h
        gate.submit("task-1", "node-a", "需人审", submitted_at);
        // 到期后 list_pending 惰性翻成 Expired
        assert!(gate.list_pending(now).is_empty());
        // 过期仍 Hold（不自动放行，也不算失败）
        assert_eq!(gate.effect_of("task-1", "node-a", now), GateNodeEffect::Hold);
        assert_eq!(GateNodeEffect::Hold.to_node_status(), None);
        // 过期后不允许事后追认批准
        assert_eq!(
            gate.decide("task-1", "node-a", true, now),
            Err(DeciseError::AlreadyDecided)
        );
    }

    #[test]
    fn double_decide_is_idempotent() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        gate.submit("task-1", "node-a", "需人审", now);
        gate.decide("task-1", "node-a", true, now).unwrap();
        let decided_at_before = gate.get("task-1", "node-a").unwrap().decided_at;
        // 第二次决策（即使改成 reject）必须被幂等拒绝，状态保持第一次
        assert_eq!(
            gate.decide("task-1", "node-a", false, now),
            Err(DeciseError::AlreadyDecided)
        );
        // 第三次（仍 approve）也必须被幂等拒绝——已决策不可改
        assert_eq!(
            gate.decide("task-1", "node-a", true, now),
            Err(DeciseError::AlreadyDecided)
        );
        let again = gate.get("task-1", "node-a").unwrap();
        assert_eq!(again.status, ApprovalStatus::Approved);
        assert_eq!(again.decided_at, decided_at_before);
    }

    #[test]
    fn empty_pending_list_and_unknown_node_releases() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        assert!(gate.list_pending(now).is_empty());
        // 没有人审挂起的节点 → 默认放行（不阻塞 DAG）
        assert_eq!(gate.effect_of("task-x", "node-y", now), GateNodeEffect::Release);
        // 对不存在的审批 decide 返回 NotFound
        assert_eq!(
            gate.decide("task-x", "node-y", true, now),
            Err(DeciseError::NotFound)
        );
    }

    #[test]
    fn dispatch_decision_maps_gate_state() {
        let now = Utc::now();
        let mut gate = ApprovalGate::new(Duration::hours(1));
        // 无人审 → Dispatch
        assert_eq!(gate.dispatch_decision("t", "n-a", now), DispatchDecision::Dispatch);
        // pending 未决 → Hold
        gate.submit("t", "n-h", "需人审", now);
        assert_eq!(gate.dispatch_decision("t", "n-h", now), DispatchDecision::Hold);
        // 批准 → Dispatch
        gate.decide("t", "n-h", true, now).unwrap();
        assert_eq!(gate.dispatch_decision("t", "n-h", now), DispatchDecision::Dispatch);
        // 拒绝 → Fail
        gate.submit("t", "n-f", "需人审", now);
        gate.decide("t", "n-f", false, now).unwrap();
        assert_eq!(gate.dispatch_decision("t", "n-f", now), DispatchDecision::Fail);
    }
}
