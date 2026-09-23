// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! node→rack→cell 分级心跳聚合（10:1:1）与注册中心侧续约语义。
//!
//! 流量数学：设节点数 N、单节点心跳周期 L（租约内）、rack fan-in B、
//! cell 发布周期 C 个 rack-digest：
//!   - 平铺：N/L 次/秒 直达注册中心
//!   - 分级：叶→rack 本地转发 N/L（进程内/同机），rack→cell N/(L·B)，cell→注册中心 N/(L·B·C)
//!
//! 10:1:1 即 B=10、每 10 条 rack-digest 合成 1 条 cell 续约 → 注册中心入流降 100 倍。
//!
//! 诚实边界：rack 代理宕机时其成员失去续约来源，靠注册中心租约摘除兜底；
//! 聚合链最坏发布间隔（[`max_aggregation_gap_seconds`]）必须显著小于注册中心
//! 侧 `lease_seconds`，否则聚合自身会触发误摘——该约束由部署方经容量模型校验。

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use mox_alliance_registry_proto::types::{InstanceStatus, RegisteredInstance};
use serde::{Deserialize, Serialize};

/// 叶级心跳（节点 → 本 rack 聚合器）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeBeat {
    pub member_id: String,
    #[serde(default)]
    pub load_current: Option<u32>,
    /// 显式上报状态（如 Draining）；`None` = Active（心跳到达即视为活跃）
    #[serde(default)]
    pub status: Option<MemberStatus>,
}

/// 聚合链内部使用的成员状态（与 proto `InstanceStatus` 区分：
/// 叶级只有 Active/Unhealthy 两态可上报，Draining 也走显式 status）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    Active,
    Unhealthy,
    Draining,
}

impl MemberStatus {
    pub fn to_instance_status(self) -> InstanceStatus {
        match self {
            MemberStatus::Active => InstanceStatus::Active,
            MemberStatus::Unhealthy => InstanceStatus::Unhealthy,
            MemberStatus::Draining => InstanceStatus::Draining,
        }
    }
}

/// rack 级聚合摘要（rack → cell 层，1 条替代 fan_in 条叶心跳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDigest {
    pub group_id: String,
    pub cell_id: String,
    pub seq: u64,
    /// 摘要生成时刻（注入时钟产出）
    pub reported_at: DateTime<Utc>,
    #[serde(default)]
    pub members: BTreeMap<String, Renewal>,
    pub member_count: usize,
    pub unhealthy_count: u32,
    pub load_total: u64,
    pub health: AggregationHealth,
    /// 聚合器中已消失（注销/淘汰）但注册中心可能仍持有的成员
    #[serde(default)]
    pub unknown_members: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregationHealth {
    Healthy,
    Degraded,
    Suspect,
}

/// 单成员续约条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Renewal {
    pub load_current: Option<u32>,
    pub status: MemberStatus,
}

/// 发送到注册中心的聚合续约载荷（cell 层合并全部 rack 摘要后的发布单元）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedRenewal {
    pub group_id: String,
    pub seq: u64,
    pub reported_at: DateTime<Utc>,
    pub members: BTreeMap<String, Renewal>,
    #[serde(default)]
    pub unknown_members: Vec<String>,
    pub health: AggregationHealth,
    pub member_count: usize,
    pub unhealthy_count: u32,
    pub load_total: u64,
}

impl AggregatedRenewal {
    pub fn from_digest(d: &GroupDigest) -> Self {
        Self {
            group_id: d.group_id.clone(),
            seq: d.seq,
            reported_at: d.reported_at,
            members: d.members.clone(),
            unknown_members: d.unknown_members.clone(),
            health: d.health,
            member_count: d.member_count,
            unhealthy_count: d.unhealthy_count,
            load_total: d.load_total,
        }
    }
}

// ─── rack 层聚合器 ─────────────────────────────────────────────────────────

/// rack 聚合器配置
#[derive(Debug, Clone)]
pub struct RackConfig {
    pub rack_id: String,
    pub cell_id: String,
    /// fan-in：每收集多少条去重后的成员心跳发布一条 rack 摘要
    pub fan_in: usize,
    /// 叶级成员默认租约（秒）
    pub node_lease: i64,
}

impl Default for RackConfig {
    fn default() -> Self {
        Self {
            rack_id: "rack-0".into(),
            cell_id: "cell-0".into(),
            fan_in: 10,
            node_lease: 15,
        }
    }
}

/// 可注入时钟（对齐 `RegistryStore::reap_expired_at` 的确定性测试约定）
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

fn system_clock() -> Clock {
    Arc::new(Utc::now)
}

/// node→rack 聚合器。心跳按成员去重：一个发布周期内同一成员多次心跳只计一条，
/// 到期合并为单条 [`GroupDigest`]。非线程安全——调用方负责互斥（如 `Arc<Mutex>`）。
pub struct RackAggregator {
    cfg: RackConfig,
    members: BTreeMap<String, Renewal>,
    /// 自上次摘要发布以来收到的（去重）心跳数
    since_publish: usize,
    seq: u64,
    /// 聚合器中已消失、待随下一份摘要上报给注册中心清理的成员
    unknown_pending: Vec<String>,
    now: Clock,
}

impl RackAggregator {
    pub fn new(cfg: RackConfig) -> Self {
        Self::with_clock(cfg, system_clock())
    }

    pub fn with_clock(cfg: RackConfig, now: Clock) -> Self {
        assert!(cfg.fan_in > 0, "fan_in 必须为正");
        Self {
            cfg,
            members: BTreeMap::new(),
            since_publish: 0,
            seq: 0,
            unknown_pending: Vec::new(),
            now,
        }
    }

    /// 登记成员（等价于一次隐式 Active 心跳，立即计入待发批）。
    /// 注册突发不主动触发发布——由后续心跳凑满 fan_in 或周期 [`flush`](Self::flush) 送出。
    pub fn register(&mut self, member_id: &str, load: Option<u32>, status: MemberStatus) {
        if self.track(member_id, load, status) {
            self.since_publish += 1;
        }
    }

    /// 叶级心跳。返回 `Some(digest)` 表示本批 fan_in 已凑满、应向上发布。
    pub fn heartbeat(&mut self, beat: &NodeBeat) -> Option<GroupDigest> {
        let status = beat.status.unwrap_or(MemberStatus::Active);
        let fresh = self.track(&beat.member_id, beat.load_current, status);
        if fresh {
            self.since_publish += 1;
        }
        if self.since_publish >= self.cfg.fan_in {
            Some(self.publish_digest())
        } else {
            None
        }
    }

    /// 成员注销：移出聚合并记入 unknown（注册中心侧可据此主动摘除）。
    pub fn forget_member(&mut self, member_id: &str) {
        if self.members.remove(member_id).is_some() {
            self.unknown_pending.push(member_id.to_string());
        }
    }

    /// 强制立即发布当前聚合状态（周期 tick 用）。
    pub fn flush(&mut self) -> GroupDigest {
        self.publish_digest()
    }

    /// 已登记成员数
    pub fn tracked(&self) -> usize {
        self.members.len()
    }

    pub fn config(&self) -> &RackConfig {
        &self.cfg
    }

    fn track(&mut self, id: &str, load: Option<u32>, status: MemberStatus) -> bool {
        let fresh = !self.members.contains_key(id);
        self.members
            .insert(id.to_string(), Renewal { load_current: load, status });
        fresh
    }

    fn publish_digest(&mut self) -> GroupDigest {
        self.seq += 1;
        let unhealthy_count = self
            .members
            .values()
            .filter(|r| r.status == MemberStatus::Unhealthy)
            .count() as u32;
        let load_total: u64 = self
            .members
            .values()
            .filter_map(|r| r.load_current)
            .sum::<u32>() as u64;
        let health = if self.members.is_empty() {
            AggregationHealth::Suspect
        } else if unhealthy_count > 0 {
            AggregationHealth::Degraded
        } else {
            AggregationHealth::Healthy
        };
        self.since_publish = 0;
        let unknown_members = std::mem::take(&mut self.unknown_pending);
        GroupDigest {
            group_id: self.cfg.rack_id.clone(),
            cell_id: self.cfg.cell_id.clone(),
            seq: self.seq,
            reported_at: (self.now)(),
            members: self.members.clone(),
            member_count: self.members.len(),
            unhealthy_count,
            load_total,
            health,
            unknown_members,
        }
    }
}

// ─── cell 层聚合器 ─────────────────────────────────────────────────────────

/// rack→cell 聚合器：收集本 cell 各 rack 的最新摘要，每 `cell_batches` 条
/// rack-digest（或显式 [`CellAggregator::publish`]）合并为一次注册中心续约。
pub struct CellAggregator {
    cell_id: String,
    latest: HashMap<String, GroupDigest>,
    since_publish: usize,
    cell_batches: usize,
    now: Clock,
}

impl CellAggregator {
    pub fn new(cell_id: impl Into<String>, cell_batches: usize) -> Self {
        Self::with_clock(cell_id, cell_batches, system_clock())
    }

    pub fn with_clock(
        cell_id: impl Into<String>,
        cell_batches: usize,
        now: Clock,
    ) -> Self {
        assert!(cell_batches > 0, "cell_batches 必须为正");
        Self {
            cell_id: cell_id.into(),
            latest: HashMap::new(),
            since_publish: 0,
            cell_batches,
            now,
        }
    }

    /// 收录一条 rack 摘要（按 `seq` 单调，旧摘要忽略）。返回 `Some(renewal)`
    /// 表示达到发布批，应向注册中心提交。
    pub fn ingest(&mut self, digest: GroupDigest) -> Option<AggregatedRenewal> {
        if digest.cell_id != self.cell_id {
            return None;
        }
        let slot = self
            .latest
            .entry(digest.group_id.clone())
            .or_insert_with(|| digest.clone());
        if digest.seq >= slot.seq {
            *slot = digest;
        }
        self.since_publish += 1;
        if self.since_publish >= self.cell_batches {
            Some(self.publish())
        } else {
            None
        }
    }

    /// 强制合并发布（周期 tick 用）。
    pub fn publish(&mut self) -> AggregatedRenewal {
        self.since_publish = 0;
        let mut groups: Vec<GroupDigest> = self.latest.values().cloned().collect();
        groups.sort_by(|a, b| a.group_id.cmp(&b.group_id));
        let mut members = BTreeMap::new();
        let mut unknown = Vec::new();
        let mut count = 0usize;
        let mut unhealthy = 0u32;
        let mut load = 0u64;
        let mut seq_sum = 0u64;
        let mut reported_at = (self.now)();
        let mut health = AggregationHealth::Healthy;
        for g in &groups {
            for (id, r) in &g.members {
                members.insert(id.clone(), r.clone());
            }
            unknown.extend(g.unknown_members.iter().cloned());
            count += g.member_count;
            unhealthy += g.unhealthy_count;
            load += g.load_total;
            seq_sum = seq_sum.saturating_add(g.seq);
            if g.reported_at < reported_at {
                reported_at = g.reported_at;
            }
            health = match (health, g.health) {
                (AggregationHealth::Suspect, _) | (_, AggregationHealth::Suspect) => {
                    AggregationHealth::Suspect
                }
                (AggregationHealth::Degraded, _) | (_, AggregationHealth::Degraded) => {
                    AggregationHealth::Degraded
                }
                _ => AggregationHealth::Healthy,
            };
        }
        AggregatedRenewal {
            group_id: self.cell_id.clone(),
            seq: seq_sum,
            reported_at,
            members,
            unknown_members: unknown,
            health,
            member_count: count,
            unhealthy_count: unhealthy,
            load_total: load,
        }
    }

    /// 已知 rack 数
    pub fn racks(&self) -> usize {
        self.latest.len()
    }
}

// ─── 注册中心侧应用语义 ────────────────────────────────────────────────────

/// 注册中心侧聚合续约应用器（作用于调用方持有的实例视图，零 IO）。
pub struct RegistryApplier;

impl RegistryApplier {
    /// 应用聚合续约：仅刷新已注册成员（聚合不隐式注册）；
    /// 聚合器已消失的 `unknown_members` 一并摘除（显式注销传播）。
    /// 返回（续约成功数，注册中心中不存在的成员数）。
    pub fn apply_renewal(
        instances: &mut HashMap<String, RegisteredInstance>,
        r: &AggregatedRenewal,
        now: DateTime<Utc>,
    ) -> (usize, usize) {
        let mut renewed = 0;
        let mut unknown = 0;
        for (id, ren) in &r.members {
            match instances.get_mut(id) {
                Some(inst) => {
                    inst.last_heartbeat_at = r.reported_at.min(now);
                    if let Some(load) = ren.load_current {
                        inst.load_current = load;
                    }
                    inst.status = ren.status.to_instance_status();
                    renewed += 1;
                }
                None => unknown += 1,
            }
        }
        for id in &r.unknown_members {
            if instances.remove(id).is_some() {
                unknown += 1;
            }
        }
        (renewed, unknown)
    }

    /// rack 停报（聚合代理失联）：把该组仍存活的成员标记 Unhealthy，
    /// 租约到期后由 reaper 摘除。返回被标记的数量。
    pub fn mark_group_suspect(
        instances: &mut HashMap<String, RegisteredInstance>,
        group_id: &str,
    ) -> usize {
        let mut marked = 0;
        for inst in instances.values_mut() {
            let Some(g) = inst.metadata.get("rack") else { continue };
            if g == group_id && inst.status == InstanceStatus::Active {
                inst.status = InstanceStatus::Unhealthy;
                marked += 1;
            }
        }
        marked
    }
}

/// 聚合链发布的最大间隔（秒）：`fan_in` 条叶心跳 × cell 合并批数。
/// 配置校验用：注册中心租约必须显著大于该值，否则聚合本身导致误摘。
pub fn max_aggregation_gap_seconds(rack: &RackAggregator, cell_batches: usize) -> i64 {
    let lease = rack.config().node_lease.max(1);
    // 保守界：满编 fan_in 心跳的最坏间隔 ≈ 一个叶租约；cell 再乘合并批数
    lease.saturating_mul(cell_batches.max(1) as i64)
}

// ─── 单元测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn fixed(t: DateTime<Utc>) -> Clock {
        Arc::new(move || t)
    }

    fn beat(id: &str) -> NodeBeat {
        NodeBeat {
            member_id: id.into(),
            load_current: Some(1),
            status: None,
        }
    }

    #[test]
    fn fan_in_batches_heartbeat_traffic_10_to_1() {
        // 10:1：10 条去重叶心跳 → 恰好 1 条 rack 摘要
        let t = Utc::now();
        let mut rack = RackAggregator::with_clock(
            RackConfig { fan_in: 10, ..Default::default() },
            fixed(t),
        );
        let mut digests = 0;
        let mut last: Option<GroupDigest> = None;
        for i in 0..100 {
            if let Some(d) = rack.heartbeat(&beat(&format!("n{i:03}"))) {
                digests += 1;
                last = Some(d);
            }
        }
        assert_eq!(digests, 10, "100 条叶心跳应聚成 10 条 rack 摘要");
        // 注入时钟必须体现在摘要的 reported_at 上
        assert_eq!(last.expect("有摘要").reported_at, t);
    }

    #[test]
    fn duplicate_beats_in_window_count_once() {
        let mut rack = RackAggregator::new(RackConfig { fan_in: 3, ..Default::default() });
        assert!(rack.heartbeat(&beat("a")).is_none());
        assert!(rack.heartbeat(&beat("a")).is_none()); // 重复，不计数
        assert!(rack.heartbeat(&beat("b")).is_none());
        let d = rack.heartbeat(&beat("c")).expect("3 个不同成员应触发发布");
        assert_eq!(d.member_count, 3);
    }

    #[test]
    fn cell_merges_racks_10_to_1_total_100x() {
        // 10:1:1 —— 10 rack × fan_in 10 = 100 条叶心跳 → 1 条 cell 续约
        let mut rack = RackAggregator::new(RackConfig {
            rack_id: "r-1".into(),
            cell_id: "c-1".into(),
            fan_in: 10,
            ..Default::default()
        });
        let mut cell = CellAggregator::new("c-1", 10);
        let mut renewal = None;
        for i in 0..100 {
            if let Some(d) = rack.heartbeat(&beat(&format!("n{i:03}"))) {
                if let Some(r) = cell.ingest(d) {
                    renewal = Some(r);
                }
            }
        }
        let r = renewal.expect("10 条 rack 摘要应触发 1 条 cell 续约");
        assert_eq!(r.members.len(), 100, "cell 续约应合并全部成员");
        assert_eq!(r.member_count, 100);
        assert_eq!(r.health, AggregationHealth::Healthy);
    }

    #[test]
    fn stale_digest_seq_ignored() {
        let mut cell = CellAggregator::new("c-1", 1);
        let d_new = GroupDigest {
            group_id: "r-1".into(),
            cell_id: "c-1".into(),
            seq: 5,
            reported_at: Utc::now(),
            members: BTreeMap::from([("x".to_string(), Renewal { load_current: None, status: MemberStatus::Active })]),
            member_count: 1,
            unhealthy_count: 0,
            load_total: 0,
            health: AggregationHealth::Healthy,
            unknown_members: vec![],
        };
        cell.ingest(d_new.clone());
        let mut d_old = d_new;
        d_old.seq = 3;
        d_old.members = BTreeMap::from([("stale".to_string(), Renewal { load_current: None, status: MemberStatus::Unhealthy })]);
        cell.ingest(d_old);
        let r = cell.publish();
        assert!(r.members.contains_key("x") && !r.members.contains_key("stale"));
    }

    #[test]
    fn applier_renews_only_registered_and_reports_unknown() {
        let t = Utc::now();
        let mut inst = RegisteredInstance {
            id: "m1".into(),
            name: "n".into(),
            version: "1".into(),
            endpoint: "http://x".into(),
            health_check_url: None,
            capabilities: vec![],
            domain: None,
            weight: 1.0,
            load_current: 0,
            load_capacity: 10,
            status: InstanceStatus::Unhealthy,
            registered_at: t - Duration::seconds(30),
            last_heartbeat_at: t - Duration::seconds(30),
            lease_seconds: 15,
            metadata: Default::default(),
        };
        inst.status = InstanceStatus::Active;
        let mut map = HashMap::new();
        map.insert("m1".to_string(), inst);

        let renewal = AggregatedRenewal {
            group_id: "r-1".into(),
            seq: 1,
            reported_at: t,
            members: BTreeMap::from([
                ("m1".to_string(), Renewal { load_current: Some(7), status: MemberStatus::Active }),
                ("ghost".to_string(), Renewal { load_current: None, status: MemberStatus::Active }),
            ]),
            unknown_members: vec!["gone".into()],
            health: AggregationHealth::Healthy,
            member_count: 2,
            unhealthy_count: 0,
            load_total: 7,
        };
        let (ok, unknown) = RegistryApplier::apply_renewal(&mut map, &renewal, t);
        assert_eq!(ok, 1);
        assert_eq!(unknown, 1);
        let m1 = map.get("m1").expect("m1 保留");
        assert_eq!(m1.last_heartbeat_at, t);
        assert_eq!(m1.load_current, 7);
        assert_eq!(m1.status, InstanceStatus::Active);
    }

    #[test]
    fn applier_marks_suspect_group_unhealthy() {
        let t = Utc::now();
        let mk = |id: &str, rack: Option<&str>| {
            let mut inst = RegisteredInstance {
                id: id.into(),
                name: id.into(),
                version: "1".into(),
                endpoint: "http://x".into(),
                health_check_url: None,
                capabilities: vec![],
                domain: None,
                weight: 1.0,
                load_current: 0,
                load_capacity: 10,
                status: InstanceStatus::Active,
                registered_at: t,
                last_heartbeat_at: t,
                lease_seconds: 15,
                metadata: Default::default(),
            };
            if let Some(r) = rack {
                inst.metadata.insert("rack".into(), r.into());
            }
            inst
        };
        let mut map = HashMap::new();
        map.insert("a".to_string(), mk("a", Some("r-1")));
        map.insert("b".to_string(), mk("b", Some("r-2")));
        map.insert("c".to_string(), mk("c", None));
        let marked = RegistryApplier::mark_group_suspect(&mut map, "r-1");
        assert_eq!(marked, 1);
        assert_eq!(map["a"].status, InstanceStatus::Unhealthy);
        assert_eq!(map["b"].status, InstanceStatus::Active);
    }

    #[test]
    fn forget_member_propagates_unknown_to_prune() {
        let mut rack = RackAggregator::new(RackConfig { fan_in: 2, ..Default::default() });
        rack.register("x", None, MemberStatus::Active);
        rack.forget_member("x");
        let d = rack.flush();
        assert_eq!(d.unknown_members, vec!["x".to_string()]);
        assert!(d.members.is_empty());
        assert_eq!(d.health, AggregationHealth::Suspect);
    }
}
