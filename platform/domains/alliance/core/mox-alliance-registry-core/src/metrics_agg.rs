// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家指标聚合纯逻辑（对应 registry-proto 预留的
//! `ExpertDirectory::get_expert_metrics` / `get_platform_overview` 契约）。
//!
//! 背景：侦察确认 registry-proto 里这两个方法仍是「未实现」默认占位。本模块先把它们
//! 背后的**纯计算**抽出来：单专家成败/延迟累加、跨专家健康度汇总。零 IO、零新依赖，
//! 只依赖 std + serde（本 crate 已用）。**不修改 proto 契约本身**——svc 层后续要接线时，
//! 直接调用 [`summarize`] 即可，无需改动既有 trait 签名。
//!
//! 健康阈值（ADR 候选，先在此处固化为纯函数常量）：
//! - 成功率 `success_rate < 0.80` → Degraded；
//! - 或失败占比 `failures / invocations > 0.20` → Degraded；
//! - 无任何调用样本（invocations == 0）视为 Healthy（无失败证据），不误报。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 单专家运行指标。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertMetrics {
    pub expert_id: String,
    pub invocations: u64,
    pub successes: u64,
    pub failures: u64,
    /// 平均延迟（毫秒），随每次结果递增维护。
    pub avg_latency_ms: f64,
    /// 成功率 = successes / invocations；无样本时为 0.0（健康判定另行处理）。
    pub success_rate: f64,
}

impl ExpertMetrics {
    pub fn new(expert_id: impl Into<String>) -> Self {
        Self {
            expert_id: expert_id.into(),
            invocations: 0,
            successes: 0,
            failures: 0,
            avg_latency_ms: 0.0,
            success_rate: 0.0,
        }
    }
}

/// 健康阈值：成功率低于此值即 Degraded。
pub const DEGRADED_SUCCESS_RATE: f64 = 0.80;
/// 健康阈值：失败占比高于此值即 Degraded。
pub const DEGRADED_FAILURE_RATIO: f64 = 0.20;

/// 记录一次专家调用结果，累加计数与滚动平均延迟。
pub fn record_result(m: &mut ExpertMetrics, ok: bool, latency_ms: u64) {
    m.invocations += 1;
    if ok {
        m.successes += 1;
    } else {
        m.failures += 1;
    }
    // 滚动平均：new_avg = (old_avg * (n-1) + latency) / n
    let n = m.invocations as f64;
    m.avg_latency_ms = (m.avg_latency_ms * (n - 1.0) + latency_ms as f64) / n;
    m.success_rate = m.successes as f64 / n;
}

/// 专家健康状态（与 aggregation.rs 的 AggregationHealth 区分：这里是专家业务成功率维度）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpertHealth {
    Healthy,
    Degraded,
}

/// 判定单个专家是否健康。
pub fn classify(m: &ExpertMetrics) -> ExpertHealth {
    if m.invocations == 0 {
        return ExpertHealth::Healthy;
    }
    let failure_ratio = m.failures as f64 / m.invocations as f64;
    if m.success_rate < DEGRADED_SUCCESS_RATE || failure_ratio > DEGRADED_FAILURE_RATIO {
        ExpertHealth::Degraded
    } else {
        ExpertHealth::Healthy
    }
}

/// 平台概览（`get_platform_overview` 契约的纯数据投影）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformOverview {
    pub total_experts: u32,
    pub healthy: u32,
    pub degraded: u32,
    /// 按状态名计数（"healthy" / "degraded"）。
    pub by_status: BTreeMap<String, u32>,
}

/// 把一批专家指标汇总成平台概览。
pub fn summarize<I: IntoIterator<Item = ExpertMetrics>>(iter: I) -> PlatformOverview {
    let mut overview = PlatformOverview::default();
    for m in iter {
        overview.total_experts += 1;
        let status = match classify(&m) {
            ExpertHealth::Healthy => {
                overview.healthy += 1;
                "healthy"
            }
            ExpertHealth::Degraded => {
                overview.degraded += 1;
                "degraded"
            }
        };
        *overview.by_status.entry(status.to_string()).or_insert(0) += 1;
    }
    overview
}

// ─── 单元测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_iterator_yields_zero_overview() {
        let v: Vec<ExpertMetrics> = vec![];
        let o = summarize(v);
        assert_eq!(o.total_experts, 0);
        assert_eq!(o.healthy, 0);
        assert_eq!(o.degraded, 0);
        assert!(o.by_status.is_empty());
    }

    #[test]
    fn all_healthy_experts_counted() {
        let mut a = ExpertMetrics::new("a");
        for _ in 0..10 {
            record_result(&mut a, true, 100);
        }
        let mut b = ExpertMetrics::new("b");
        for _ in 0..5 {
            record_result(&mut b, true, 200);
        }
        let o = summarize(vec![a, b]);
        assert_eq!(o.total_experts, 2);
        assert_eq!(o.healthy, 2);
        assert_eq!(o.degraded, 0);
        assert_eq!(o.by_status.get("healthy"), Some(&2));
    }

    #[test]
    fn one_degraded_expert_detected_by_low_success_rate() {
        let mut good = ExpertMetrics::new("good");
        for _ in 0..10 {
            record_result(&mut good, true, 50);
        }
        // 7 成功 3 失败 → 成功率 0.7 < 0.8 → degraded
        let mut bad = ExpertMetrics::new("bad");
        for _ in 0..7 {
            record_result(&mut bad, true, 50);
        }
        for _ in 0..3 {
            record_result(&mut bad, false, 50);
        }
        let o = summarize(vec![good, bad]);
        assert_eq!(o.total_experts, 2);
        assert_eq!(o.healthy, 1);
        assert_eq!(o.degraded, 1);
        assert_eq!(o.by_status.get("degraded"), Some(&1));
    }

    #[test]
    fn success_rate_and_failure_ratio_formulas() {
        let mut m = ExpertMetrics::new("x");
        record_result(&mut m, true, 10);
        record_result(&mut m, false, 10);
        record_result(&mut m, true, 10);
        assert_eq!(m.invocations, 3);
        assert_eq!(m.successes, 2);
        assert_eq!(m.failures, 1);
        assert!((m.success_rate - 2.0 / 3.0).abs() < 1e-9);
        // 2/3 成功率 < 0.8 → degraded
        assert_eq!(classify(&m), ExpertHealth::Degraded);
    }

    #[test]
    fn average_latency_rolling() {
        let mut m = ExpertMetrics::new("x");
        record_result(&mut m, true, 100);
        assert!((m.avg_latency_ms - 100.0).abs() < 1e-9);
        record_result(&mut m, true, 300);
        // (100*1 + 300) / 2 = 200
        assert!((m.avg_latency_ms - 200.0).abs() < 1e-9);
        record_result(&mut m, true, 500);
        // (100 + 300 + 500) / 3 = 300
        assert!((m.avg_latency_ms - 300.0).abs() < 1e-9);
        // 全成功 → healthy
        assert_eq!(classify(&m), ExpertHealth::Healthy);
    }
}
