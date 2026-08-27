// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let info = c.create_bucket("my-first-bucket").await.unwrap();
    assert_eq!(info.name, "my-first-bucket");
    println!("XJ-OK: cloud-001_create_bucket");
}
