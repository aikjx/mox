// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 云盘 M4：冷热分层引擎 (HOT / WARM / COLD / GLACIER 四级 Lifecycle, v2.1)
//!
//! # 存储类迁移规则
//!
//! | 类      | 时间窗口        | 典型场景             | 读取行为               |
//! |---------|----------------|----------------------|------------------------|
//! | HOT     | 0 ~ 30 天      | 业务活跃数据          | 直读                   |
//! | WARM    | 30 ~ 90 天     | 非频繁访问            | 读 → 自动回温到 HOT    |
//! | COLD    | 90 ~ 365 天    | 归档/合规留存         | 读 → restore → HOT     |
//! | GLACIER | 365 天以上     | 长期归档/监管封存     | 读 → restore(数小时) → HOT |
//!
//! 每日 UTC 02:00 由 `transition_scan()` 触发全量扫描并生成迁移计划；
//! 任何对 WARM/COLD/GLACIER 对象的读都通过 `touch_and_restore_to_hot()` 回到 HOT。
//!
//! # P1 优化（v2.2）
//!
//! - **复制等待门控（Replication Wait Gate）**：当对象复制状态为 Pending/Failed 时，
//!   阻塞 Delete/Transition 类生命周期动作，降级为不执行，直到复制完成。
//! - **DeleteAllVersions 短路路径**：当对象命中全版本删除规则且满足
//!   （无 Object Lock + 无版本锁定 + 无 Pending 复制）时，直接生成全版本删除计划
//!   并跳过逐版本评估。
//!
//! 算法参考：RustFS lifecycle evaluator (Apache 2.0) — `crates/lifecycle/src/evaluator.rs`
//! 本实现为自研重写，未直接复制代码。

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use crate::scanner::{ScanBudget, ScanBudgetTracker, ScanStats};

/// 存储类枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum StorageClass {
    #[default]
    Hot,
    Warm,
    Cold,
    /// v2.1: 冷归档（AWS Glacier/阿里云归档-冷归档），取回需数小时
    Glacier,
}

impl StorageClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            StorageClass::Hot => "HOT",
            StorageClass::Warm => "WARM",
            StorageClass::Cold => "COLD",
            StorageClass::Glacier => "GLACIER",
        }
    }
}


// ---------------------------------------------------------------------------
// P1: 复制等待门控 — 对象复制状态（生命周期视图，简化枚举）
// ---------------------------------------------------------------------------

/// 对象复制状态（生命周期门控用简化枚举）
///
/// 与 `crate::replication::ReplicationStatus` 对应，但额外提供 `None`
/// 表示未配置复制规则（不门控）。
///
/// 注意：本类型与 `crate::replication::ObjectReplicationStatus`（完整记录结构体）
/// 同名但位于不同模块；lib.rs 中以别名 `LifecycleReplicationStatus` 导出以避免冲突。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub enum ObjectReplicationStatus {
    /// 未配置复制规则（不门控）
    #[default]
    None,
    /// 复制待处理（门控 Delete/Transition）
    Pending,
    /// 复制完成（不门控）
    Completed,
    /// 复制失败（门控 Delete/Transition）
    Failed,
}


/// 未启用版本化时的默认版本 ID（S3 约定）
fn default_version_id() -> String {
    "null".to_string()
}

/// 对象元数据（生命周期视图）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleObjectMeta {
    pub key: String,
    pub bucket: String,
    pub size_bytes: u64,
    pub class: StorageClass,
    /// 创建时间 ms (UNIX epoch)
    pub created_at_ms: u64,
    /// 上次访问时间 ms
    pub last_accessed_at_ms: u64,
    /// 上次类变更时间 ms
    pub last_transition_ms: u64,
    /// v2.2: 版本 ID（未启用版本化时为 "null"）
    #[serde(default = "default_version_id")]
    pub version_id: String,
    /// v2.2: 对象复制状态（用于生命周期门控）
    #[serde(default)]
    pub replication_status: ObjectReplicationStatus,
    /// v2.2: 对象是否被 Object Lock 锁定（保留期内不可删除）
    #[serde(default)]
    pub object_locked: bool,
}

/// 迁移动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionAction {
    HotToWarm,
    WarmToCold,
    /// v2.1: COLD → GLACIER（冷归档）
    ColdToGlacier,
    /// 读回温
    WarmRestoreToHot,
    /// 归档 restore（慢速）+ 回温
    ColdRestoreToHot,
    /// v2.1: Glacier restore（最慢，通常 3~12h）+ 回温
    GlacierRestoreToHot,
    /// v2.2: 删除指定版本
    DeleteVersion,
    /// v2.2: 删除对象的所有版本（短路路径）
    DeleteAllVersions,
}

/// 迁移计划项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionPlan {
    pub bucket: String,
    pub key: String,
    pub from: StorageClass,
    pub to: StorageClass,
    pub action: TransitionAction,
    pub scheduled_at_ms: u64,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// P1: DeleteAllVersions 短路路径
// ---------------------------------------------------------------------------

/// 全版本删除计划（DeleteAllVersions 短路路径产物）
///
/// 当对象命中全版本删除规则且满足安全条件（无 Object Lock、无版本锁定、
/// 无 Pending 复制）时生成，调用方应据此删除该对象的所有版本并跳过
/// 剩余逐版本生命周期评估。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAllVersionsPlan {
    pub bucket: String,
    pub key: String,
    /// 要删除的所有版本 ID
    pub version_ids: Vec<String>,
    pub reason: String,
    pub scheduled_at_ms: u64,
}

// ---------------------------------------------------------------------------
// P1: 复制等待门控判断函数
// ---------------------------------------------------------------------------

/// 判断给定的生命周期动作是否被复制状态阻塞。
///
/// Pending/Failed 状态阻塞 Delete/Transition 类动作（HotToWarm、WarmToCold、
/// ColdToGlacier、DeleteVersion、DeleteAllVersions）；Restore 类动作（读触发回温）
/// 不被门控，因为它们由用户读操作触发而非生命周期扫描。
///
/// 算法参考：RustFS `replication_status_blocks_lifecycle` + `lifecycle_action_waits_for_replication`
/// (Apache 2.0, `crates/lifecycle/src/evaluator.rs`)，本实现为自研重写。
pub fn replication_status_blocks_lifecycle(
    status: ObjectReplicationStatus,
    action: &TransitionAction,
) -> bool {
    match status {
        ObjectReplicationStatus::Pending | ObjectReplicationStatus::Failed => {
            matches!(
                action,
                TransitionAction::HotToWarm
                    | TransitionAction::WarmToCold
                    | TransitionAction::ColdToGlacier
                    | TransitionAction::DeleteVersion
                    | TransitionAction::DeleteAllVersions
            )
        },
        _ => false,
    }
}

/// 生命周期全局统计（可 JSON 序列化，供 /cloud/lifecycle/stats 返回）
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudLifecycleStats {
    pub objects_hot: u64,
    pub objects_warm: u64,
    pub objects_cold: u64,
    /// v2.1: GLACIER 层对象数
    pub objects_glacier: u64,
    pub bytes_hot: u64,
    pub bytes_warm: u64,
    pub bytes_cold: u64,
    /// v2.1: GLACIER 层字节数
    pub bytes_glacier: u64,
    pub transitions_last_24h: u64,
    pub restores_last_24h: u64,
    pub scanned_at_ms: u64,
    /// v2.2: 被复制等待门控阻塞的生命周期动作计数
    #[serde(default)]
    pub replication_blocked_count: u64,
    /// v2.2: DeleteAllVersions 短路触发计数
    #[serde(default)]
    pub delete_all_short_circuit_count: u64,
}

/// 时间阈值（可配置，默认与 spec 对齐）
#[derive(Debug, Clone, Copy)]
pub struct LifecycleThresholds {
    /// 创建/最后访问后多少 ms 进入 WARM，默认 30 天
    pub hot_to_warm_ms: u64,
    /// 创建/最后访问后多少 ms 进入 COLD，默认 90 天
    pub warm_to_cold_ms: u64,
    /// v2.1: COLD 且 anchor 超过该 ms 后进入 GLACIER，默认 365 天
    pub cold_to_glacier_ms: u64,
}

impl Default for LifecycleThresholds {
    fn default() -> Self {
        const DAY_MS: u64 = 24 * 60 * 60 * 1000;
        Self {
            hot_to_warm_ms: 30 * DAY_MS,
            warm_to_cold_ms: 90 * DAY_MS,
            cold_to_glacier_ms: 365 * DAY_MS,
        }
    }
}

/// 冷热分层引擎核心
pub struct HotWarmColdLifecycle {
    thresholds: LifecycleThresholds,
    /// 内部对象存储视图：`(bucket, key) -> meta`
    objects: parking_lot::Mutex<BTreeMap<(String, String), LifecycleObjectMeta>>,
    transition_counter: parking_lot::Mutex<u64>,
    restore_counter: parking_lot::Mutex<u64>,
    /// v2.2: 被复制门控阻塞的动作计数
    replication_blocked_counter: parking_lot::Mutex<u64>,
    /// v2.2: DeleteAllVersions 短路触发计数
    delete_all_short_circuit_counter: parking_lot::Mutex<u64>,
    /// v2.2: 标记为 DeleteAllVersions 候选的 (bucket, key) 集合
    delete_all_candidates: parking_lot::Mutex<HashSet<(String, String)>>,
    /// v2.2: 启用了 Object Lock 的桶集合
    object_lock_buckets: parking_lot::Mutex<HashSet<String>>,
    /// v2.3: 三维扫描预算（None = 不限制，保持向后兼容）
    scan_budget: Option<ScanBudget>,
    /// v2.3: 最近一次 transition_scan 的统计快照
    last_scan_stats: parking_lot::Mutex<Option<ScanStats>>,
}

impl Default for HotWarmColdLifecycle {
    fn default() -> Self {
        Self::new(LifecycleThresholds::default())
    }
}

impl HotWarmColdLifecycle {
    pub fn new(thresholds: LifecycleThresholds) -> Self {
        Self {
            thresholds,
            objects: parking_lot::Mutex::new(BTreeMap::new()),
            transition_counter: parking_lot::Mutex::new(0),
            restore_counter: parking_lot::Mutex::new(0),
            replication_blocked_counter: parking_lot::Mutex::new(0),
            delete_all_short_circuit_counter: parking_lot::Mutex::new(0),
            delete_all_candidates: parking_lot::Mutex::new(HashSet::new()),
            object_lock_buckets: parking_lot::Mutex::new(HashSet::new()),
            scan_budget: None,
            last_scan_stats: parking_lot::Mutex::new(None),
        }
    }

    /// v2.3: 构建器方法 — 设置三维扫描预算
    ///
    /// ```
    /// use mox_cloud_s3_svc::lifecycle::{HotWarmColdLifecycle, LifecycleThresholds};
    /// use mox_cloud_s3_svc::scanner::ScanBudget;
    /// let lc = HotWarmColdLifecycle::new(LifecycleThresholds::default())
    ///     .with_scan_budget(ScanBudget::default());
    /// ```
    pub fn with_scan_budget(mut self, budget: ScanBudget) -> Self {
        self.scan_budget = Some(budget);
        self
    }

    /// v2.3: 获取当前扫描预算配置（None = 未启用预算限制）
    pub fn scan_budget(&self) -> Option<&ScanBudget> {
        self.scan_budget.as_ref()
    }

    /// v2.3: 获取最近一次 transition_scan 的统计快照
    pub fn last_scan_stats(&self) -> Option<ScanStats> {
        self.last_scan_stats.lock().clone()
    }

    pub fn thresholds(&self) -> LifecycleThresholds {
        self.thresholds
    }

    pub fn update_thresholds(&mut self, t: LifecycleThresholds) {
        self.thresholds = t;
    }

    /// 注册/更新一个对象（PUT 成功时调用）
    pub fn upsert_object(&self, meta: LifecycleObjectMeta) {
        let mut objs = self.objects.lock();
        objs.insert((meta.bucket.clone(), meta.key.clone()), meta);
    }

    /// 删除对象
    pub fn remove_object(&self, bucket: &str, key: &str) {
        self.objects.lock().remove(&(bucket.to_string(), key.to_string()));
    }

    // -----------------------------------------------------------------------
    // P1: DeleteAllVersions 候选管理 + Object Lock 桶管理
    // -----------------------------------------------------------------------

    /// 标记 (bucket, key) 为 DeleteAllVersions 候选（命中桶级全版本删除规则时调用）
    pub fn mark_delete_all_candidate(&self, bucket: &str, key: &str) {
        self.delete_all_candidates.lock().insert((bucket.to_string(), key.to_string()));
    }

    /// 取消 DeleteAllVersions 候选标记
    pub fn unmark_delete_all_candidate(&self, bucket: &str, key: &str) {
        self.delete_all_candidates.lock().remove(&(bucket.to_string(), key.to_string()));
    }

    /// 设置桶是否启用 Object Lock
    pub fn set_bucket_object_lock(&self, bucket: &str, enabled: bool) {
        let mut buckets = self.object_lock_buckets.lock();
        if enabled {
            buckets.insert(bucket.to_string());
        } else {
            buckets.remove(bucket);
        }
    }

    /// 查询桶是否启用 Object Lock
    pub fn is_bucket_object_locked(&self, bucket: &str) -> bool {
        self.object_lock_buckets.lock().contains(bucket)
    }

    // -----------------------------------------------------------------------
    // P1: DeleteAllVersions 短路评估
    // -----------------------------------------------------------------------

    /// 评估对象是否满足 DeleteAllVersions 短路条件。
    ///
    /// 返回 `Some(plan)` 表示满足条件，应执行全版本删除并短路（跳过逐版本评估）；
    /// 返回 `None` 表示不满足，应继续逐版本评估。
    ///
    /// 短路条件（全部满足）：
    /// 1. 桶未启用 Object Lock
    /// 2. 无版本被锁定（`object_locked == false`）
    /// 3. 无版本处于复制 Pending 状态
    ///
    /// 算法参考：RustFS Evaluator::eval_inner 中 DeleteAllVersionsAction 分支
    /// (Apache 2.0, `crates/lifecycle/src/evaluator.rs`)，本实现为自研重写。
    ///
    /// # 当前模型说明
    /// 当前 `HotWarmColdLifecycle` 为单版本模型（`(bucket, key) -> meta`），
    /// `versions` 切片通常含一个元素；多版本短路待 `versioning` 模块深度集成后生效。
    pub fn evaluate_delete_all_versions(
        &self,
        bucket: &str,
        key: &str,
        versions: &[LifecycleObjectMeta],
        now_ms: u64,
        object_lock_enabled: bool,
    ) -> Option<DeleteAllVersionsPlan> {
        // 条件1：桶未启用 Object Lock
        if object_lock_enabled {
            return None;
        }
        // 条件2 & 3：无版本被锁定，无版本处于复制 Pending 状态
        for v in versions {
            if v.object_locked {
                return None;
            }
            if v.replication_status == ObjectReplicationStatus::Pending {
                return None;
            }
        }
        // 满足全部条件，生成全版本删除计划
        Some(DeleteAllVersionsPlan {
            bucket: bucket.to_string(),
            key: key.to_string(),
            version_ids: versions.iter().map(|v| v.version_id.clone()).collect(),
            reason: "DeleteAllVersions short-circuit".to_string(),
            scheduled_at_ms: now_ms,
        })
    }

    /// 扫描所有 DeleteAllVersions 候选，生成全版本删除计划。
    ///
    /// `apply = true` 时实际从对象存储中移除已短路的对象并取消候选标记。
    /// 返回满足短路条件的删除计划列表。
    pub fn delete_all_scan(&self, now_ms: u64, apply: bool) -> Vec<DeleteAllVersionsPlan> {
        // 快照候选集合与 Object Lock 桶集合（避免在持有 objects 锁时嵌套加锁）
        let candidates: Vec<(String, String)> =
            self.delete_all_candidates.lock().iter().cloned().collect();
        let object_lock_buckets: HashSet<String> = self.object_lock_buckets.lock().clone();

        let mut plans: Vec<DeleteAllVersionsPlan> = Vec::new();
        let mut removed_keys: Vec<(String, String)> = Vec::new();

        {
            let mut objs = self.objects.lock();
            for (bucket, key) in &candidates {
                let k = (bucket.clone(), key.clone());
                let Some(meta) = objs.get(&k) else {
                    continue;
                };
                let object_lock_enabled = object_lock_buckets.contains(bucket);
                let versions = [meta.clone()];
                if let Some(plan) = self.evaluate_delete_all_versions(
                    bucket,
                    key,
                    &versions,
                    now_ms,
                    object_lock_enabled,
                ) {
                    if apply {
                        objs.remove(&k);
                        removed_keys.push(k);
                    }
                    plans.push(plan);
                }
            }
        }

        // objects 锁已释放，安全地更新候选标记与统计
        if apply && !removed_keys.is_empty() {
            let mut candidates_lock = self.delete_all_candidates.lock();
            for k in &removed_keys {
                candidates_lock.remove(k);
            }
            *self.delete_all_short_circuit_counter.lock() += removed_keys.len() as u64;
        }

        plans
    }

    /// 读取：如果是 WARM/COLD/GLACIER → 自动回温到 HOT，返回 (新class, 是否发生restore)
    pub fn touch_and_restore_to_hot(
        &self,
        bucket: &str,
        key: &str,
        now_ms: u64,
    ) -> Option<(StorageClass, bool)> {
        let mut objs = self.objects.lock();
        let k = &(bucket.to_string(), key.to_string());
        let meta = objs.get_mut(k)?;
        let old_class = meta.class;
        match old_class {
            StorageClass::Hot => {
                meta.last_accessed_at_ms = now_ms;
                Some((StorageClass::Hot, false))
            },
            StorageClass::Warm => {
                meta.class = StorageClass::Hot;
                meta.last_accessed_at_ms = now_ms;
                meta.last_transition_ms = now_ms;
                *self.restore_counter.lock() += 1;
                *self.transition_counter.lock() += 1;
                Some((StorageClass::Hot, true))
            },
            StorageClass::Cold => {
                meta.class = StorageClass::Hot;
                meta.last_accessed_at_ms = now_ms;
                meta.last_transition_ms = now_ms;
                *self.restore_counter.lock() += 1;
                *self.transition_counter.lock() += 1;
                Some((StorageClass::Hot, true))
            },
            StorageClass::Glacier => {
                meta.class = StorageClass::Hot;
                meta.last_accessed_at_ms = now_ms;
                meta.last_transition_ms = now_ms;
                *self.restore_counter.lock() += 1;
                *self.transition_counter.lock() += 1;
                Some((StorageClass::Hot, true))
            },
        }
    }

    /// 定时扫描：基于 `now_ms` 生成迁移计划；`apply = true` 时实际应用
    ///
    /// v2.2 新增：
    /// - **复制等待门控**：对象复制状态为 Pending/Failed 时，跳过 Delete/Transition 类动作
    /// - **DeleteAllVersions 短路**：对象为 DeleteAllVersions 候选且满足安全条件时，
    ///   跳过迁移评估（对象将由 `delete_all_scan` 统一删除）
    ///
    /// v2.3 新增：
    /// - **三维扫描预算**：如果通过 [`with_scan_budget`](Self::with_scan_budget) 设置了预算，
    ///   扫描过程中会检查时间 / IO / 容量预算，超限则提前终止；
    ///   每个对象记录触发令牌桶限速，迁移操作记录迁移字节数。
    ///   扫描结束后统计快照可通过 [`last_scan_stats`](Self::last_scan_stats) 获取。
    pub fn transition_scan(&self, now_ms: u64, apply: bool) -> Vec<TransitionPlan> {
        let mut plans: Vec<TransitionPlan> = Vec::new();

        // v2.3: 如果配置了扫描预算，创建追踪器
        let budget_tracker: Option<ScanBudgetTracker> =
            self.scan_budget.as_ref().map(|b| ScanBudgetTracker::new(b.clone()));

        // 快照 DeleteAllVersions 候选集合与 Object Lock 桶集合
        // （避免在持有 objects 锁时嵌套加锁）
        let delete_all_keys: HashSet<(String, String)> = self.delete_all_candidates.lock().clone();
        let object_lock_buckets: HashSet<String> = self.object_lock_buckets.lock().clone();

        let mut objs = self.objects.lock();
        for (k, meta) in objs.iter_mut() {
            // v2.3: 预算检查 — 时间 / 容量超限则提前终止
            if let Some(ref tracker) = budget_tracker {
                if !tracker.can_continue() {
                    break;
                }
                // 记录扫描对象（含字节数），触发 IO 限速
                tracker.record_object(meta.size_bytes);
            }

            // ---- P1: DeleteAllVersions 短路检查 ----
            // 如果该对象是 DeleteAllVersions 候选且满足短路条件，
            // 跳过迁移评估（对象将由 delete_all_scan 统一删除）
            if delete_all_keys.contains(k) {
                let object_lock_enabled = object_lock_buckets.contains(&k.0);
                let versions = [meta.clone()];
                if self
                    .evaluate_delete_all_versions(
                        &k.0,
                        &k.1,
                        &versions,
                        now_ms,
                        object_lock_enabled,
                    )
                    .is_some()
                {
                    // 短路：跳过该对象的逐版本迁移评估
                    continue;
                }
            }

            // 使用 max(created, last_accessed, last_transition) 作为"活跃度锚"
            let anchor =
                meta.created_at_ms.max(meta.last_accessed_at_ms).max(meta.last_transition_ms);
            let age = now_ms.saturating_sub(anchor);
            match meta.class {
                StorageClass::Hot if age >= self.thresholds.hot_to_warm_ms => {
                    let plan = TransitionPlan {
                        bucket: k.0.clone(),
                        key: k.1.clone(),
                        from: StorageClass::Hot,
                        to: StorageClass::Warm,
                        action: TransitionAction::HotToWarm,
                        scheduled_at_ms: now_ms,
                        reason: format!(
                            "age_ms={} >= hot_to_warm_ms={}",
                            age, self.thresholds.hot_to_warm_ms
                        ),
                    };
                    // ---- P1: 复制等待门控 ----
                    if replication_status_blocks_lifecycle(meta.replication_status, &plan.action) {
                        *self.replication_blocked_counter.lock() += 1;
                        continue;
                    }
                    if apply {
                        meta.class = StorageClass::Warm;
                        meta.last_transition_ms = now_ms;
                        *self.transition_counter.lock() += 1;
                        // v2.3: 记录迁移字节数
                        if let Some(ref tracker) = budget_tracker {
                            tracker.record_migration(meta.size_bytes);
                        }
                    }
                    plans.push(plan);
                },
                StorageClass::Warm if age >= self.thresholds.warm_to_cold_ms => {
                    let plan = TransitionPlan {
                        bucket: k.0.clone(),
                        key: k.1.clone(),
                        from: StorageClass::Warm,
                        to: StorageClass::Cold,
                        action: TransitionAction::WarmToCold,
                        scheduled_at_ms: now_ms,
                        reason: format!(
                            "age_ms={} >= warm_to_cold_ms={}",
                            age, self.thresholds.warm_to_cold_ms
                        ),
                    };
                    // ---- P1: 复制等待门控 ----
                    if replication_status_blocks_lifecycle(meta.replication_status, &plan.action) {
                        *self.replication_blocked_counter.lock() += 1;
                        continue;
                    }
                    if apply {
                        meta.class = StorageClass::Cold;
                        meta.last_transition_ms = now_ms;
                        *self.transition_counter.lock() += 1;
                        if let Some(ref tracker) = budget_tracker {
                            tracker.record_migration(meta.size_bytes);
                        }
                    }
                    plans.push(plan);
                },
                StorageClass::Cold if age >= self.thresholds.cold_to_glacier_ms => {
                    let plan = TransitionPlan {
                        bucket: k.0.clone(),
                        key: k.1.clone(),
                        from: StorageClass::Cold,
                        to: StorageClass::Glacier,
                        action: TransitionAction::ColdToGlacier,
                        scheduled_at_ms: now_ms,
                        reason: format!(
                            "age_ms={} >= cold_to_glacier_ms={}",
                            age, self.thresholds.cold_to_glacier_ms
                        ),
                    };
                    // ---- P1: 复制等待门控 ----
                    if replication_status_blocks_lifecycle(meta.replication_status, &plan.action) {
                        *self.replication_blocked_counter.lock() += 1;
                        continue;
                    }
                    if apply {
                        meta.class = StorageClass::Glacier;
                        meta.last_transition_ms = now_ms;
                        *self.transition_counter.lock() += 1;
                        if let Some(ref tracker) = budget_tracker {
                            tracker.record_migration(meta.size_bytes);
                        }
                    }
                    plans.push(plan);
                },
                _ => {},
            }
        }

        // v2.3: 保存扫描统计快照
        if let Some(ref tracker) = budget_tracker {
            *self.last_scan_stats.lock() = Some(tracker.stats());
        }

        plans
    }

    /// 聚合统计
    pub fn stats(&self, now_ms: u64) -> CloudLifecycleStats {
        let mut s = CloudLifecycleStats {
            scanned_at_ms: now_ms,
            transitions_last_24h: *self.transition_counter.lock(),
            restores_last_24h: *self.restore_counter.lock(),
            replication_blocked_count: *self.replication_blocked_counter.lock(),
            delete_all_short_circuit_count: *self.delete_all_short_circuit_counter.lock(),
            ..Default::default()
        };
        let objs = self.objects.lock();
        for meta in objs.values() {
            match meta.class {
                StorageClass::Hot => {
                    s.objects_hot += 1;
                    s.bytes_hot += meta.size_bytes;
                },
                StorageClass::Warm => {
                    s.objects_warm += 1;
                    s.bytes_warm += meta.size_bytes;
                },
                StorageClass::Cold => {
                    s.objects_cold += 1;
                    s.bytes_cold += meta.size_bytes;
                },
                StorageClass::Glacier => {
                    s.objects_glacier += 1;
                    s.bytes_glacier += meta.size_bytes;
                },
            }
        }
        s
    }

    /// 查询单个对象当前类
    pub fn class_of(&self, bucket: &str, key: &str) -> Option<StorageClass> {
        self.objects.lock().get(&(bucket.to_string(), key.to_string())).map(|m| m.class)
    }

    /// 查询对象元数据（只读克隆）
    pub fn meta_of(&self, bucket: &str, key: &str) -> Option<LifecycleObjectMeta> {
        self.objects.lock().get(&(bucket.to_string(), key.to_string())).cloned()
    }

    pub fn object_count(&self) -> usize {
        self.objects.lock().len()
    }
}

/// 便捷：将 Arc 化引擎共享给并发系统
pub type SharedLifecycle = Arc<HotWarmColdLifecycle>;

/// v2.1 向后兼容别名：旧代码 `LifecycleEngine::default()` 可继续使用
pub use HotWarmColdLifecycle as LifecycleEngine;

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_MS: u64 = 24 * 60 * 60 * 1000;

    fn make_meta(bucket: &str, key: &str, created_ms: u64, sz: u64) -> LifecycleObjectMeta {
        LifecycleObjectMeta {
            key: key.into(),
            bucket: bucket.into(),
            size_bytes: sz,
            class: StorageClass::Hot,
            created_at_ms: created_ms,
            last_accessed_at_ms: created_ms,
            last_transition_ms: created_ms,
            version_id: "null".to_string(),
            replication_status: ObjectReplicationStatus::None,
            object_locked: false,
        }
    }

    #[test]
    fn t_a1_1_hot_to_warm_after_31d() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("b1", "k1", t0, 1024));
        assert_eq!(lc.class_of("b1", "k1"), Some(StorageClass::Hot));
        let t31 = t0 + 31 * DAY_MS;
        let plans = lc.transition_scan(t31, true);
        assert!(!plans.is_empty(), "expect at least 1 plan");
        assert_eq!(plans[0].from, StorageClass::Hot);
        assert_eq!(plans[0].to, StorageClass::Warm);
        assert_eq!(lc.class_of("b1", "k1"), Some(StorageClass::Warm));
    }

    #[test]
    fn t_a1_2_warm_touch_restores_to_hot() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("b1", "k1", t0, 1));
        let t31 = t0 + 31 * DAY_MS;
        lc.transition_scan(t31, true);
        assert_eq!(lc.class_of("b1", "k1"), Some(StorageClass::Warm));
        let (c, restored) = lc.touch_and_restore_to_hot("b1", "k1", t31 + 5).unwrap();
        assert_eq!(c, StorageClass::Hot);
        assert!(restored, "WARM touch should restore");
        let meta = lc.meta_of("b1", "k1").unwrap();
        assert_eq!(meta.last_accessed_at_ms, t31 + 5);
    }

    #[test]
    fn t_a1_3_cold_restore_to_hot() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("b1", "big.log", t0, 100_000_000));
        let t95 = t0 + 95 * DAY_MS;
        let plans = lc.transition_scan(t95, true);
        // 先迁到 WARM (30d 阈值) 再迁到 COLD (90d 阈值)；一次 scan 应该两步：我们实现的是按当前类匹配一步
        // 所以需两次 scan（或一次 scan 内多次迭代——我们做一次，但 scan 只做单向单步；所以这里：
        //   t95 - t0 = 95 day ≥ 30d → class HOT → WARM
        // 再 scan 一次：95 ≥ 90d (warm_to_cold) anchor=t95 变成 t0(=created)，所以：
        //   我们的 anchor 是 max(created,last_accessed,last_transition)
        //   第一次 scan 后 last_transition = t95。再次 scan anchor = t95; age=0 → 不再迁
        // 所以我们需要：直接让 age = now - created（在当前实现里我们取了 max(created,acc,trans) 的 age ）
        // 所以要演示 WARM→COLD：需第一次 transition 时间为 t31，第二次 t95 时 age = t95 - t31 = 64d < 90d → 不够
        // 因此让我们直接手改 meta 到 WARM 且 anchor = t0 (将 last_transition 与 last_accessed 改回 t0)
        // → 这更符合现实：用户从未访问过，anchor 保持 t0。
        drop(plans);
        // 手动模拟 unaccessed warm state
        {
            let mut objs = lc.objects.lock();
            let m = objs.get_mut(&("b1".to_string(), "big.log".to_string())).unwrap();
            m.class = StorageClass::Warm;
            m.last_transition_ms = t0;
            m.last_accessed_at_ms = t0;
        }
        let plans2 = lc.transition_scan(t95, true);
        assert_eq!(plans2.len(), 1);
        assert_eq!(plans2[0].from, StorageClass::Warm);
        assert_eq!(plans2[0].to, StorageClass::Cold);
        let (c2, restored2) = lc.touch_and_restore_to_hot("b1", "big.log", t95 + 5).unwrap();
        assert_eq!(c2, StorageClass::Hot);
        assert!(restored2);
    }

    #[test]
    fn t_a1_4_stats_json_roundtrip() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("b1", "a", t0, 100));
        lc.upsert_object(make_meta("b1", "b", t0, 200));
        // put b into cold directly
        {
            let mut objs = lc.objects.lock();
            objs.get_mut(&("b1".into(), "b".into())).unwrap().class = StorageClass::Cold;
        }
        let s = lc.stats(t0 + 1000);
        let j = serde_json::to_string(&s).unwrap();
        let s2: CloudLifecycleStats = serde_json::from_str(&j).unwrap();
        assert_eq!(s, s2);
        assert_eq!(s.objects_hot, 1);
        assert_eq!(s.objects_cold, 1);
        assert_eq!(s.bytes_hot, 100);
        assert_eq!(s.bytes_cold, 200);
    }

    #[test]
    fn t_a1_5_empty_scan_no_crash() {
        let lc = HotWarmColdLifecycle::default();
        let plans = lc.transition_scan(12345, true);
        assert!(plans.is_empty());
        let s = lc.stats(0);
        assert_eq!(s.objects_hot, 0);
        assert_eq!(s.objects_warm, 0);
        assert_eq!(s.objects_cold, 0);
    }

    #[test]
    fn t_a1_6_mixed_scan_applies_only_threshold_violators() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        // 10d → stay HOT
        lc.upsert_object(make_meta("b", "fresh", t0 + 20 * DAY_MS, 1));
        // 40d anchor with HOT → → WARM
        lc.upsert_object(make_meta("b", "aging", t0, 2));
        let t_now = t0 + 40 * DAY_MS;
        let plans = lc.transition_scan(t_now, true);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].key, "aging");
        assert_eq!(lc.class_of("b", "fresh"), Some(StorageClass::Hot));
    }

    #[test]
    fn t_a1_7_hot_access_prolongs_stay() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("b", "k", t0, 1));
        // 访问时间：t0 + 29d (anchor 变为 t0+29d)
        lc.touch_and_restore_to_hot("b", "k", t0 + 29 * DAY_MS);
        // t_now = t0 + 31d → age = 31 - 29 = 2d < 30 → stay HOT
        let plans = lc.transition_scan(t0 + 31 * DAY_MS, true);
        assert_eq!(plans.len(), 0, "object accessed 2d ago should stay HOT");
        assert_eq!(lc.class_of("b", "k"), Some(StorageClass::Hot));
    }

    // ---------- T25-1 Glacier v2.1 tests (TDD RED phase) ----------
    #[test]
    fn t25_01_storage_class_glacier_exists_and_serializes() {
        // Glacier enum variant 必须存在并序列化为 "GLACIER"
        let sc = StorageClass::Glacier;
        let j = serde_json::to_string(&sc).unwrap();
        assert_eq!(j, "\"GLACIER\"");
        assert_eq!(sc.as_str(), "GLACIER");
    }

    #[test]
    fn t25_02_cold_to_glacier_transition_after_366d() {
        // COLD 且 anchor 时间 ≥ cold_to_glacier_ms (默认 365d) → transition_scan 返回 ColdToGlacier plan
        const YEAR_MS: u64 = 365 * DAY_MS;
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("archive", "2023-finance.zip", t0, 10_000_000));
        {
            let mut objs = lc.objects.lock();
            let m = objs.get_mut(&("archive".to_string(), "2023-finance.zip".to_string())).unwrap();
            m.class = StorageClass::Cold;
            m.last_transition_ms = t0;
            m.last_accessed_at_ms = t0;
        }
        // 366 天：超过 365d 阈值
        let t366 = t0 + YEAR_MS + DAY_MS;
        let plans = lc.transition_scan(t366, true);
        assert!(!plans.is_empty(), "expect ColdToGlacier plan after 366d for COLD object");
        // 有一条 ColdToGlacier
        let glacier_plan =
            plans.iter().find(|p| matches!(p.action, TransitionAction::ColdToGlacier));
        assert!(
            glacier_plan.is_some(),
            "ColdToGlacier action missing; got actions: {:?}",
            plans.iter().map(|p| format!("{:?}", p.action)).collect::<Vec<_>>()
        );
        let p = glacier_plan.unwrap();
        assert_eq!(p.from, StorageClass::Cold);
        assert_eq!(p.to, StorageClass::Glacier);
        assert_eq!(lc.class_of("archive", "2023-finance.zip"), Some(StorageClass::Glacier));
    }

    #[test]
    fn t25_03_cold_object_364d_stays_not_glacier() {
        const YEAR_MS: u64 = 365 * DAY_MS;
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("archive", "2023-finance.zip", t0, 1));
        {
            let mut objs = lc.objects.lock();
            let m = objs.get_mut(&("archive".to_string(), "2023-finance.zip".to_string())).unwrap();
            m.class = StorageClass::Cold;
            m.last_transition_ms = t0;
            m.last_accessed_at_ms = t0;
        }
        let t364 = t0 + YEAR_MS - DAY_MS; // 364 天 < 365d 阈值
        let plans = lc.transition_scan(t364, true);
        let has_cold_glacier =
            plans.iter().any(|p| matches!(p.action, TransitionAction::ColdToGlacier));
        assert!(
            !has_cold_glacier,
            "364d COLD object must NOT generate ColdToGlacier; plans count={}",
            plans.len()
        );
        assert_eq!(lc.class_of("archive", "2023-finance.zip"), Some(StorageClass::Cold));
    }

    #[test]
    fn t25_04_lifecycleengine_backward_alias_compat() {
        // 兼容别名：pub use HotWarmColdLifecycle as LifecycleEngine 必须存在
        // 且所有 v2.0 既有 tests (stats/upsert_object/class_of) 行为一致
        let lc = LifecycleEngine::default();
        let t0 = 1_700_000_000_000u64;
        lc.upsert_object(make_meta("compat", "k", t0, 42));
        assert_eq!(lc.class_of("compat", "k"), Some(StorageClass::Hot));
        let s = lc.stats(t0 + 1);
        assert_eq!(s.objects_hot, 1);
        assert_eq!(s.bytes_hot, 42);
        // 默认阈值：hot_to_warm_ms=30d, warm_to_cold_ms=90d
        let t = lc.thresholds();
        assert_eq!(t.hot_to_warm_ms, 30 * DAY_MS);
        assert_eq!(t.warm_to_cold_ms, 90 * DAY_MS);
        // 新字段 cold_to_glacier_ms 默认 365 天
        assert_eq!(t.cold_to_glacier_ms, 365 * DAY_MS);
    }

    // =======================================================================
    // P1 优化新增测试（v2.2）
    // =======================================================================

    /// 测试复制等待门控判断函数的各种组合
    #[test]
    fn test_replication_status_blocks_lifecycle() {
        // Pending + HotToWarm → 门控
        assert!(replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Pending,
            &TransitionAction::HotToWarm
        ));
        // Completed + HotToWarm → 不门控
        assert!(!replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Completed,
            &TransitionAction::HotToWarm
        ));
        // None + DeleteAllVersions → 不门控
        assert!(!replication_status_blocks_lifecycle(
            ObjectReplicationStatus::None,
            &TransitionAction::DeleteAllVersions
        ));
        // Failed + WarmToCold → 门控
        assert!(replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Failed,
            &TransitionAction::WarmToCold
        ));
        // Pending + ColdToGlacier → 门控
        assert!(replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Pending,
            &TransitionAction::ColdToGlacier
        ));
        // Pending + DeleteVersion → 门控
        assert!(replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Pending,
            &TransitionAction::DeleteVersion
        ));
        // Pending + WarmRestoreToHot → 不门控（Restore 类不由生命周期扫描触发）
        assert!(!replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Pending,
            &TransitionAction::WarmRestoreToHot
        ));
        // Completed + DeleteAllVersions → 不门控
        assert!(!replication_status_blocks_lifecycle(
            ObjectReplicationStatus::Completed,
            &TransitionAction::DeleteAllVersions
        ));
    }

    /// 测试 transition_scan 尊重复制等待门控：
    /// Pending 状态不生成迁移计划；改为 Completed 后生成迁移计划
    #[test]
    fn test_transition_scan_respects_replication_gate() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        let t31 = t0 + 31 * DAY_MS;

        // 创建一个 replication_status = Pending 的对象（年龄 ≥ 30d，应迁移但被门控）
        let mut pending_meta = make_meta("b1", "pending-obj", t0, 1024);
        pending_meta.replication_status = ObjectReplicationStatus::Pending;
        lc.upsert_object(pending_meta);

        // 调用 transition_scan，验证不生成迁移计划（被门控）
        let plans_pending = lc.transition_scan(t31, true);
        assert!(
            plans_pending.is_empty(),
            "Pending replication object should be gated, got {} plans",
            plans_pending.len()
        );
        // 对象应仍为 HOT（未被迁移）
        assert_eq!(lc.class_of("b1", "pending-obj"), Some(StorageClass::Hot));

        // 验证门控统计
        let stats = lc.stats(t31);
        assert_eq!(stats.replication_blocked_count, 1);

        // 将 replication_status 改为 Completed，再次调用，验证生成迁移计划
        {
            let mut objs = lc.objects.lock();
            let m = objs.get_mut(&("b1".to_string(), "pending-obj".to_string())).unwrap();
            m.replication_status = ObjectReplicationStatus::Completed;
        }
        let plans_completed = lc.transition_scan(t31, true);
        assert_eq!(plans_completed.len(), 1, "Completed replication should allow transition");
        assert_eq!(plans_completed[0].action, TransitionAction::HotToWarm);
        assert_eq!(lc.class_of("b1", "pending-obj"), Some(StorageClass::Warm));
    }

    /// 测试 DeleteAllVersions 短路评估：
    /// 无 Object Lock + 无 Pending → Some；启用 Object Lock → None；Pending → None
    #[test]
    fn test_delete_all_versions_short_circuit() {
        let lc = HotWarmColdLifecycle::default();
        let now = 1_700_000_000_000u64;

        // 创建对象版本列表（无 Object Lock、无 Pending 复制）
        let v1 = LifecycleObjectMeta {
            key: "data/file.bin".into(),
            bucket: "b1".into(),
            size_bytes: 100,
            class: StorageClass::Hot,
            created_at_ms: now - 1000,
            last_accessed_at_ms: now - 1000,
            last_transition_ms: now - 1000,
            version_id: "v1".into(),
            replication_status: ObjectReplicationStatus::Completed,
            object_locked: false,
        };
        let v2 = LifecycleObjectMeta {
            version_id: "v2".into(),
            replication_status: ObjectReplicationStatus::None,
            ..v1.clone()
        };
        let versions = vec![v1.clone(), v2.clone()];

        // 无 Object Lock + 无 Pending → 返回 Some(plan)
        let plan = lc.evaluate_delete_all_versions("b1", "data/file.bin", &versions, now, false);
        assert!(plan.is_some(), "should short-circuit when no lock and no pending");
        let plan = plan.unwrap();
        assert_eq!(plan.bucket, "b1");
        assert_eq!(plan.key, "data/file.bin");
        assert_eq!(plan.version_ids, vec!["v1".to_string(), "v2".to_string()]);
        assert_eq!(plan.reason, "DeleteAllVersions short-circuit");

        // 启用 Object Lock → 返回 None
        let plan_locked =
            lc.evaluate_delete_all_versions("b1", "data/file.bin", &versions, now, true);
        assert!(plan_locked.is_none(), "should NOT short-circuit when object lock enabled");

        // 有一个版本 Pending → 返回 None
        let mut versions_pending = versions.clone();
        versions_pending[1].replication_status = ObjectReplicationStatus::Pending;
        let plan_pending =
            lc.evaluate_delete_all_versions("b1", "data/file.bin", &versions_pending, now, false);
        assert!(plan_pending.is_none(), "should NOT short-circuit when any version is Pending");

        // 有一个版本被锁定 → 返回 None
        let mut versions_obj_locked = versions.clone();
        versions_obj_locked[0].object_locked = true;
        let plan_obj_locked = lc.evaluate_delete_all_versions(
            "b1",
            "data/file.bin",
            &versions_obj_locked,
            now,
            false,
        );
        assert!(plan_obj_locked.is_none(), "should NOT short-circuit when any version is locked");
    }

    /// 测试新创建的 LifecycleObjectMeta 的 replication_status 默认为 None
    #[test]
    fn test_lifecycle_object_meta_default_replication_status() {
        // 通过反序列化验证 serde(default) 生效
        let json = r#"{
            "key": "test.txt",
            "bucket": "b1",
            "size_bytes": 42,
            "class": "HOT",
            "created_at_ms": 1000,
            "last_accessed_at_ms": 1000,
            "last_transition_ms": 1000
        }"#;
        let meta: LifecycleObjectMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.replication_status, ObjectReplicationStatus::None);
        assert_eq!(meta.version_id, "null");
        assert!(!meta.object_locked);

        // 直接构造验证默认值
        let meta2 = make_meta("b2", "k2", 0, 1);
        assert_eq!(meta2.replication_status, ObjectReplicationStatus::None);
        assert_eq!(meta2.version_id, "null");
        assert!(!meta2.object_locked);
    }

    /// 测试 delete_all_scan 与 transition_scan 的短路联动：
    /// DeleteAllVersions 候选对象在 transition_scan 中被跳过
    #[test]
    fn test_delete_all_scan_and_transition_short_circuit() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        let t31 = t0 + 31 * DAY_MS;

        // 对象 A：DeleteAllVersions 候选（无锁、无 Pending），年龄 ≥ 30d
        lc.upsert_object(make_meta("b1", "obj-a", t0, 100));
        lc.mark_delete_all_candidate("b1", "obj-a");

        // 对象 B：非候选，年龄 ≥ 30d，应正常迁移
        lc.upsert_object(make_meta("b1", "obj-b", t0, 200));

        // transition_scan：obj-a 应被短路跳过，obj-b 应生成迁移计划
        let plans = lc.transition_scan(t31, false);
        assert_eq!(plans.len(), 1, "only non-candidate should generate transition plan");
        assert_eq!(plans[0].key, "obj-b");

        // delete_all_scan：obj-a 应生成全版本删除计划
        let delete_plans = lc.delete_all_scan(t31, false);
        assert_eq!(delete_plans.len(), 1);
        assert_eq!(delete_plans[0].key, "obj-a");
        assert_eq!(delete_plans[0].version_ids, vec!["null".to_string()]);

        // apply=true 时对象被移除
        let delete_plans_apply = lc.delete_all_scan(t31, true);
        assert_eq!(delete_plans_apply.len(), 1);
        assert_eq!(lc.object_count(), 1, "obj-a should be removed");
        assert!(lc.meta_of("b1", "obj-a").is_none());
        assert!(lc.meta_of("b1", "obj-b").is_some());

        // 验证短路统计
        let stats = lc.stats(t31);
        assert_eq!(stats.delete_all_short_circuit_count, 1);
    }

    // =======================================================================
    // v2.3: 三维扫描预算集成测试
    // =======================================================================

    /// 测试 transition_scan 集成预算后，容量超限时提前终止
    #[test]
    fn test_scan_budget_integration_max_objects() {
        use crate::scanner::{CapacityBudget, ScanBudget};

        // 设置 max_objects_per_scan = 2
        let budget = ScanBudget {
            capacity: CapacityBudget { max_objects_per_scan: 2, ..Default::default() },
            ..Default::default()
        };
        let lc = HotWarmColdLifecycle::default().with_scan_budget(budget);
        let t0 = 1_700_000_000_000u64;
        let t31 = t0 + 31 * DAY_MS;

        // 插入 5 个年龄 ≥ 30d 的对象（都应迁移）
        for i in 0..5 {
            lc.upsert_object(make_meta("b1", &format!("obj-{i}"), t0, 100));
        }

        // 扫描：预算限制最多扫描 2 个对象，应只处理前 2 个
        let plans = lc.transition_scan(t31, true);
        assert_eq!(plans.len(), 2, "budget should limit scan to 2 objects, got {}", plans.len());

        // 验证统计快照
        let stats = lc.last_scan_stats().expect("scan stats should be set");
        assert_eq!(stats.objects_scanned, 2);
        assert!(stats.budget_exceeded, "should mark budget_exceeded");

        // 验证只有 2 个对象被迁移
        let mut warm_count = 0;
        for i in 0..5 {
            if lc.class_of("b1", &format!("obj-{i}")) == Some(StorageClass::Warm) {
                warm_count += 1;
            }
        }
        assert_eq!(warm_count, 2, "only 2 objects should be migrated");
    }

    /// 测试无预算时 transition_scan 行为不变（向后兼容）
    #[test]
    fn test_scan_budget_none_backward_compatible() {
        let lc = HotWarmColdLifecycle::default();
        let t0 = 1_700_000_000_000u64;
        let t31 = t0 + 31 * DAY_MS;

        for i in 0..5 {
            lc.upsert_object(make_meta("b1", &format!("obj-{i}"), t0, 100));
        }

        let plans = lc.transition_scan(t31, true);
        assert_eq!(plans.len(), 5, "no budget should scan all objects");
        assert!(lc.last_scan_stats().is_none(), "no budget → no stats snapshot");
    }

    /// 测试 with_scan_budget 构建器和 scan_budget() 访问器
    #[test]
    fn test_with_scan_budget_accessor() {
        use crate::scanner::ScanBudget;

        let lc = HotWarmColdLifecycle::default();
        assert!(lc.scan_budget().is_none());

        let budget = ScanBudget::default();
        let lc = lc.with_scan_budget(budget);
        assert!(lc.scan_budget().is_some());
    }
}
