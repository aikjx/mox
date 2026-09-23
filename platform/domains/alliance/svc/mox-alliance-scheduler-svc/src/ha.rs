// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 调度器多活（HA）运行面：租约选主 + 只在 leader 上跑的周期对账。
//!
//! 请求路径（`POST /tasks`、`GET /tasks/:id`…）本身无状态，多副本直接 LB 即可；
//! 唯一需要单点的是 [`reconcile_active_tasks`]：它扫全表并逐条向执行器求证，
//! 多副本并发跑会对执行器产生 N 倍轮询、并互相覆盖任务终态。
//!
//! ## 开启方式（默认关闭，行为与单副本一致）
//! ```text
//! MOX_ALLIANCE_STORAGE_MODE=sqlite          # 必须是共享权威状态（file/memory 不支持多活）
//! MOX_ALLIANCE_HA_MODE=on                   # 开启租约选主与 leader 对账
//! MOX_ALLIANCE_HA_LEASE_MS=10000            # 租约时长；接管时延上界
//! MOX_ALLIANCE_HA_TICK_MS=3000              # 竞选/续约/对账节奏（默认 lease/3）
//! MOX_ALLIANCE_HA_STALL_MS=300000           # 孤儿判定的静默窗口
//! MOX_ALLIANCE_HA_DB=data/alliance_tasks.db # 租约表所在库（与任务表同库同权威源）
//! MOX_ALLIANCE_HA_HOLDER=scheduler-1        # 副本标识（缺省 scheduler-<pid>）
//! ```
//!
//! ## 为什么租约表必须与任务表同库
//! 领导权的权威源若与任务状态的权威源分离，就会出现"副本自认 leader，
//! 但它写的任务表和别人读到的任务表不是同一份"。SQLite WAL 下一个库文件即一个
//! 一致性域，`BEGIN IMMEDIATE` 事务保证抢主的读-判-写跨进程原子。
//!
//! ## 诚实边界
//! - 只对**同一 SQLite 库可见**的副本互斥（同机或共享盘）。跨机需换 PG 后端
//!   （实现同一对 trait 即可），本模块不假装支持跨机仲裁。
//! - `file`/`memory` 仓库下开启 HA 直接拒绝启动（`SchedulerServer::ha_storage_ok`）：
//!   前者是全量快照单写者，并发副本会互相覆盖；后者压根不共享状态，选主只是自娱自乐。
//! - 接管时延上界 = `lease`。分区/长停顿的旧 leader 由 fencing 任期兜底
//!   （见 [`leadership`](mox_alliance_scheduler_core::leadership) 模块文档）。
//! - 开启 HA 时任务库以 `new_shared` 打开 ⇒ **不做启动清扫**（那会把对端正在跑的
//!   running 任务改写成 interrupted）；异常态收敛改由 leader 的
//!   `TaskSchedulerImpl::reconcile_active_tasks` 逐条向执行器求证完成。
//!   全副本同时宕机时，遗留 running 任务要等一轮对账才可见地转 Failed。

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use mox_alliance_scheduler_core::{LeaderElector, SqliteLeaseStore, TaskSchedulerImpl};
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// 本服务的选主域：与执行器/网关注册等其他域互不干扰。
pub const SCOPE: &str = "alliance-scheduler";

/// HA 运行参数（全部来自环境变量，缺值即关闭）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaConfig {
    /// 副本标识（写入租约表 holder 列）。
    pub holder: String,
    /// 租约时长：leader 停止续约后，standby 最迟在此时间后接管。
    pub lease: Duration,
    /// 竞选/续约/对账节奏（应显著小于 `lease`，否则一次抖动就丢主）。
    pub tick: Duration,
    /// 静默窗口：执行器明确"不认识"且超过此时长无进展的任务才判为孤儿。
    pub stall: Duration,
    /// 租约表所在 SQLite 库（应与任务库同一路径）。
    pub db: PathBuf,
}

impl HaConfig {
    /// 从进程环境读取；`MOX_ALLIANCE_HA_MODE` 未开启时返回 `None`。
    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }

    /// 纯函数版（供单测注入变量，不碰真实进程环境）。
    fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let flag = get("MOX_ALLIANCE_HA_MODE")?;
        if !matches!(flag.trim().to_ascii_lowercase().as_str(), "1" | "on" | "true" | "yes") {
            return None;
        }
        let lease_ms = lookup_ms(&get, "MOX_ALLIANCE_HA_LEASE_MS", 10_000);
        let tick_ms = {
            let dflt = (lease_ms / 3).max(1_000);
            lookup_ms(&get, "MOX_ALLIANCE_HA_TICK_MS", dflt)
        };
        let stall_ms = lookup_ms(&get, "MOX_ALLIANCE_HA_STALL_MS", 300_000);
        Some(Self {
            holder: get("MOX_ALLIANCE_HA_HOLDER")
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| format!("scheduler-{}", std::process::id())),
            lease: Duration::from_millis(lease_ms.max(1_000)),
            tick: Duration::from_millis(tick_ms.max(200)),
            stall: Duration::from_millis(stall_ms),
            db: get("MOX_ALLIANCE_HA_DB")
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("data").join("alliance_tasks.db")),
        })
    }

    /// 校验参数自洽：tick 必须远小于 lease，否则续约赶不上过期→反复丢主。
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.tick * 2 < self.lease,
            "MOX_ALLIANCE_HA_TICK_MS({})×2 必须小于 MOX_ALLIANCE_HA_LEASE_MS({})：\
             续约节奏赶不上租约到期会反复丢主",
            self.tick.as_millis(),
            self.lease.as_millis()
        );
        anyhow::ensure!(
            !self.stall.is_zero(),
            "MOX_ALLIANCE_HA_STALL_MS=0 会让任何一次瞬时 404 立刻判死任务"
        );
        Ok(())
    }
}

fn lookup_ms(get: &impl Fn(&str) -> Option<String>, key: &str, default: u64) -> u64 {
    get(key).and_then(|v| v.trim().parse::<u64>().ok()).unwrap_or(default)
}

/// 共享的选主状态机（HTTP 观测端点与后台循环共用一把锁）。
pub type SharedElector = Arc<Mutex<LeaderElector>>;

fn lock(e: &SharedElector) -> std::sync::MutexGuard<'_, LeaderElector> {
    e.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 仅测试用：把选主状态机挂到进程内存的仲裁点上，验证接管逻辑而不依赖 SQLite 文件。
#[cfg(test)]
pub(crate) fn elector_in_memory(cfg: &HaConfig) -> SharedElector {
    Arc::new(Mutex::new(LeaderElector::new(
        Arc::new(mox_alliance_scheduler_core::MemoryLeaseStore::new()),
        SCOPE,
        cfg.holder.clone(),
        cfg.lease,
    )))
}

/// 装配 HA：建租约存储 → 建选主状态机 → 起后台循环。
///
/// 返回 `(选主视图, 后台任务)`；调用方在进程收尾时 `abort` 任务并让位。
pub fn start(
    cfg: &HaConfig,
    scheduler: Arc<TaskSchedulerImpl>,
) -> anyhow::Result<(SharedElector, JoinHandle<()>)> {
    cfg.validate()?;
    let store = Arc::new(SqliteLeaseStore::new(&cfg.db)?);
    let elector: SharedElector =
        Arc::new(Mutex::new(LeaderElector::new(store, SCOPE, cfg.holder.clone(), cfg.lease)));
    info!(
        holder = %cfg.holder,
        lease_ms = cfg.lease.as_millis(),
        tick_ms = cfg.tick.as_millis(),
        stall_ms = cfg.stall.as_millis(),
        db = %cfg.db.display(),
        "调度器 HA 启用：租约选主 + leader 专属对账"
    );
    let task = spawn_loop(elector.clone(), scheduler, cfg.clone());
    Ok((elector, task))
}

/// 起后台循环（时钟/仲裁点与真实装配同一份代码，测试可直接驱动）。
pub fn spawn_loop(
    elector: SharedElector,
    scheduler: Arc<TaskSchedulerImpl>,
    cfg: HaConfig,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let stall = cfg.stall;
        let lease_ms = cfg.lease.as_millis();
        let mut ticker = tokio::time::interval(cfg.tick);
        let mut was_leader = false;
        loop {
            ticker.tick().await;
            // step() 含 SQLite 写事务（同步 IO）→ 移出 async 线程。
            let step_elector = elector.clone();
            let stepped = tokio::task::spawn_blocking(move || lock(&step_elector).step()).await;
            let view = match stepped {
                Ok(Ok(v)) => v,
                Ok(Err(e)) => {
                    warn!("HA 选主轮次失败（仲裁点不可用？）: {e}");
                    continue;
                },
                Err(e) => {
                    warn!("HA 选主任务阻塞失败: {e}");
                    return;
                },
            };
            if view.leader != was_leader {
                info!(
                    leader = view.leader,
                    epoch = view.epoch,
                    lease_ms = lease_ms,
                    "调度器领导权变更"
                );
                was_leader = view.leader;
            }
            if !view.leader {
                continue;
            }

            match scheduler.reconcile_active_tasks(stall, chrono::Utc::now()).await {
                Ok(report) => {
                    // fencing 复查：本轮跑完后领导权是否还在同一任期。
                    // 租约选主只保证互斥、不保证旧 leader 那一轮不跑完，故这里只观测不追回
                    // （对账写本身幂等，双跑不产生错误状态）。
                    let fenced_out = !lock(&elector).leadership().accepts(view.epoch);
                    if report.scanned > 0 || !report.reclaimed.is_empty() || fenced_out {
                        info!(
                            scanned = report.scanned,
                            synced = report.synced,
                            reclaimed = report.reclaimed.len(),
                            waited = report.waited,
                            executor_unreachable = report.executor_unreachable,
                            fenced_out,
                            epoch = view.epoch,
                            "HA 对账一轮完成"
                        );
                    }
                },
                Err(e) => warn!("HA 对账失败（下一轮重试）: {e}"),
            }
        }
    })
}

/// 主动让位：进程收尾时调用，standby 因此不必等满一个租约周期才能接管。
///
/// 让位失败不致命——租约到期本身就是兜底，故只记日志不向调用方冒泡错误。
pub fn resign(elector: &SharedElector) {
    let mut guard = lock(elector);
    if !guard.leadership().leader {
        return; // 本就未持有领导权：不让位，也不留下误导性日志
    }
    match guard.resign() {
        Ok(()) => info!("调度器 HA 已让位：standby 可立即接管"),
        Err(e) => warn!("调度器 HA 让位失败（由租约到期兜底）: {e}"),
    }
}

/// `GET /leadership` 的响应体：本副本视角 + 仲裁点权威快照。
pub fn status_json(elector: &SharedElector) -> serde_json::Value {
    let guard = lock(elector);
    let view = guard.leadership();
    serde_json::json!({
        "scope": guard.scope(),
        "holder": guard.holder(),
        "is_leader": view.leader,
        "epoch": view.epoch,
        "lease_ms": guard.lease().as_millis() as u64,
        "expires_at": view.expires_at.map(|t| t.to_rfc3339()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mox_alliance_common_proto::{
        AllianceMode, AllianceResult, CollaborationPlan, FusionStrategy, Task, TaskPriority,
        TaskStatus,
    };
    use mox_alliance_executor_proto::ExecutionStatus;
    use mox_alliance_scheduler_core::{
        ExecutorBridge, InMemoryTaskRepository, LeaseStore, MemoryLeaseStore, TaskRepository,
    };
    use mox_alliance_scheduler_proto::types::SchedulerConfig;
    use uuid::Uuid;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let owned: Vec<(String, String)> =
            pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect();
        move |k: &str| owned.iter().find(|(key, _)| key == k).map(|(_, v)| v.clone())
    }

    #[test]
    fn off_by_default_and_by_explicit_false() {
        assert_eq!(HaConfig::from_lookup(vars(&[])), None, "未设置即关闭");
        assert_eq!(HaConfig::from_lookup(vars(&[("MOX_ALLIANCE_HA_MODE", "off")])), None);
        assert_eq!(HaConfig::from_lookup(vars(&[("MOX_ALLIANCE_HA_MODE", "0")])), None);
    }

    #[test]
    fn on_yields_derived_defaults() {
        let cfg = HaConfig::from_lookup(vars(&[("MOX_ALLIANCE_HA_MODE", "on")])).unwrap();
        assert_eq!(cfg.lease, Duration::from_millis(10_000));
        assert_eq!(cfg.tick, Duration::from_millis(3_333), "tick 缺省取 lease/3");
        assert_eq!(cfg.stall, Duration::from_millis(300_000));
        assert_eq!(cfg.db, PathBuf::from("data").join("alliance_tasks.db"));
        assert!(cfg.holder.starts_with("scheduler-"));
        cfg.validate().unwrap();
    }

    #[test]
    fn explicit_values_win_and_ticks_must_outrun_lease() {
        let cfg = HaConfig::from_lookup(vars(&[
            ("MOX_ALLIANCE_HA_MODE", "true"),
            ("MOX_ALLIANCE_HA_LEASE_MS", "4000"),
            ("MOX_ALLIANCE_HA_TICK_MS", "9000"),
            ("MOX_ALLIANCE_HA_STALL_MS", "0"),
            ("MOX_ALLIANCE_HA_HOLDER", "sched-a"),
            ("MOX_ALLIANCE_HA_DB", "/tmp/x.db"),
        ]))
        .unwrap();
        assert_eq!(cfg.holder, "sched-a");
        assert_eq!(cfg.tick, Duration::from_millis(9_000));
        assert_eq!(cfg.db, PathBuf::from("/tmp/x.db"));
        assert!(cfg.validate().is_err(), "tick×2 >= lease 与 stall=0 都必须拒绝启动");
    }

    #[test]
    fn unparseable_numbers_fall_back_to_defaults() {
        let cfg = HaConfig::from_lookup(vars(&[
            ("MOX_ALLIANCE_HA_MODE", "yes"),
            ("MOX_ALLIANCE_HA_LEASE_MS", "abc"),
            ("MOX_ALLIANCE_HA_HOLDER", "   "),
        ]))
        .unwrap();
        assert_eq!(cfg.lease, Duration::from_millis(10_000));
        assert!(cfg.holder.starts_with("scheduler-"), "空白 holder 视为未设置");
    }

    #[tokio::test]
    async fn standby_takes_over_at_a_higher_epoch() {
        // 两个副本共用一个仲裁点：同域内只允许一个 leader；让位后 standby 立即接管且任期 +1。
        let cfg = HaConfig::from_lookup(vars(&[("MOX_ALLIANCE_HA_MODE", "on")])).unwrap();
        let store = Arc::new(MemoryLeaseStore::new());
        let ea = Arc::new(Mutex::new(LeaderElector::new(store.clone(), SCOPE, "a", cfg.lease)));
        let eb = Arc::new(Mutex::new(LeaderElector::new(store.clone(), SCOPE, "b", cfg.lease)));

        let la = lock(&ea).step().unwrap();
        let lb = lock(&eb).step().unwrap();
        assert!(la.leader && !lb.leader, "同域内只允许一个 leader");
        assert_eq!(lb.epoch, la.epoch, "standby 的视角是仲裁点当前任期");
        assert!(!lb.accepts(la.epoch), "standby 不得接受任何任期——否则双主各自写状态");
        assert_eq!(store.state(SCOPE).unwrap().epoch, la.epoch, "竞选失败者不得抬高仲裁点上的任期");

        // leader 让位 → standby 下一步即接管，无需等租约到期。
        resign(&ea);
        let after = lock(&eb).step().unwrap();
        assert!(after.leader);
        assert!(after.epoch > lb.epoch, "接管必须带来更高的任期");
        assert!(!lock(&ea).leadership().accepts(after.epoch), "让位后的旧副本不得再接受新任期");
    }

    #[tokio::test]
    async fn reconcile_polls_the_executor_only_on_the_leader() {
        let cfg = HaConfig {
            holder: "a".into(),
            lease: Duration::from_millis(4_000),
            tick: Duration::from_millis(20),
            stall: Duration::from_millis(60_000),
            db: PathBuf::from(":memory:"),
        };

        // 两个副本共用一个仲裁点，各自一份任务仓库 + 执行器桥（同一份种子数据）。
        let store = Arc::new(MemoryLeaseStore::new());
        let ea = Arc::new(Mutex::new(LeaderElector::new(store.clone(), SCOPE, "a", cfg.lease)));
        let eb = Arc::new(Mutex::new(LeaderElector::new(store.clone(), SCOPE, "b", cfg.lease)));
        assert!(lock(&ea).step().unwrap().leader, "先让 a 抢到主");

        let (sched_a, bridge_a, repo_a, task_a) = seeded_scheduler();
        let (sched_b, bridge_b, repo_b, task_b) = seeded_scheduler();
        let loop_a = spawn_loop(ea.clone(), sched_a, cfg.clone());
        let loop_b = spawn_loop(eb.clone(), sched_b, cfg);
        tokio::time::sleep(Duration::from_millis(300)).await;
        loop_a.abort();
        loop_b.abort();

        let leader_polls = bridge_a.polls();
        assert!(leader_polls > 0, "leader 应周期性向执行器求证（轮询 0 次说明对账没跑）");
        assert_eq!(
            bridge_b.polls(),
            0,
            "standby 不得轮询执行器：否则 N 副本产生 N 倍轮询并互相覆盖终态"
        );

        // leader 把执行器回报的进度写回了任务；standby 的种子任务原封不动。
        // 注意：直接读仓库而非 scheduler.get_task()——后者是请求路径，本身就会去
        // 执行器同步状态，会把"谁在轮询"这一观测信号污染掉。
        assert!(progress_of(&repo_a, task_a) > 0.0, "leader 应回填进度");
        assert_eq!(progress_of(&repo_b, task_b), 0.0, "standby 的仓库不该被周期职责改动");
    }

    /// 只记录轮询次数的执行器桥：本测试关心"谁在跑周期职责"，
    /// 对账语义本身由 core 的 reconcile 测试覆盖。
    #[derive(Default)]
    struct PollBridge {
        polls: std::sync::atomic::AtomicUsize,
    }

    impl PollBridge {
        fn polls(&self) -> usize {
            self.polls.load(std::sync::atomic::Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl ExecutorBridge for PollBridge {
        async fn submit_plan(&self, _task: &Task, _plan: CollaborationPlan) -> AllianceResult<()> {
            Ok(())
        }
        async fn cancel_task(
            &self,
            _task_id: Uuid,
            _tenant_id: Uuid,
            _reason: Option<String>,
        ) -> AllianceResult<()> {
            Ok(())
        }
        async fn get_status(
            &self,
            task_id: Uuid,
            _tenant_id: Uuid,
        ) -> AllianceResult<ExecutionStatus> {
            self.polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(ExecutionStatus {
                task_id,
                total_nodes: 4,
                completed_nodes: 1,
                running_nodes: 3,
                failed_nodes: 0,
                pending_nodes: 0,
                skipped_nodes: 0,
                cancelled_nodes: 0,
                progress: 0.25,
                started_at: None,
                estimated_remaining_ms: None,
            })
        }
        async fn pause_task(
            &self,
            _task_id: Uuid,
            _tenant_id: Uuid,
        ) -> mox_alliance_common_proto::AllianceResult<()> {
            Ok(())
        }
        async fn resume_task(
            &self,
            _task_id: Uuid,
            _tenant_id: Uuid,
        ) -> mox_alliance_common_proto::AllianceResult<()> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    /// 调度器 + 仓库里一条正在跑的活跃任务（standby 与 leader 用同一份种子）。
    fn seeded_scheduler(
    ) -> (Arc<TaskSchedulerImpl>, Arc<PollBridge>, Arc<InMemoryTaskRepository>, Uuid) {
        let bridge = Arc::new(PollBridge::default());
        let repo = Arc::new(InMemoryTaskRepository::new());
        let mut task = Task::new(Uuid::new_v4(), Uuid::new_v4(), "t".to_string(), "d".to_string());
        task.status = TaskStatus::Running;
        task.started_at = Some(chrono::Utc::now());
        repo.save(&task).expect("种子任务应可写入仓库");
        let id = task.task_id;
        let scheduler = Arc::new(
            TaskSchedulerImpl::new_with_bridge(
                task_scheduler_config(),
                Arc::new(mox_alliance_scheduler_core::ModularWeightMatcher::new()),
                bridge.clone(),
            )
            .with_task_repository(repo.clone()),
        );
        (scheduler, bridge, repo, id)
    }

    /// 直接读仓库拿进度（绕开 scheduler.get_task 请求路径的执行器同步）。
    fn progress_of(repo: &Arc<InMemoryTaskRepository>, task_id: Uuid) -> f32 {
        repo.get(task_id).expect("读取任务不应失败").expect("任务应存在").progress
    }

    fn task_scheduler() -> TaskSchedulerImpl {
        TaskSchedulerImpl::new_with_bridge(
            task_scheduler_config(),
            Arc::new(mox_alliance_scheduler_core::ModularWeightMatcher::new()),
            Arc::new(mox_alliance_scheduler_core::NoopExecutorBridge),
        )
    }

    fn task_scheduler_config() -> SchedulerConfig {
        SchedulerConfig {
            max_concurrent_tasks: 10,
            queue_capacity: 100,
            default_priority: TaskPriority::Normal,
            default_mode: AllianceMode::Parallel,
            default_fusion_strategy: FusionStrategy::Weighted,
            plan_generation_timeout_ms: 30_000,
        }
    }

    #[tokio::test]
    async fn spawned_loop_keeps_service_alive_as_leader() {
        let cfg = HaConfig {
            holder: "a".into(),
            lease: Duration::from_millis(2_000),
            tick: Duration::from_millis(250),
            stall: Duration::from_millis(50),
            db: PathBuf::from(":memory:"),
        };
        let elector = elector_in_memory(&cfg);
        let task = spawn_loop(elector.clone(), Arc::new(task_scheduler()), cfg);
        tokio::time::sleep(Duration::from_millis(700)).await;
        assert!(lock(&elector).leadership().leader, "首个副本应稳定持有领导权");
        assert!(!task.is_finished(), "后台循环不得因一轮对账就退出");
        task.abort();
    }

    #[test]
    fn status_json_exposes_the_arbitration_view() {
        let cfg = HaConfig {
            holder: "a".into(),
            lease: Duration::from_millis(2_000),
            tick: Duration::from_millis(500),
            stall: Duration::from_millis(60_000),
            db: PathBuf::from(":memory:"),
        };
        let elector = elector_in_memory(&cfg);

        let before = status_json(&elector);
        assert_eq!(before["scope"], SCOPE);
        assert_eq!(before["holder"], "a");
        assert_eq!(before["is_leader"], false, "未竞选前不得自称 leader");
        assert_eq!(before["epoch"], 0);
        assert!(before["expires_at"].is_null());

        let view = lock(&elector).step().unwrap();
        let after = status_json(&elector);
        assert_eq!(after["is_leader"], true);
        assert_eq!(after["epoch"], view.epoch);
        assert_eq!(after["lease_ms"], 2_000);
        assert!(
            after["expires_at"].is_string(),
            "leader 必须暴露租约到期时刻：运维据此判断接管上界"
        );
    }

    /// 真实 SQLite 仲裁点上的装配：`start()` 建租约表 + 起后台循环，
    /// 两个副本（两条连接，等价两个进程）收敛为一个 leader，leader 消失后对端接管。
    #[tokio::test]
    async fn start_on_sqlite_elects_one_leader_and_takes_over() {
        let dir = std::env::temp_dir().join(format!("ha_sqlite_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let base = HaConfig {
            holder: "a".into(),
            lease: Duration::from_millis(1_500),
            tick: Duration::from_millis(400),
            stall: Duration::from_millis(60_000),
            db: dir.join("tasks.db"),
        };
        let (ea, ta) = start(&base, Arc::new(task_scheduler())).unwrap();
        let cfg_b = HaConfig { holder: "b".into(), ..base.clone() };
        let (eb, tb) = start(&cfg_b, Arc::new(task_scheduler())).unwrap();

        wait_until(|| lock(&ea).leadership().leader || lock(&eb).leadership().leader).await;
        let (va, vb) = (lock(&ea).leadership(), lock(&eb).leadership());
        assert!(!(va.leader && vb.leader), "同一租约域内出现双主即故障：va={va:?} vb={vb:?}");
        assert!(va.leader || vb.leader, "周期职责必须有人承担：va={va:?} vb={vb:?}");

        // 模拟 leader 进程消失：abort 后台循环但不让位，对端只能靠租约到期接管。
        let (dead, alive, dead_view) =
            if va.leader { (ta, eb.clone(), va) } else { (tb, ea.clone(), vb) };
        dead.abort();
        wait_until(|| lock(&alive).leadership().leader).await;
        let taken = lock(&alive).leadership();
        assert!(
            taken.epoch > dead_view.epoch,
            "接管必须抬高任期，让旧 leader 的迟到写被 fencing 拒掉：{taken:?} vs {dead_view:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 轮询等待条件成立（上限 8s，100ms 一次）。
    /// 用轮询而非固定 sleep：把时序敏感测试与机器负载解耦。
    async fn wait_until(pred: impl Fn() -> bool) {
        for _ in 0..80 {
            if pred() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        panic!("等待条件超时（8s）");
    }
}
