// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("existential").await.unwrap();
    let info = c.head_bucket("existential").await.unwrap();
    assert_eq!(info.name, "existential");
    assert_eq!(info.acl, "private");
    println!("XJ-OK: cloud-004_head_bucket");
}
