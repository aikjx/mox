// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    c.put_object("archives", "2020/q1/report.parquet", b"parquet-bytes".to_vec()).await.unwrap();
    c.lifecycle_restore("archives", "2020/q1/report.parquet", 7).await.unwrap();
    // Restoring a missing object must fail with NotFound
    let err = c.lifecycle_restore("archives", "nope/path.bin", 3).await.unwrap_err();
    assert!(matches!(err, mox_cloud_sdk::CloudError::NotFound(_)), "got: {:?}", err);
    println!("XJ-OK: cloud-027_lifecycle_cold_to_hot_restore");
}
