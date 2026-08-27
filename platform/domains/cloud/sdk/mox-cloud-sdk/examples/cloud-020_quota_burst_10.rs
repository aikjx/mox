// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.quota_set("api-v1", 1000, 10).await.unwrap();
    let q = c.quota_get("api-v1").await.unwrap();
    assert_eq!(q.burst, 10);
    // burst of 10 requests pass the check
    for _ in 0..10 {
        c.quota_check("api-v1", 1).await.unwrap();
    }
    println!("XJ-OK: cloud-020_quota_burst_10");
}
