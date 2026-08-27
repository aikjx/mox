// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::{Client, LifecycleRule};

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("coldline").await.unwrap();
    c.lifecycle_put_rule("coldline", LifecycleRule {
        id: "warm->cold@180d".into(),
        from_storage_class: "warm".into(),
        to_storage_class: "cold".into(),
        after_days: 180,
        prefix: "backups/".into(),
    }).await.unwrap();
    let rules = c.lifecycle_list_rules("coldline").await.unwrap();
    let r = rules.iter().find(|r| r.after_days == 180).unwrap();
    assert_eq!(r.to_storage_class, "cold");
    println!("XJ-OK: cloud-026_lifecycle_warm_to_cold_180d");
}
