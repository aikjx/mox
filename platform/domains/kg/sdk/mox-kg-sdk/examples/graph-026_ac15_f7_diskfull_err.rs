// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::{Client, GraphError};

#[tokio::main]
async fn main() {
    let g = Client::new();
    // Baseline: without fault → OK
    let (bytes, _r) = g.ac15_f7_diskfull(1024).await.unwrap();
    assert_eq!(bytes, 1024);
    // Inject f7 fault → DiskFull error
    g.ac15_inject("f7").await.unwrap();
    let err = g.ac15_f7_diskfull(4096).await.unwrap_err();
    assert!(matches!(err, GraphError::DiskFull(_)), "got: {:?}", err);
    let report = g.ac15_report().await.unwrap();
    assert!(report.diskfull_triggered);
    println!("XJ-OK: graph-026_ac15_f7_diskfull_err");
}
