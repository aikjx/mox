// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器多活：租约选主（lease-based leader election）与故障接管。
//!
//! 见 `docs/architecture/microservices/07-massive-scale-100k-nodes.md` §五「调度器水平扩展」。
//! 调度器任务状态已外置到 [`TaskRepository`](crate::storage::TaskRepository)，副本本身无状态，
//! 因此**请求路径**（submit/cancel/get）可随意 LB 分发，不需要选主。
//! 需要「唯一持有者」的是**周期型职责**（例如
//! [`reconcile_active_tasks`](crate::TaskSchedulerImpl::reconcile_active_tasks)：
//! 扫全表 + 逐个向执行器对账 + 判定孤儿任务）：N 个副本各自跑同一条扫表对账，
//! 会对执行器产生 N 倍轮询，且并发写同一任务的终态会互相覆盖。
//! 本模块给出一个租约选主状态机，让该类职责只在 leader 上跑，并在 leader 宕机后被 standby 接管。
//!
//! ## 三条正确性约束
//! 1. **互斥**：同一 `scope` 在任意时刻至多一个持有者（由 [`LeaseStore`] 的 CAS 决定，
//!    SQLite 实现靠 `BEGIN IMMEDIATE` 跨进程串行化写者）。
//! 2. **故障接管**：leader 停止续约后，其租约到期，standby 在下一个 tick 抢占成功，
//!    接管时延上界 = `lease`（无需额外探测协议）。
//! 3. **fencing token**：每次成功抢占都会令 `epoch` +1。旧 leader 假死复活后拿到的必然是
//!    更大的新 epoch，其携带旧 epoch 的动作可被 [`Leadership::accepts`] 拒绝——
//!    这是"分区/长 GC 停顿后旧 leader 迟到的写"唯一可靠的兜底。
//!
//! 诚实边界：租约选主只保证**互斥**，不保证**无重复执行**（leader 在两步之间停顿、
//! 租约过期被接管后，旧 leader 已开始的那一轮仍可能跑完）。因此交由 leader 执行的职责
//! 必须自身幂等（对账写是幂等 upsert、状态转换由协议层转换表仲裁）。跨机部署时
//! 各副本时钟偏移会直接进入接管时延，故 `lease` 应显著大于 NTP 同步误差。

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use mox_alliance_common_proto::AllianceResult;
use serde::{Deserialize, Serialize};

/// 可注入时钟（测试用固定时间，避免 sleep 造成的抖动）。
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

fn system_clock() -> Clock {
    Arc::new(Utc::now)
}

fn deadline(now: DateTime<Utc>, lease: Duration) -> DateTime<Utc> {
    // 毫秒精度足够（租约量级为秒），并避免 Duration 溢出的错误分支。
    let ms = i64::try_from(lease.as_millis()).unwrap_or(i64::MAX);
    now + ChronoDuration::milliseconds(ms)
}

/// 一个选主域（scope）的租约快照。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LeaseState {
    /// 当前持有者；`None` 表示无人认领（从未选举或已让位）。
    pub holder: Option<String>,
    /// 任期号：每次成功抢占 +1，只增不减（fencing token）。
    pub epoch: u64,
    /// 租约到期时刻；`None` 当且仅当 `holder` 为 `None`。
    pub expires_at: Option<DateTime<Utc>>,
}

impl LeaseState {
    /// 在 `now` 时刻是否有人持有（未过期）。
    pub fn is_held_at(&self, now: DateTime<Utc>) -> bool {
        self.holder.is_some() && self.expires_at.is_some_and(|t| t > now)
    }

    /// 在 `now` 时刻是否由 `who` 持有。
    pub fn held_by(&self, who: &str, now: DateTime<Utc>) -> bool {
        self.is_held_at(now) && self.holder.as_deref() == Some(who)
    }
}

/// 单次租约 CAS 决策（纯函数，两种 [`LeaseStore`] 实现共用同一语义）。
///
/// 返回 `Some(新状态)` 表示需要写入；`None` 表示他人仍持有、本次竞选失败且**不得**
/// 改动存储（失败者不能抬高 epoch，否则空转即可制造任期风暴）。
pub fn decide(
    cur: &LeaseState,
    who: &str,
    now: DateTime<Utc>,
    lease: Duration,
) -> Option<LeaseState> {
    if cur.held_by(who, now) {
        // 续约：任期不变，只延长到期时刻。
        return Some(LeaseState {
            holder: Some(who.to_string()),
            epoch: cur.epoch,
            expires_at: Some(deadline(now, lease)),
        });
    }
    if cur.is_held_at(now) {
        // 他人持有且未过期 → 落选，不写。
        return None;
    }
    // 无人持有，或持有者租约已过期（含本人过期后重来）→ 抢占，任期 +1。
    Some(LeaseState {
        holder: Some(who.to_string()),
        epoch: cur.epoch + 1,
        expires_at: Some(deadline(now, lease)),
    })
}

/// 租约存储：选主的权威仲裁点。
///
/// 实现必须让 [`LeaseStore::campaign`] 的「读-判-写」不可分割：
/// [`MemoryLeaseStore`] 靠互斥锁，[`SqliteLeaseStore`](crate::storage::SqliteLeaseStore)
/// 靠 SQLite 写事务（`BEGIN IMMEDIATE` + `busy_timeout`）跨进程串行化。
pub trait LeaseStore: Send + Sync {
    /// 只读快照（不存在时返回默认空状态）。
    fn state(&self, scope: &str) -> AllianceResult<LeaseState>;

    /// 竞选/续约 `scope` 的领导权，返回操作后的权威状态。
    fn campaign(
        &self,
        scope: &str,
        holder: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> AllianceResult<LeaseState>;

    /// 主动让位：仅持有者本人的释放生效（幂等，非持有者调用为无操作）。
    fn release(&self, scope: &str, holder: &str) -> AllianceResult<()>;
}

/// 进程内存租约存储：单进程多副本仿真与单元测试的参考实现。
#[derive(Default)]
pub struct MemoryLeaseStore {
    leases: Mutex<BTreeMap<String, LeaseState>>,
}

impl MemoryLeaseStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LeaseStore for MemoryLeaseStore {
    fn state(&self, scope: &str) -> AllianceResult<LeaseState> {
        Ok(self
            .leases
            .lock()
            .unwrap()
            .get(scope)
            .cloned()
            .unwrap_or_default())
    }

    fn campaign(
        &self,
        scope: &str,
        holder: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> AllianceResult<LeaseState> {
        let mut leases = self.leases.lock().unwrap();
        let cur = leases.get(scope).cloned().unwrap_or_default();
        let next = decide(&cur, holder, now, lease).unwrap_or(cur);
        leases.insert(scope.to_string(), next.clone());
        Ok(next)
    }

    fn release(&self, scope: &str, holder: &str) -> AllianceResult<()> {
        let mut leases = self.leases.lock().unwrap();
        if let Some(cur) = leases.get_mut(scope) {
            if cur.holder.as_deref() == Some(holder) {
                // 保留 epoch：让位不得让后续任期回退。
                *cur = LeaseState {
                    holder: None,
                    epoch: cur.epoch,
                    expires_at: None,
                };
            }
        }
        Ok(())
    }
}

/// 一轮 `step()` 后的领导权视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Leadership {
    /// 本副本是否为 leader。
    pub leader: bool,
    /// 本副本当前认定的任期号（落选时为 observed 的 epoch，用于只读诊断）。
    pub epoch: u64,
    /// 本副本租约到期时刻（非 leader 时为 `None`）。
    pub expires_at: Option<DateTime<Utc>>,
}

impl Leadership {
    /// fencing 校验：只有本轮仍持有的任期才放行写动作。
    ///
    /// 用法：leader 在开始一轮职责前记下 `epoch`，提交副作用时用本方法与最新
    /// `leadership()` 比对；不相等说明期间已被接管，必须丢弃这一轮。
    pub fn accepts(&self, epoch: u64) -> bool {
        self.leader && self.epoch == epoch
    }
}

/// 租约选主状态机：每次 [`step`](Self::step) 完成一次「竞选或续约」。
///
/// 调用方（服务层的周期任务）按 `lease/2` 的节奏驱动它；本类型不做 IO 也不起线程。
pub struct LeaderElector {
    store: Arc<dyn LeaseStore>,
    scope: String,
    holder: String,
    lease: Duration,
    now: Clock,
    view: Leadership,
}

impl LeaderElector {
    pub fn new(
        store: Arc<dyn LeaseStore>,
        scope: impl Into<String>,
        holder: impl Into<String>,
        lease: Duration,
    ) -> Self {
        Self::with_clock(store, scope, holder, lease, system_clock())
    }

    pub fn with_clock(
        store: Arc<dyn LeaseStore>,
        scope: impl Into<String>,
        holder: impl Into<String>,
        lease: Duration,
        now: Clock,
    ) -> Self {
        Self {
            store,
            scope: scope.into(),
            holder: holder.into(),
            lease,
            now,
            view: Leadership::default(),
        }
    }

    /// 副本标识（写入租约表的 holder）。
    pub fn holder(&self) -> &str {
        &self.holder
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn lease(&self) -> Duration {
        self.lease
    }

    /// 竞选/续约一个 tick。成为 leader 时返回 `leader=true` 与新任期。
    pub fn step(&mut self) -> AllianceResult<Leadership> {
        let now = (self.now)();
        let st = self.store.campaign(&self.scope, &self.holder, now, self.lease)?;
        self.view = Leadership {
            leader: st.held_by(&self.holder, now),
            epoch: st.epoch,
            expires_at: if st.holder.as_deref() == Some(self.holder.as_str()) {
                st.expires_at
            } else {
                None
            },
        };
        Ok(self.view)
    }

    /// 最近一次 `step()` 的结果（不触发新的 CAS）。
    pub fn leadership(&self) -> Leadership {
        self.view
    }

    /// 优雅让位（进程退出时调用，让 standby 立即接管而不必等租约过期）。
    pub fn resign(&mut self) -> AllianceResult<()> {
        self.store.release(&self.scope, &self.holder)?;
        self.view = Leadership {
            leader: false,
            epoch: self.view.epoch,
            expires_at: None,
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc as Tz};
    use mox_alliance_common_proto::AllianceError;

    fn at(secs: i64) -> DateTime<Utc> {
        Tz.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap() + ChronoDuration::seconds(secs)
    }

    /// 可拨动的测试时钟（多个 elector 共享同一份"当前时间"）。
    #[derive(Default)]
    struct TestClock {
        now: Mutex<i64>,
    }
    impl TestClock {
        fn shared(start: i64) -> (Arc<Self>, Clock) {
            let c = Arc::new(TestClock {
                now: Mutex::new(start),
            });
            let cell = c.clone();
            (
                c,
                Arc::new(move || {
                    let secs = *cell.now.lock().unwrap();
                    at(secs)
                }),
            )
        }
        fn advance(&self, secs: i64) {
            *self.now.lock().unwrap() += secs;
        }
    }

    const LEASE: Duration = Duration::from_secs(10);

    /// 不需要拨表的用例用的固定时钟。
    fn fixed(secs: i64) -> Clock {
        Arc::new(move || at(secs))
    }

    #[test]
    fn first_candidate_wins_and_others_stay_standby() {
        let store = Arc::new(MemoryLeaseStore::new());
        let now = fixed(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut b = LeaderElector::with_clock(store.clone(), "scheduler", "b", LEASE, now);

        assert!(a.step().unwrap().leader, "首个竞选者成为 leader");
        let bview = b.step().unwrap();
        assert!(!bview.leader, "同一租约内第二个竞选者必须落选");
        assert_eq!(bview.epoch, a.leadership().epoch, "落选者不得抬高任期");
        assert_eq!(
            store.state("scheduler").unwrap().holder.as_deref(),
            Some("a")
        );
    }

    #[test]
    fn leader_renews_without_bumping_epoch() {
        let store = Arc::new(MemoryLeaseStore::new());
        let (clock, now) = TestClock::shared(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now);

        let first = a.step().unwrap();
        clock.advance(4);
        let second = a.step().unwrap();
        assert!(second.leader);
        assert_eq!(first.epoch, second.epoch, "续约不换任期");
        assert_eq!(second.expires_at, Some(at(14)), "续约把到期时刻推后一个 lease");
    }

    #[test]
    fn standby_takes_over_after_lease_expiry() {
        let store = Arc::new(MemoryLeaseStore::new());
        let (clock, now) = TestClock::shared(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut b = LeaderElector::with_clock(store.clone(), "scheduler", "b", LEASE, now);

        let old = a.step().unwrap();
        a.step().unwrap();
        // a 宕机：停止续约。
        clock.advance(11);
        let taken = b.step().unwrap();
        assert!(taken.leader, "租约过期后 standby 接管");
        assert_eq!(taken.epoch, old.epoch + 1, "接管必须换任期");
        assert!(!old.accepts(taken.epoch), "旧 leader 的任期不再被接受（fencing）");
    }

    #[test]
    fn zombie_leader_cannot_resume_old_epoch() {
        let store = Arc::new(MemoryLeaseStore::new());
        let (clock, now) = TestClock::shared(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut b = LeaderElector::with_clock(store.clone(), "scheduler", "b", LEASE, now);

        let a0 = a.step().unwrap();
        clock.advance(11);
        let b0 = b.step().unwrap(); // b 接管
        clock.advance(1);
        let a1 = a.step().unwrap(); // 假死的 a 迟到续约
        assert!(!a1.leader, "已被接管的旧 leader 不能靠续约夺回领导权");
        assert_eq!(a1.epoch, b0.epoch, "a 只能观察到 b 的任期");
        assert!(b0.accepts(b0.epoch));
        assert!(!b0.accepts(a0.epoch), "b 不会接受 a 的旧任期写请求");
    }

    #[test]
    fn epoch_is_monotonic_across_repeated_takeovers() {
        let store = Arc::new(MemoryLeaseStore::new());
        let (clock, now) = TestClock::shared(0);
        let mut prev = 0u64;
        for round in 0..5 {
            let holder = if round % 2 == 0 { "a" } else { "b" };
            let mut e = LeaderElector::with_clock(
                store.clone(),
                "scheduler",
                holder,
                LEASE,
                now.clone(),
            );
            let view = e.step().unwrap();
            assert!(view.leader, "第 {round} 轮应接管成功");
            assert!(view.epoch > prev, "任期只增不减：{:?} -> {:?}", prev, view.epoch);
            prev = view.epoch;
            clock.advance(11);
        }
        assert_eq!(prev, 5, "每轮接管 +1");
    }

    #[test]
    fn resign_lets_standby_take_over_without_waiting() {
        let store = Arc::new(MemoryLeaseStore::new());
        let now = fixed(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut b = LeaderElector::with_clock(store.clone(), "scheduler", "b", LEASE, now);

        let a0 = a.step().unwrap();
        a.resign().unwrap();
        assert!(!a.leadership().leader);
        let b0 = b.step().unwrap();
        assert!(b0.leader, "让位后 standby 立刻接管，无需等租约过期");
        assert_eq!(b0.epoch, a0.epoch + 1, "让位不使任期回退");
    }

    #[test]
    fn release_by_non_holder_is_noop() {
        let store = Arc::new(MemoryLeaseStore::new());
        let now = fixed(0);
        let mut a = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut b = LeaderElector::with_clock(store.clone(), "scheduler", "b", LEASE, now);
        a.step().unwrap();

        b.resign().unwrap(); // b 并非持有者
        assert_eq!(
            store.state("scheduler").unwrap().holder.as_deref(),
            Some("a"),
            "非持有者的释放不得踢掉现任 leader"
        );
    }

    #[test]
    fn scopes_are_independent() {
        let store = Arc::new(MemoryLeaseStore::new());
        let now = fixed(0);
        let mut ha = LeaderElector::with_clock(store.clone(), "scheduler", "a", LEASE, now.clone());
        let mut hb = LeaderElector::with_clock(store.clone(), "registry", "a", LEASE, now);
        assert!(ha.step().unwrap().leader);
        assert!(hb.step().unwrap().leader, "不同 scope 各自选主，互不排斥");
        assert_eq!(store.state("registry").unwrap().epoch, 1);
    }

    #[test]
    fn decide_expires_at_boundary_is_not_held() {
        // expires_at == now 视为已过期（半开区间），接管方可在同一时刻抢占。
        let cur = LeaseState {
            holder: Some("a".into()),
            epoch: 3,
            expires_at: Some(at(10)),
        };
        assert!(!cur.is_held_at(at(10)));
        let next = decide(&cur, "b", at(10), LEASE).expect("到期即可抢占");
        assert_eq!(next.holder.as_deref(), Some("b"));
        assert_eq!(next.epoch, 4);
        assert_eq!(next.expires_at, Some(at(20)));
    }

    #[test]
    fn store_errors_surface_as_internal() {
        struct Broken;
        impl LeaseStore for Broken {
            fn state(&self, _scope: &str) -> AllianceResult<LeaseState> {
                Err(AllianceError::internal("boom"))
            }
            fn campaign(
                &self,
                _scope: &str,
                _holder: &str,
                _now: DateTime<Utc>,
                _lease: Duration,
            ) -> AllianceResult<LeaseState> {
                Err(AllianceError::internal("boom"))
            }
            fn release(&self, _scope: &str, _holder: &str) -> AllianceResult<()> {
                Err(AllianceError::internal("boom"))
            }
        }
        let mut e = LeaderElector::new(Arc::new(Broken), "s", "a", LEASE);
        assert!(e.step().is_err(), "仲裁点故障必须让调用方知道，而不是静默当作 leader");
        assert!(!e.leadership().leader, "出错时保持上一次视图（初始为 false）");
    }
}
