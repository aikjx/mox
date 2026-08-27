// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_cloud_sdk::Client;

#[tokio::main]
async fn main() {
    let c = Client::new();
    for i in 0..5u8 {
        c.put_object("logs", &format!("2024/Jan/{:02}.log", i), vec![i; 16]).await.unwrap();
    }
    c.put_object("logs", "2023/Dec/31.log", vec![0u8; 8]).await.unwrap();
    let items = c.list_prefix("logs", "2024/Jan/", None).await.unwrap();
    assert_eq!(items.len(), 5);
    println!("XJ-OK: cloud-009_list_prefix");
}
