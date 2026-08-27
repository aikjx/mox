// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::prelude::*;

#[tokio::main]
async fn main() {
    let client = Client::new();
    let bucket = "complete-bucket";
    let key = "final/assembly.dat";
    let uid = client
        .create_multipart_upload(bucket, key)
        .await
        .unwrap();
    let mut parts: Vec<PartEtag> = Vec::new();
    for n in 1..=3u16 {
        let data = format!("PART-{}-DATA-", n).into_bytes();
        let pe = client
            .upload_part(bucket, key, &uid, n, data)
            .await
            .unwrap();
        parts.push(pe);
    }
    let final_etag = client
        .complete_multipart_upload(bucket, key, &uid, parts)
        .await
        .unwrap();
    assert!(!final_etag.is_empty());
    let obj = client.get_object(bucket, key).await.unwrap();
    assert_eq!(obj.len(), 3 * 14); // "PART-N-DATA-" is 14 bytes each
    println!("XJ-OK: t3_03_complete etag={} size={}", final_etag, obj.len());
}
