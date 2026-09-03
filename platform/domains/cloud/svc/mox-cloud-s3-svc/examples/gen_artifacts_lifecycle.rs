// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_s3_svc::lifecycle::{
    CloudLifecycleStats, HotWarmColdLifecycle, LifecycleObjectMeta, LifecycleThresholds,
    ObjectReplicationStatus, StorageClass,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn system_time_to_ms(t: SystemTime) -> u64 {
    t.duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn main() {
    let t = LifecycleThresholds::default();
    let lc = HotWarmColdLifecycle::new(t);
    // 模拟对象集：hot=700 warm=200 cold=100
    let now = system_time_to_ms(SystemTime::now());
    let warm_created = now.saturating_sub(60 * 60 * 24 * 60 * 1000);
    let cold_created = now.saturating_sub(60 * 60 * 24 * 180 * 1000);

    for i in 0..700 {
        lc.upsert_object(LifecycleObjectMeta {
            key: format!("hot_{i}"),
            bucket: "b1".to_string(),
            size_bytes: 1024,
            class: StorageClass::Hot,
            created_at_ms: now,
            last_accessed_at_ms: now,
            last_transition_ms: 0,
            version_id: "null".to_string(),
            replication_status: ObjectReplicationStatus::None,
            object_locked: false,
        });
    }
    for i in 0..200 {
        lc.upsert_object(LifecycleObjectMeta {
            key: format!("warm_{i}"),
            bucket: "b1".to_string(),
            size_bytes: 1024,
            class: StorageClass::Warm,
            created_at_ms: warm_created,
            last_accessed_at_ms: warm_created,
            last_transition_ms: warm_created,
            version_id: "null".to_string(),
            replication_status: ObjectReplicationStatus::None,
            object_locked: false,
        });
    }
    for i in 0..100 {
        lc.upsert_object(LifecycleObjectMeta {
            key: format!("cold_{i}"),
            bucket: "b1".to_string(),
            size_bytes: 1024,
            class: StorageClass::Cold,
            created_at_ms: cold_created,
            last_accessed_at_ms: cold_created,
            last_transition_ms: cold_created,
            version_id: "null".to_string(),
            replication_status: ObjectReplicationStatus::None,
            object_locked: false,
        });
    }
    let _plans = lc.transition_scan(now, true);
    let stats: CloudLifecycleStats = lc.stats(now);
    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
}
