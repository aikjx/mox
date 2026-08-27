// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("src", "orig.bin", b"copy-me-123".to_vec()).await.unwrap();
    c.copy_object("src", "orig.bin", "dst", "clone.bin").await.unwrap();
    let got = c.get_object("dst", "clone.bin").await.unwrap();
    assert_eq!(got, b"copy-me-123");
    println!("XJ-OK: cloud-010_copy_object");
}
