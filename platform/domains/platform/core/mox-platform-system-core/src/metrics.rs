// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 可观测性：进程内指标聚合 + Prometheus 文本格式导出（I-04）
//!
//! 指标是「可运维、可治理」企业级系统的硬性要求（NFR-08）。
//! 此处用进程内原子计数器实现，零外部依赖；导出为 Prometheus 拉取格式，
//! 可由 node_exporter/Prometheus 直接抓取，亦可作为后续接入 OTel 的基石。
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics {
    pub requests: Arc<AtomicU64>,
    pub errors: Arc<AtomicU64>,
    /// 领域事件发布总数（与 EventBus 共享同一原子，确保计数一致）
    pub events_published: Arc<AtomicU64>,
    pub audit_records: Arc<AtomicU64>,
    pub notifications: Arc<AtomicU64>,
    pub active_members: Arc<AtomicU64>,
    pub http_latency_sum_ms: Arc<AtomicU64>,
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(AtomicU64::new(0)),
            errors: Arc::new(AtomicU64::new(0)),
            events_published: Arc::new(AtomicU64::new(0)),
            audit_records: Arc::new(AtomicU64::new(0)),
            notifications: Arc::new(AtomicU64::new(0)),
            active_members: Arc::new(AtomicU64::new(0)),
            http_latency_sum_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn inc_requests(&self) {
        self.requests.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_errors(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_events(&self) {
        self.events_published.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_audit(&self) {
        self.audit_records.fetch_add(1, Ordering::Relaxed);
    }
    pub fn inc_notifications(&self) {
        self.notifications.fetch_add(1, Ordering::Relaxed);
    }
    /// 以原子方式累加 HTTP 请求延迟（毫秒）
    pub fn add_latency_ms(&self, ms: u64) {
        self.http_latency_sum_ms.fetch_add(ms, Ordering::Relaxed);
    }
    pub fn set_active_members(&self, n: u64) {
        self.active_members.store(n, Ordering::Relaxed);
    }

    /// 导出为 Prometheus 文本格式
    pub fn render_prometheus(&self) -> String {
        let mut s = String::new();
        let req = self.requests.load(Ordering::Relaxed);
        let err = self.errors.load(Ordering::Relaxed);
        let ev = self.events_published.load(Ordering::Relaxed);
        let aud = self.audit_records.load(Ordering::Relaxed);
        let ntfs = self.notifications.load(Ordering::Relaxed);
        let active = self.active_members.load(Ordering::Relaxed);
        let lat = self.http_latency_sum_ms.load(Ordering::Relaxed);
        let avg = lat.checked_div(req).unwrap_or(0);

        s.push_str("# HELP mox_requests_total 总 HTTP 请求数\n");
        s.push_str("# TYPE mox_requests_total counter\n");
        s.push_str(&format!("mox_requests_total {}\n", req));

        s.push_str("# HELP mox_errors_total 4xx/5xx 响应数\n");
        s.push_str("# TYPE mox_errors_total counter\n");
        s.push_str(&format!("mox_errors_total {}\n", err));

        s.push_str("# HELP mox_events_published_total 领域事件发布总数\n");
        s.push_str("# TYPE mox_events_published_total counter\n");
        s.push_str(&format!("mox_events_published_total {}\n", ev));

        s.push_str("# HELP mox_audit_records_total 审计记录总数\n");
        s.push_str("# TYPE mox_audit_records_total counter\n");
        s.push_str(&format!("mox_audit_records_total {}\n", aud));

        s.push_str("# HELP mox_notifications_total 生成的通知总数\n");
        s.push_str("# TYPE mox_notifications_total counter\n");
        s.push_str(&format!("mox_notifications_total {}\n", ntfs));

        s.push_str("# HELP mox_active_members 当前活跃成员数（瞬时量）\n");
        s.push_str("# TYPE mox_active_members gauge\n");
        s.push_str(&format!("mox_active_members {}\n", active));

        s.push_str("# HELP mox_http_latency_avg_ms 平均 HTTP 延迟（毫秒）\n");
        s.push_str("# TYPE mox_http_latency_avg_ms gauge\n");
        s.push_str(&format!("mox_http_latency_avg_ms {}\n", avg));
        s
    }
}
