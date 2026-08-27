// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.quota_set("tenant-x", 50, 10).await.unwrap();
    let q = c.quota_get("tenant-x").await.unwrap();
    assert_eq!(q.requests_per_minute, 50);
    c.quota_check("tenant-x", 1).await.unwrap();
    println!("XJ-OK: cloud-019_quota_50_per_min");
}
