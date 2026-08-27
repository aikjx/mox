// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let uid = client
        .create_multipart_upload("my-bucket", "data/large-file.bin")
        .await
        .expect("create multipart upload");
    assert!(!uid.is_empty(), "upload_id must not be empty");
    assert!(
        uid.starts_with("mpu-"),
        "upload_id must start with mpu- prefix, got {}",
        uid
    );
    println!("XJ-OK: t3_01_create_upload id={}", uid);
}
