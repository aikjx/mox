// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_kg_sdk::Client;

#[tokio::main]
async fn main() {
    let g = Client::new();
    let (ms, report) = g.ac15_f13_lag_spike(12_500).await.unwrap();
    assert_eq!(ms, 12_500);
    assert_eq!(report.lag_spike_ms, 12_500);
    assert_eq!(report.fault_tag, "f13");
    println!("XJ-OK: graph-029_ac15_f13_lag_spike");
}
