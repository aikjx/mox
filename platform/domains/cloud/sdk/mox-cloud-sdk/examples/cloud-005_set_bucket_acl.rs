// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.create_bucket("pub-read").await.unwrap();
    c.set_bucket_acl("pub-read", "public-read").await.unwrap();
    let info = c.head_bucket("pub-read").await.unwrap();
    assert_eq!(info.acl, "public-read");
    println!("XJ-OK: cloud-005_set_bucket_acl");
}
