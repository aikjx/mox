// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("alpha").await.unwrap();
    c.create_bucket("beta").await.unwrap();
    c.create_bucket("gamma").await.unwrap();
    let list = c.list_buckets().await.unwrap();
    assert_eq!(list.len(), 3);
    println!("XJ-OK: cloud-003_list_buckets");
}
