// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::{Client, PartEtag};

#[tokio::main]
async fn main() {
    let c = Client::new();
    let uid = c.create_multipart_upload("big", "huge.bin").await.unwrap();
    let mut parts: Vec<PartEtag> = Vec::new();
    for n in 1..=4u16 {
        let chunk = vec![n as u8; 1024];
        let pe = c.upload_part("big", "huge.bin", &uid, n, chunk).await.unwrap();
        parts.push(pe);
    }
    let final_etag = c.complete_multipart_upload("big", "huge.bin", &uid, parts).await.unwrap();
    assert!(!final_etag.is_empty());
    let assembled = c.get_object("big", "huge.bin").await.unwrap();
    assert_eq!(assembled.len(), 4 * 1024);
    println!("XJ-OK: cloud-011_multipart_upload");
}
