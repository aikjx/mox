//! L4 生命周期评估抽象 —— 对象存储分级（Hot/Warm/Cold/Glacier）与过期策略。
//!
//! [`LifecycleEvaluator`] 是纯同步的策略 trait，不涉及 I/O，由生命周期管理
//! 服务调用以判断对象是否应迁移存储层级、是否过期、下次扫描时间等。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 存储层级与迁移
// ---------------------------------------------------------------------------

/// 对象存储层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageClass {
    Hot,
    Warm,
    Cold,
    Glacier,
}

impl std::fmt::Display for StorageClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageClass::Hot => write!(f, "hot"),
            StorageClass::Warm => write!(f, "warm"),
            StorageClass::Cold => write!(f, "cold"),
            StorageClass::Glacier => write!(f, "glacier"),
        }
    }
}

/// 存储层级迁移决策。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageClassTransition {
    pub from: StorageClass,
    pub to: StorageClass,
    pub reason: String,
}

// ---------------------------------------------------------------------------
// 复制状态与生命周期动作
// ---------------------------------------------------------------------------

/// 跨区域复制状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReplicationStatus {
    None,
    Pending,
    Completed,
    Failed,
}

impl std::fmt::Display for ReplicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReplicationStatus::None => write!(f, "none"),
            ReplicationStatus::Pending => write!(f, "pending"),
            ReplicationStatus::Completed => write!(f, "completed"),
            ReplicationStatus::Failed => write!(f, "failed"),
        }
    }
}

/// 生命周期动作枚举。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleAction {
    None,
    Transition { from: StorageClass, to: StorageClass },
    RestoreToHot { from: StorageClass },
    DeleteVersion { version_id: String },
    DeleteAllVersions,
}

impl std::fmt::Display for LifecycleAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleAction::None => write!(f, "none"),
            LifecycleAction::Transition { from, to } => {
                write!(f, "transition({}->{})", from, to)
            },
            LifecycleAction::RestoreToHot { from } => write!(f, "restore-to-hot({})", from),
            LifecycleAction::DeleteVersion { version_id } => {
                write!(f, "delete-version({})", version_id)
            },
            LifecycleAction::DeleteAllVersions => write!(f, "delete-all-versions"),
        }
    }
}

// ---------------------------------------------------------------------------
// 阈值与对象元数据
// ---------------------------------------------------------------------------

/// 生命周期迁移阈值（单位：天）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LifecycleThresholds {
    pub hot_to_warm_days: u64,
    pub warm_to_cold_days: u64,
    pub cold_to_glacier_days: u64,
}

impl Default for LifecycleThresholds {
    fn default() -> Self {
        Self { hot_to_warm_days: 30, warm_to_cold_days: 90, cold_to_glacier_days: 180 }
    }
}

/// 对象生命周期评估所需的元数据快照。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectLifecycleMeta {
    pub bucket: String,
    pub key: String,
    pub size_bytes: u64,
    pub class: StorageClass,
    pub created_at_ms: u64,
    pub last_accessed_at_ms: u64,
    pub last_transition_ms: u64,
    pub version_id: String,
    pub replication_status: ReplicationStatus,
    pub object_locked: bool,
}

// ---------------------------------------------------------------------------
// 核心 trait
// ---------------------------------------------------------------------------

/// L4 生命周期评估抽象（纯同步，无 I/O）。
///
/// trait 是 object-safe 的，所有方法均为 `&self` 且返回具体类型。
pub trait LifecycleEvaluator: Send + Sync {
    /// 判断对象是否应发生存储层级迁移，返回迁移决策；无需迁移则返回 `None`。
    fn should_transition(
        &self,
        meta: &ObjectLifecycleMeta,
        now_ms: u64,
        thresholds: &LifecycleThresholds,
    ) -> Option<StorageClassTransition>;

    /// 判断对象是否已过期（应被删除）。
    fn should_expire(&self, meta: &ObjectLifecycleMeta, now_ms: u64) -> bool;

    /// 计算下次扫描时间戳（毫秒）。
    fn next_scan_time(&self, last_scan_ms: u64, scan_interval_sec: u64) -> u64;

    /// 判断当前复制状态是否阻塞指定的生命周期动作。
    fn replication_blocks(&self, status: &ReplicationStatus, action: &LifecycleAction) -> bool;
}

// ---------------------------------------------------------------------------
// 单元测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyEvaluator;

    impl LifecycleEvaluator for DummyEvaluator {
        fn should_transition(
            &self,
            meta: &ObjectLifecycleMeta,
            now_ms: u64,
            thresholds: &LifecycleThresholds,
        ) -> Option<StorageClassTransition> {
            let age_days = (now_ms.saturating_sub(meta.last_accessed_at_ms)) / 86_400_000;
            match meta.class {
                StorageClass::Hot if age_days >= thresholds.hot_to_warm_days => {
                    Some(StorageClassTransition {
                        from: StorageClass::Hot,
                        to: StorageClass::Warm,
                        reason: format!("not accessed for {} days", age_days),
                    })
                },
                _ => None,
            }
        }

        fn should_expire(&self, meta: &ObjectLifecycleMeta, now_ms: u64) -> bool {
            now_ms.saturating_sub(meta.created_at_ms) > 365 * 86_400_000
        }

        fn next_scan_time(&self, last_scan_ms: u64, scan_interval_sec: u64) -> u64 {
            last_scan_ms + scan_interval_sec * 1000
        }

        fn replication_blocks(&self, status: &ReplicationStatus, action: &LifecycleAction) -> bool {
            matches!(status, ReplicationStatus::Pending)
                && matches!(action, LifecycleAction::DeleteAllVersions)
        }
    }

    #[test]
    fn test_types_construct() {
        let thresholds = LifecycleThresholds::default();
        assert_eq!(thresholds.hot_to_warm_days, 30);
        assert_eq!(thresholds.warm_to_cold_days, 90);
        assert_eq!(thresholds.cold_to_glacier_days, 180);

        let meta = ObjectLifecycleMeta {
            bucket: "my-bucket".into(),
            key: "path/object.dat".into(),
            size_bytes: 1024,
            class: StorageClass::Hot,
            created_at_ms: 0,
            last_accessed_at_ms: 0,
            last_transition_ms: 0,
            version_id: "v1".into(),
            replication_status: ReplicationStatus::None,
            object_locked: false,
        };
        assert_eq!(meta.class, StorageClass::Hot);
        assert!(!meta.object_locked);

        let transition = StorageClassTransition {
            from: StorageClass::Hot,
            to: StorageClass::Warm,
            reason: "age".into(),
        };
        assert_eq!(transition.from, StorageClass::Hot);

        let action =
            LifecycleAction::Transition { from: StorageClass::Warm, to: StorageClass::Cold };
        assert_eq!(action.to_string(), "transition(warm->cold)");
        assert_eq!(LifecycleAction::None.to_string(), "none");
        assert_eq!(
            LifecycleAction::DeleteVersion { version_id: "v2".into() }.to_string(),
            "delete-version(v2)"
        );

        assert_eq!(StorageClass::Glacier.to_string(), "glacier");
        assert_eq!(ReplicationStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_trait_object_safe() {
        let evaluator: Box<dyn LifecycleEvaluator> = Box::new(DummyEvaluator);

        let meta = ObjectLifecycleMeta {
            bucket: "b".into(),
            key: "k".into(),
            size_bytes: 0,
            class: StorageClass::Hot,
            created_at_ms: 0,
            last_accessed_at_ms: 0,
            last_transition_ms: 0,
            version_id: "v".into(),
            replication_status: ReplicationStatus::None,
            object_locked: false,
        };

        let thresholds = LifecycleThresholds::default();
        let now = 40 * 86_400_000u64; // 40 天
        let transition = evaluator.should_transition(&meta, now, &thresholds);
        assert!(transition.is_some());
        assert_eq!(transition.unwrap().to, StorageClass::Warm);

        assert!(evaluator.should_expire(&meta, 400 * 86_400_000));
        assert!(!evaluator.should_expire(&meta, 10 * 86_400_000));

        assert_eq!(evaluator.next_scan_time(1000, 60), 1000 + 60_000);

        assert!(evaluator
            .replication_blocks(&ReplicationStatus::Pending, &LifecycleAction::DeleteAllVersions));
        assert!(!evaluator.replication_blocks(
            &ReplicationStatus::Completed,
            &LifecycleAction::DeleteAllVersions
        ));
    }
}
