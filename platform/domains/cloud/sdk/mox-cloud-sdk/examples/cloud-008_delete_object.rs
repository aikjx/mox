// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("bkt", "trash.bin", b"scrap".to_vec()).await.unwrap();
    c.delete_object("bkt", "trash.bin").await.unwrap();
    let err = c.get_object("bkt", "trash.bin").await.unwrap_err();
    assert!(matches!(err, mox_cloud_sdk::CloudError::NotFound(_)));
    println!("XJ-OK: cloud-008_delete_object");
}
