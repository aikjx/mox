// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, GraphError};

#[tokio::main]
async fn main() {
    let g = Client::new();
    // No fault → normal success
    let (dedup, _) = g.ac15_f12_timeout_dedup(7).await.unwrap();
    assert_eq!(dedup, 7);
    // Inject fault → Timeout
    g.ac15_inject("f12").await.unwrap();
    let err = g.ac15_f12_timeout_dedup(3).await.unwrap_err();
    assert!(matches!(err, GraphError::Timeout(_)), "got: {:?}", err);
    let r = g.ac15_report().await.unwrap();
    assert_eq!(r.timeout_hits, 1);
    assert_eq!(r.dedup_hits, 10); // 7 + 3
    println!("XJ-OK: graph-028_ac15_f12_timeout_dedup");
}
