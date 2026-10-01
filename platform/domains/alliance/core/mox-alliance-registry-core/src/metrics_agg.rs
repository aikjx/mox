// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.

//! 专家指标聚合纯逻辑（对应 registry-proto 预留的
//! `ExpertDirectory::get_expert_metrics` / `get_platform_overview` 契约）。
//!
//! 背景：registry-proto 里这两个方法原为「未实现」默认占位。本模块提供两类纯计算：
//! 1. **调用成功率健康引擎** [`record_result`]/[`classify`]/[`summarize`]：单专家成败/
//!    延迟累加、跨专家健康度汇总（喂调用遥测后产出 healthy/degraded 分布）。
//! 2. **实例库存概览** [`inventory_platform_overview`]：直接从注册中心现存实例视图
//!    聚合出 proto `PlatformOverview`（总数/活跃/领域数），无需调用遥测即可上线。
//!
//! 零 IO、零新依赖；复用本 crate 已依赖的 `mox-alliance-registry-proto` 类型做映射，
//! 不修改 proto 既有字段。
//!
//! 健康阈值单一来源：[`HealthThresholds`]（默认成功率 0.80 / 失败占比 0.20）。
//! 无调用样本（invocations == 0）视为 Healthy（无失败证据），不误报。

use std::collections::BTreeSet;
use std::collections::BTreeMap;

use mox_alliance_registry_proto::types::{PlatformOverview as ProtoPlatformOverview, RegisteredInstance};
use serde::{Deserialize, Serialize};

/// 单专家运行指标（调用成功率健康引擎的内部累计态）。
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

/// 健康阈值单一来源（ADR 候选）。
///
/// 本应位于 config-core 作为可热更新配置；但 registry-core 当前不依赖 config-core
/// （本次铁律禁止改 Cargo.toml 加依赖边），故先在消费侧固化为可注入的纯结构。
/// 后续把本结构搬入 config-core 时，调用方只需把 [`HealthThresholds::DEFAULT`]
/// 换成配置引擎读出的值即可，函数签名不变。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HealthThresholds {
    /// 成功率低于此值即 Degraded。
    pub degraded_success_rate: f64,
    /// 失败占比高于此值即 Degraded。
    pub degraded_failure_ratio: f64,
}

impl HealthThresholds {
    /// 默认阈值：成功率 0.80 / 失败占比 0.20。
    pub const DEFAULT: HealthThresholds = HealthThresholds {
        degraded_success_rate: 0.80,
        degraded_failure_ratio: 0.20,
    };
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// 兼容旧名：默认成功率阈值（= [`HealthThresholds::DEFAULT`]）。
pub const DEGRADED_SUCCESS_RATE: f64 = HealthThresholds::DEFAULT.degraded_success_rate;
/// 兼容旧名：默认失败占比阈值。
pub const DEGRADED_FAILURE_RATIO: f64 = HealthThresholds::DEFAULT.degraded_failure_ratio;

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

/// 用给定阈值判定单个专家是否健康。
pub fn classify_with(m: &ExpertMetrics, th: &HealthThresholds) -> ExpertHealth {
    if m.invocations == 0 {
        return ExpertHealth::Healthy;
    }
    let failure_ratio = m.failures as f64 / m.invocations as f64;
    if m.success_rate < th.degraded_success_rate || failure_ratio > th.degraded_failure_ratio {
        ExpertHealth::Degraded
    } else {
        ExpertHealth::Healthy
    }
}

/// 用默认阈值判定单个专家是否健康。
pub fn classify(m: &ExpertMetrics) -> ExpertHealth {
    classify_with(m, &HealthThresholds::DEFAULT)
}

/// 平台概览（调用成功率健康引擎的纯数据投影，与 proto `PlatformOverview` 区分）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformOverview {
    pub total_experts: u32,
    pub healthy: u32,
    pub degraded: u32,
    /// 按状态名计数（"healthy" / "degraded"）。
    pub by_status: BTreeMap<String, u32>,
}

/// 用给定阈值把一批专家指标汇总成平台概览。
pub fn summarize_with<I: IntoIterator<Item = ExpertMetrics>>(
    iter: I,
    th: &HealthThresholds,
) -> PlatformOverview {
    let mut overview = PlatformOverview::default();
    for m in iter {
        overview.total_experts += 1;
        let status = match classify_with(&m, th) {
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

/// 用默认阈值汇总。
pub fn summarize<I: IntoIterator<Item = ExpertMetrics>>(iter: I) -> PlatformOverview {
    summarize_with(iter, &HealthThresholds::DEFAULT)
}

/// 从注册中心**现存实例视图**聚合出 proto `PlatformOverview`（`get_platform_overview`
/// 契约的可立即上线实现）。
///
/// 字段映射：
/// - `total_experts` = 现存实例数；
/// - `active_experts` = `InstanceStatus::Active` 数；
/// - `total_domains` = 去重后的非空 domain 数；
/// - `total_consultations` = 0（当前 svc 尚未记录逐次调用遥测，待接入后由
///   调用健康引擎的 `summarize` 填充）。
pub fn inventory_platform_overview(instances: &[RegisteredInstance]) -> ProtoPlatformOverview {
    let mut active = 0i32;
    let mut domains = BTreeSet::new();
    for i in instances {
        if i.status == mox_alliance_registry_proto::types::InstanceStatus::Active {
            active += 1;
        }
        if let Some(d) = &i.domain {
            domains.insert(d.clone());
        }
    }
    ProtoPlatformOverview {
        total_experts: instances.len() as i32,
        active_experts: active,
        total_domains: domains.len() as i32,
        total_consultations: 0,
    }
}

// ─── 单元测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn default_thresholds_are_80_percent_and_20_percent() {
        assert_eq!(HealthThresholds::DEFAULT.degraded_success_rate, 0.80);
        assert_eq!(HealthThresholds::DEFAULT.degraded_failure_ratio, 0.20);
        assert_eq!(HealthThresholds::DEFAULT, HealthThresholds::default());
    }

    #[test]
    fn thresholds_are_overridable() {
        // 把阈值放宽到 0.5：同样 2/3 成功率的专家不再 degraded
        let th = HealthThresholds {
            degraded_success_rate: 0.50,
            degraded_failure_ratio: 0.50,
        };
        let mut m = ExpertMetrics::new("x");
        record_result(&mut m, true, 10);
        record_result(&mut m, false, 10);
        record_result(&mut m, true, 10);
        assert_eq!(classify_with(&m, &th), ExpertHealth::Healthy);
        // 默认阈值下仍是 degraded（2/3 < 0.8）
        assert_eq!(classify(&m), ExpertHealth::Degraded);
    }

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
        assert!((m.avg_latency_ms - 200.0).abs() < 1e-9);
        record_result(&mut m, true, 500);
        assert!((m.avg_latency_ms - 300.0).abs() < 1e-9);
        assert_eq!(classify(&m), ExpertHealth::Healthy);
    }

    fn inst(id: &str, domain: Option<&str>, status: mox_alliance_registry_proto::types::InstanceStatus) -> RegisteredInstance {
        let now = Utc::now();
        RegisteredInstance {
            id: id.into(),
            name: id.into(),
            version: "1".into(),
            endpoint: "http://x".into(),
            health_check_url: None,
            capabilities: vec![],
            domain: domain.map(|s| s.to_string()),
            weight: 1.0,
            load_current: 0,
            load_capacity: 10,
            status,
            registered_at: now,
            last_heartbeat_at: now,
            lease_seconds: 15,
            metadata: Default::default(),
        }
    }

    #[test]
    fn inventory_overview_counts_active_and_distinct_domains() {
        use mox_alliance_registry_proto::types::InstanceStatus;
        let instances = vec![
            inst("a", Some("code"), InstanceStatus::Active),
            inst("b", Some("code"), InstanceStatus::Active),
            inst("c", Some("math"), InstanceStatus::Unhealthy),
            inst("d", None, InstanceStatus::Draining),
        ];
        let o = inventory_platform_overview(&instances);
        assert_eq!(o.total_experts, 4);
        assert_eq!(o.active_experts, 2);
        assert_eq!(o.total_domains, 2); // code + math（None 不计）
        assert_eq!(o.total_consultations, 0);
    }

    #[test]
    fn inventory_overview_empty() {
        let o = inventory_platform_overview(&[]);
        assert_eq!(o.total_experts, 0);
        assert_eq!(o.active_experts, 0);
        assert_eq!(o.total_domains, 0);
    }
}
