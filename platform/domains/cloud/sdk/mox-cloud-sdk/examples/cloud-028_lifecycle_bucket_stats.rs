// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::{Client, LifecycleRule};

#[tokio::main]
async fn main() {
    let c = Client::new();
    for i in 0..10u8 {
        let key = format!("data/chunk_{:03}.bin", i);
        c.put_object("sb", &key, vec![i; (i as usize) * 64 + 64]).await.unwrap();
    }
    c.lifecycle_put_rule("sb", LifecycleRule {
        id: "r1".into(), from_storage_class: "hot".into(),
        to_storage_class: "warm".into(), after_days: 30, prefix: "data/".into(),
    }).await.unwrap();
    let stats = c.lifecycle_bucket_stats("sb").await.unwrap();
    assert_eq!(stats.bucket, "sb");
    assert!(stats.transitioned_last_30d > 0);
    println!("XJ-OK: cloud-028_lifecycle_bucket_stats");
}
