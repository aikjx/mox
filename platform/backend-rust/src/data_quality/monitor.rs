// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! 数据质量监控
//!
//! 核心能力：
//! - 定时质量检查调度
//! - 质量趋势追踪
//! - 质量告警（阈值触发）
//! - 质量报告生成
//! - SLA 合规监控

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

use super::rules::{QualityRuleEngine, RuleData, AssetQualityScore};

/// 质量告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityAlert {
    pub id: String,
    pub asset_id: String,
    pub rule_id: Option<String>,
    pub dimension: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub current_score: f64,
    pub threshold: f64,
    pub trend: AlertTrend,
    pub created_at: String,
    pub acknowledged: bool,
    pub acknowledged_at: Option<String>,
}

/// 告警严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertSeverity {
    Critical,
    Warning,
    Info,
}

/// 告警趋势
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AlertTrend {
    Degrading,
    Improving,
    Stable,
    New,
}

/// 质量报告
#[derive(Debug, Clone, Serialize)]
pub struct QualityReport {
    pub id: String,
    pub generated_at: String,
    pub period_start: String,
    pub period_end: String,
    pub total_assets: usize,
    pub avg_quality_score: f64,
    pub assets_by_score_range: HashMap<String, usize>,
    pub dimension_avg_scores: HashMap<String, f64>,
    pub total_alerts: usize,
    pub critical_alerts: usize,
    pub warning_alerts: usize,
    pub top_degraded_assets: Vec<AssetQualitySummary>,
    pub top_improved_assets: Vec<AssetQualitySummary>,
    pub sla_compliance: f64,
}

/// 资产质量摘要
#[derive(Debug, Clone, Serialize)]
pub struct AssetQualitySummary {
    pub asset_id: String,
    pub asset_name: String,
    pub current_score: f64,
    pub previous_score: f64,
    pub change: f64,
    pub dimension: String,
}

/// 质量历史记录
#[derive(Debug, Clone, Serialize)]
struct QualityHistoryEntry {
    asset_id: String,
    score: f64,
    dimension_scores: HashMap<String, f64>,
    timestamp: String,
}

/// 质量监控器
pub struct QualityMonitor {
    rule_engine: Arc<QualityRuleEngine>,
    history: RwLock<Vec<QualityHistoryEntry>>,
    alerts: RwLock<Vec<QualityAlert>>,
    alert_thresholds: RwLock<HashMap<String, f64>>,
    sla_target: RwLock<f64>,
    total_checks: AtomicU64,
    total_alerts_generated: AtomicU64,
}

impl QualityMonitor {
    /// 创建质量监控器
    pub fn new(rule_engine: Arc<QualityRuleEngine>) -> Self {
        let mut thresholds = HashMap::new();
        thresholds.insert("overall".to_string(), 80.0);
        thresholds.insert("critical".to_string(), 90.0);

        Self {
            rule_engine,
            history: RwLock::new(Vec::new()),
            alerts: RwLock::new(Vec::new()),
            alert_thresholds: RwLock::new(thresholds),
            sla_target: RwLock::new(95.0),
            total_checks: AtomicU64::new(0),
            total_alerts_generated: AtomicU64::new(0),
        }
    }

    /// 执行资产质量检查
    pub fn check_asset(&self, asset_id: &str, data: &RuleData) -> AssetQualityScore {
        self.total_checks.fetch_add(1, Ordering::Relaxed);

        let score = self.rule_engine.calculate_asset_score(asset_id, data);

        // 记录历史
        let entry = QualityHistoryEntry {
            asset_id: asset_id.to_string(),
            score: score.overall_score,
            dimension_scores: score.dimension_scores.iter()
                .map(|(k, v)| (format!("{:?}", k), *v))
                .collect(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        if let Ok(mut history) = self.history.write() {
            history.push(entry);
            // 保留最近 10000 条
            if history.len() > 10000 {
                let drain_count = history.len() - 10000;
                history.drain(0..drain_count);
            }
        }

        // 检查告警
        self.check_alerts(&score);

        score
    }

    /// 检查并生成告警
    fn check_alerts(&self, score: &AssetQualityScore) {
        let thresholds = self.alert_thresholds.read().unwrap();
        let overall_threshold = thresholds.get("overall").copied().unwrap_or(80.0);

        // 获取历史分数用于趋势判断
        let previous_score = self.get_latest_history_score(&score.asset_id);

        if score.overall_score < overall_threshold {
            let trend = if let Some(prev) = previous_score {
                if score.overall_score < prev { AlertTrend::Degrading }
                else if score.overall_score > prev { AlertTrend::Improving }
                else { AlertTrend::Stable }
            } else {
                AlertTrend::New
            };

            let severity = if score.overall_score < 60.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };

            let alert = QualityAlert {
                id: Uuid::new_v4().to_string(),
                asset_id: score.asset_id.clone(),
                rule_id: None,
                dimension: "overall".to_string(),
                severity,
                message: format!(
                    "资产 {} 质量分数 {:.1}% 低于阈值 {:.1}%",
                    score.asset_id, score.overall_score, overall_threshold
                ),
                current_score: score.overall_score,
                threshold: overall_threshold,
                trend,
                created_at: chrono::Utc::now().to_rfc3339(),
                acknowledged: false,
                acknowledged_at: None,
            };

            if let Ok(mut alerts) = self.alerts.write() {
                alerts.push(alert);
            }
            self.total_alerts_generated.fetch_add(1, Ordering::Relaxed);
        }

        // 检查各维度
        for (dimension, dim_score) in &score.dimension_scores {
            let dim_key = format!("{:?}", dimension);
            let dim_threshold = thresholds.get(&dim_key).copied().unwrap_or(85.0);
            if *dim_score < dim_threshold {
                let alert = QualityAlert {
                    id: Uuid::new_v4().to_string(),
                    asset_id: score.asset_id.clone(),
                    rule_id: None,
                    dimension: dim_key.clone(),
                    severity: AlertSeverity::Warning,
                    message: format!(
                        "资产 {} 维度 {} 分数 {:.1}% 低于阈值 {:.1}%",
                        score.asset_id, dim_key, dim_score, dim_threshold
                    ),
                    current_score: *dim_score,
                    threshold: dim_threshold,
                    trend: AlertTrend::Stable,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    acknowledged: false,
                    acknowledged_at: None,
                };
                if let Ok(mut alerts) = self.alerts.write() {
                    alerts.push(alert);
                }
            }
        }
    }

    /// 获取最新历史分数
    fn get_latest_history_score(&self, asset_id: &str) -> Option<f64> {
        let history = self.history.read().ok()?;
        history.iter()
            .rev()
            .find(|e| e.asset_id == asset_id)
            .map(|e| e.score)
    }

    /// 获取资产质量趋势
    pub fn get_asset_trend(&self, asset_id: &str, limit: usize) -> Vec<(String, f64)> {
        let history = self.history.read().unwrap();
        history.iter()
            .filter(|e| e.asset_id == asset_id)
            .rev()
            .take(limit)
            .map(|e| (e.timestamp.clone(), e.score))
            .collect()
    }

    /// 确认告警
    pub fn acknowledge_alert(&self, alert_id: &str) -> bool {
        if let Ok(mut alerts) = self.alerts.write() {
            if let Some(alert) = alerts.iter_mut().find(|a| a.id == alert_id) {
                alert.acknowledged = true;
                alert.acknowledged_at = Some(chrono::Utc::now().to_rfc3339());
                return true;
            }
        }
        false
    }

    /// 获取所有告警
    pub fn get_alerts(&self, acknowledged: Option<bool>) -> Vec<QualityAlert> {
        let alerts = self.alerts.read().unwrap();
        alerts.iter()
            .filter(|a| acknowledged.map_or(true, |ack| a.acknowledged == ack))
            .cloned()
            .collect()
    }

    /// 设置告警阈值
    pub fn set_alert_threshold(&self, dimension: &str, threshold: f64) {
        if let Ok(mut thresholds) = self.alert_thresholds.write() {
            thresholds.insert(dimension.to_string(), threshold);
        }
    }

    /// 设置 SLA 目标
    pub fn set_sla_target(&self, target: f64) {
        if let Ok(mut sla) = self.sla_target.write() {
            *sla = target;
        }
    }

    /// 生成质量报告
    pub fn generate_report(&self, period_hours: u64) -> QualityReport {
        let history = self.history.read().unwrap();
        let alerts = self.alerts.read().unwrap();
        let sla_target = *self.sla_target.read().unwrap();

        let now = chrono::Utc::now();
        let period_start = now - chrono::Duration::hours(period_hours as i64);

        let period_history: Vec<_> = history.iter()
            .filter(|e| {
                let ts = chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(now);
                ts >= period_start
            })
            .collect();

        // 按资产分组取最新分数
        let mut latest_scores: HashMap<String, f64> = HashMap::new();
        for entry in &period_history {
            latest_scores.insert(entry.asset_id.clone(), entry.score);
        }

        let total_assets = latest_scores.len();
        let avg_score = if total_assets > 0 {
            latest_scores.values().sum::<f64>() / total_assets as f64
        } else {
            100.0
        };

        // 分数区间分布
        let mut score_ranges = HashMap::new();
        for score in latest_scores.values() {
            let range = if *score >= 95.0 { "95-100" }
                else if *score >= 85.0 { "85-95" }
                else if *score >= 70.0 { "70-85" }
                else if *score >= 50.0 { "50-70" }
                else { "0-50" };
            *score_ranges.entry(range.to_string()).or_insert(0) += 1;
        }

        // SLA 合规率
        let sla_compliant = latest_scores.values().filter(|s| **s >= sla_target).count();
        let sla_compliance = if total_assets > 0 {
            sla_compliant as f64 / total_assets as f64 * 100.0
        } else {
            100.0
        };

        // 周期内告警
        let period_alerts: Vec<_> = alerts.iter()
            .filter(|a| {
                let ts = chrono::DateTime::parse_from_rfc3339(&a.created_at)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or(now);
                ts >= period_start
            })
            .collect();

        QualityReport {
            id: Uuid::new_v4().to_string(),
            generated_at: now.to_rfc3339(),
            period_start: period_start.to_rfc3339(),
            period_end: now.to_rfc3339(),
            total_assets,
            avg_quality_score: avg_score,
            assets_by_score_range: score_ranges,
            dimension_avg_scores: HashMap::new(),
            total_alerts: period_alerts.len(),
            critical_alerts: period_alerts.iter().filter(|a| a.severity == AlertSeverity::Critical).count(),
            warning_alerts: period_alerts.iter().filter(|a| a.severity == AlertSeverity::Warning).count(),
            top_degraded_assets: vec![],
            top_improved_assets: vec![],
            sla_compliance,
        }
    }

    /// 获取统计
    pub fn stats(&self) -> MonitorStats {
        let history = self.history.read().unwrap();
        let alerts = self.alerts.read().unwrap();
        MonitorStats {
            total_checks: self.total_checks.load(Ordering::Relaxed),
            total_history_entries: history.len(),
            total_alerts: alerts.len(),
            unacknowledged_alerts: alerts.iter().filter(|a| !a.acknowledged).count(),
            total_alerts_generated: self.total_alerts_generated.load(Ordering::Relaxed),
            sla_target: *self.sla_target.read().unwrap(),
            alert_thresholds: self.alert_thresholds.read().unwrap().clone(),
        }
    }
}

/// 监控器统计
#[derive(Debug, Clone, Serialize)]
pub struct MonitorStats {
    pub total_checks: u64,
    pub total_history_entries: usize,
    pub total_alerts: usize,
    pub unacknowledged_alerts: usize,
    pub total_alerts_generated: u64,
    pub sla_target: f64,
    pub alert_thresholds: HashMap<String, f64>,
}
