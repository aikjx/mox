// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    let data = b"hello cloud object storage".to_vec();
    let etag = c.put_object("bkt", "greeting.txt", data.clone()).await.unwrap();
    assert!(!etag.is_empty());
    println!("XJ-OK: cloud-006_put_object");
}
