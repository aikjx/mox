// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use mox_cloud_s3_svc::lifecycle::{CloudLifecycleStats, HotWarmColdLifecycle, LifecycleThresholds};
use std::time::Duration;
fn main() {
    let t = LifecycleThresholds::default();
    let lc = HotWarmColdLifecycle::new(t.clone());
    // 模拟对象集：hot=700 warm=200 cold=100
    let base = std::time::SystemTime::now();
    for i in 0..700 { lc.touch_object("b1", &format!("hot_{i}"), base); }
    let warm_base = base.checked_sub(Duration::from_secs(60*60*24*60)).unwrap();
    for i in 0..200 { lc.touch_object("b1", &format!("warm_{i}"), warm_base); }
    let cold_base = base.checked_sub(Duration::from_secs(60*60*24*180)).unwrap();
    for i in 0..100 { lc.touch_object("b1", &format!("cold_{i}"), cold_base); }
    lc.scan_transition();
    let stats: CloudLifecycleStats = lc.stats();
    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
}
