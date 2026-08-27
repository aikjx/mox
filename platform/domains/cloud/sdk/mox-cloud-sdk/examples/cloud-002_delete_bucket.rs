// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("doomed-bucket").await.unwrap();
    c.delete_bucket("doomed-bucket").await.unwrap();
    let list = c.list_buckets().await.unwrap();
    assert!(!list.iter().any(|b| b.name == "doomed-bucket"));
    println!("XJ-OK: cloud-002_delete_bucket");
}
