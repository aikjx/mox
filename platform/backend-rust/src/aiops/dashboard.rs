//! AIOps 仪表盘
//!
//! 核心能力：
//! - 异常汇总与分类
//! - 根因分析报告整合
//! - 预测趋势可视化数据
//! - 智能建议生成
//! - SLA 合规报告
//! - 运维健康度评分

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

use super::{AnomalyEvent, AnomalySeverity};
use super::root_cause::RootCauseResult;
use super::predictive_scaler::ScalingRecommendation;

/// 智能建议
#[derive(Debug, Clone, Serialize)]
pub struct IntelligentSuggestion {
    pub id: String,
    pub category: SuggestionCategory,
    pub title: String,
    pub description: String,
    pub priority: SuggestionPriority,
    pub estimated_impact: String,
    pub effort: String,
    pub related_anomalies: Vec<String>,
    pub related_root_causes: Vec<String>,
    pub action_items: Vec<String>,
    pub created_at: String,
    pub acknowledged: bool,
}

/// 建议类别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SuggestionCategory {
    Performance,
    Reliability,
    Cost,
    Security,
    Scalability,
    Operational,
}

/// 建议优先级
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SuggestionPriority {
    Immediate,
    High,
    Medium,
    Low,
}

/// AIOps 报告
#[derive(Debug, Clone, Serialize)]
pub struct AiopsReport {
    pub id: String,
    pub generated_at: String,
    pub period_start: String,
    pub period_end: String,
    pub overall_health_score: f64,
    pub anomaly_summary: AnomalySummary,
    pub root_cause_summary: RootCauseSummary,
    pub scaling_summary: ScalingSummary,
    pub sla_compliance: SlaCompliance,
    pub top_suggestions: Vec<IntelligentSuggestion>,
    pub trend_data: TrendData,
}

/// 异常汇总
#[derive(Debug, Clone, Serialize)]
pub struct AnomalySummary {
    pub total_anomalies: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub unresolved: usize,
    pub by_metric: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub mttr_seconds: f64,
}

/// 根因汇总
#[derive(Debug, Clone, Serialize)]
pub struct RootCauseSummary {
    pub total_analyses: usize,
    pub avg_confidence: f64,
    pub top_root_causes: Vec<(String, f64)>,
    pub avg_analysis_duration_ms: f64,
}

/// 扩缩容汇总
#[derive(Debug, Clone, Serialize)]
pub struct ScalingSummary {
    pub total_recommendations: usize,
    pub scale_ups: usize,
    pub scale_downs: usize,
    pub executed: usize,
    pub avg_confidence: f64,
}

/// SLA 合规
#[derive(Debug, Clone, Serialize)]
pub struct SlaCompliance {
    pub target: f64,
    pub actual: f64,
    pub compliant: bool,
    pub downtime_seconds: u64,
    pub incidents: usize,
}

/// 趋势数据
#[derive(Debug, Clone, Serialize)]
pub struct TrendData {
    pub anomaly_trend: Vec<(String, usize)>,
    pub latency_trend: Vec<(String, f64)>,
    pub error_rate_trend: Vec<(String, f64)>,
    pub resource_utilization_trend: Vec<(String, f64)>,
}

/// AIOps 仪表盘
pub struct AiopsDashboard {
    anomalies: RwLock<Vec<AnomalyEvent>>,
    root_cause_results: RwLock<Vec<RootCauseResult>>,
    scaling_recommendations: RwLock<Vec<ScalingRecommendation>>,
    suggestions: RwLock<Vec<IntelligentSuggestion>>,
    sla_target: RwLock<f64>,
    total_reports_generated: std::sync::atomic::AtomicU64,
}

impl AiopsDashboard {
    /// 创建 AIOps 仪表盘
    pub fn new() -> Self {
        Self {
            anomalies: RwLock::new(Vec::new()),
            root_cause_results: RwLock::new(Vec::new()),
            scaling_recommendations: RwLock::new(Vec::new()),
            suggestions: RwLock::new(Vec::new()),
            sla_target: RwLock::new(99.9),
            total_reports_generated: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// 添加异常事件
    pub fn add_anomaly(&self, anomaly: AnomalyEvent) {
        self.anomalies.write().unwrap().push(anomaly);
    }

    /// 添加根因分析结果
    pub fn add_root_cause_result(&self, result: RootCauseResult) {
        self.root_cause_results.write().unwrap().push(result);
    }

    /// 添加扩缩容建议
    pub fn add_scaling_recommendation(&self, recommendation: ScalingRecommendation) {
        self.scaling_recommendations.write().unwrap().push(recommendation);
    }

    /// 生成智能建议
    pub fn generate_suggestions(&self) -> Vec<IntelligentSuggestion> {
        let mut suggestions = Vec::new();

        let anomalies = self.anomalies.read().unwrap();
        let critical_anomalies: Vec<_> = anomalies.iter()
            .filter(|a| a.severity == AnomalySeverity::Critical && !a.resolved)
            .collect();

        // 基于关键异常生成建议
        for anomaly in &critical_anomalies {
            let suggestion = IntelligentSuggestion {
                id: Uuid::new_v4().to_string(),
                category: SuggestionCategory::Reliability,
                title: format!("立即处理关键异常: {}", anomaly.metric_name),
                description: anomaly.description.clone(),
                priority: SuggestionPriority::Immediate,
                estimated_impact: "高 - 可能影响服务可用性".to_string(),
                effort: "中".to_string(),
                related_anomalies: vec![anomaly.id.clone()],
                related_root_causes: vec![],
                action_items: vec![
                    format!("查看指标 {} 的详细趋势", anomaly.metric_name),
                    "检查相关服务的日志和事件".to_string(),
                    "执行根因分析定位问题源头".to_string(),
                    "实施修复并验证".to_string(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                acknowledged: false,
            };
            suggestions.push(suggestion);
        }

        // 基于扩缩容建议生成成本优化建议
        let scaling = self.scaling_recommendations.read().unwrap();
        let scale_downs: Vec<_> = scaling.iter()
            .filter(|r| r.decision == super::predictive_scaler::ScalingDecision::ScaleDown && !r.executed)
            .collect();

        if !scale_downs.is_empty() {
            suggestions.push(IntelligentSuggestion {
                id: Uuid::new_v4().to_string(),
                category: SuggestionCategory::Cost,
                title: format!("优化资源配置: {} 个资源建议缩容", scale_downs.len()),
                description: "基于预测性分析，部分资源利用率持续偏低，建议缩容以降低成本".to_string(),
                priority: SuggestionPriority::Medium,
                estimated_impact: format!("预计节省 {} 个资源实例", scale_downs.len()),
                effort: "低".to_string(),
                related_anomalies: vec![],
                related_root_causes: vec![],
                action_items: vec![
                    "审查缩容建议列表".to_string(),
                    "验证缩容不会影响服务质量".to_string(),
                    "在低峰期执行缩容".to_string(),
                    "监控缩容后的服务表现".to_string(),
                ],
                created_at: chrono::Utc::now().to_rfc3339(),
                acknowledged: false,
            });
        }

        // 基于根因分析生成运维建议
        let root_causes = self.root_cause_results.read().unwrap();
        if !root_causes.is_empty() {
            let avg_confidence: f64 = root_causes.iter().map(|r| r.confidence).sum::<f64>() / root_causes.len() as f64;
            if avg_confidence > 0.7 {
                suggestions.push(IntelligentSuggestion {
                    id: Uuid::new_v4().to_string(),
                    category: SuggestionCategory::Operational,
                    title: "加强故障根因定位能力".to_string(),
                    description: format!("根因分析平均置信度 {:.0}%，建议完善依赖图和监控以提高定位精度", avg_confidence * 100.0),
                    priority: SuggestionPriority::Low,
                    estimated_impact: "提高 MTTR 改善效率".to_string(),
                    effort: "中".to_string(),
                    related_anomalies: vec![],
                    related_root_causes: root_causes.iter().take(3).map(|r| r.id.clone()).collect(),
                    action_items: vec![
                        "完善服务依赖图".to_string(),
                        "增加分布式追踪覆盖".to_string(),
                        "优化根因分析算法参数".to_string(),
                    ],
                    created_at: chrono::Utc::now().to_rfc3339(),
                    acknowledged: false,
                });
            }
        }

        // 保存建议
        self.suggestions.write().unwrap().extend(suggestions.clone());
        suggestions
    }

    /// 生成 AIOps 报告
    pub fn generate_report(&self, period_hours: u64) -> AiopsReport {
        self.total_reports_generated.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let now = chrono::Utc::now();
        let period_start = now - chrono::Duration::hours(period_hours as i64);

        let anomalies = self.anomalies.read().unwrap();
        let period_anomalies: Vec<_> = anomalies.iter()
            .filter(|a| {
                chrono::DateTime::parse_from_rfc3339(&a.timestamp)
                    .map(|dt| dt.with_timezone(&chrono::Utc) >= period_start)
                    .unwrap_or(false)
            })
            .collect();

        // 异常汇总
        let anomaly_summary = AnomalySummary {
            total_anomalies: period_anomalies.len(),
            critical: period_anomalies.iter().filter(|a| a.severity == AnomalySeverity::Critical).count(),
            high: period_anomalies.iter().filter(|a| a.severity == AnomalySeverity::High).count(),
            medium: period_anomalies.iter().filter(|a| a.severity == AnomalySeverity::Medium).count(),
            low: period_anomalies.iter().filter(|a| a.severity == AnomalySeverity::Low).count(),
            unresolved: period_anomalies.iter().filter(|a| !a.resolved).count(),
            by_metric: period_anomalies.iter().fold(HashMap::new(), |mut acc, a| {
                *acc.entry(a.metric_name.clone()).or_insert(0) += 1;
                acc
            }),
            by_type: period_anomalies.iter().fold(HashMap::new(), |mut acc, a| {
                *acc.entry(format!("{:?}", a.anomaly_type)).or_insert(0) += 1;
                acc
            }),
            mttr_seconds: 0.0, // 简化
        };

        // 根因汇总
        let root_causes = self.root_cause_results.read().unwrap();
        let root_cause_summary = RootCauseSummary {
            total_analyses: root_causes.len(),
            avg_confidence: if !root_causes.is_empty() {
                root_causes.iter().map(|r| r.confidence).sum::<f64>() / root_causes.len() as f64
            } else { 0.0 },
            top_root_causes: root_causes.iter()
                .flat_map(|r| r.root_causes.iter().map(|rc| (rc.node_name.clone(), rc.score)))
                .take(10)
                .collect(),
            avg_analysis_duration_ms: if !root_causes.is_empty() {
                root_causes.iter().map(|r| r.analysis_duration_ms as f64).sum::<f64>() / root_causes.len() as f64
            } else { 0.0 },
        };

        // 扩缩容汇总
        let scaling = self.scaling_recommendations.read().unwrap();
        let scaling_summary = ScalingSummary {
            total_recommendations: scaling.len(),
            scale_ups: scaling.iter().filter(|r| r.decision == super::predictive_scaler::ScalingDecision::ScaleUp).count(),
            scale_downs: scaling.iter().filter(|r| r.decision == super::predictive_scaler::ScalingDecision::ScaleDown).count(),
            executed: scaling.iter().filter(|r| r.executed).count(),
            avg_confidence: if !scaling.is_empty() {
                scaling.iter().map(|r| r.confidence).sum::<f64>() / scaling.len() as f64
            } else { 0.0 },
        };

        // 整体健康分数
        let health_score = if anomaly_summary.total_anomalies > 0 {
            let critical_weight = anomaly_summary.critical as f64 * 10.0;
            let high_weight = anomaly_summary.high as f64 * 5.0;
            let medium_weight = anomaly_summary.medium as f64 * 2.0;
            let penalty = (critical_weight + high_weight + medium_weight).min(80.0);
            (100.0 - penalty).max(20.0)
        } else {
            95.0
        };

        // SLA 合规
        let sla_target = *self.sla_target.read().unwrap();
        let sla_compliance = SlaCompliance {
            target: sla_target,
            actual: health_score,
            compliant: health_score >= sla_target,
            downtime_seconds: 0,
            incidents: anomaly_summary.critical,
        };

        // 生成建议
        let top_suggestions = self.generate_suggestions();

        AiopsReport {
            id: Uuid::new_v4().to_string(),
            generated_at: now.to_rfc3339(),
            period_start: period_start.to_rfc3339(),
            period_end: now.to_rfc3339(),
            overall_health_score: health_score,
            anomaly_summary,
            root_cause_summary,
            scaling_summary,
            sla_compliance,
            top_suggestions: top_suggestions.into_iter().take(5).collect(),
            trend_data: TrendData {
                anomaly_trend: vec![],
                latency_trend: vec![],
                error_rate_trend: vec![],
                resource_utilization_trend: vec![],
            },
        }
    }

    /// 确认建议
    pub fn acknowledge_suggestion(&self, suggestion_id: &str) -> bool {
        if let Some(mut suggestions) = self.suggestions.write().ok() {
            if let Some(s) = suggestions.iter_mut().find(|s| s.id == suggestion_id) {
                s.acknowledged = true;
                return true;
            }
        }
        false
    }

    /// 设置 SLA 目标
    pub fn set_sla_target(&self, target: f64) {
        *self.sla_target.write().unwrap() = target;
    }

    /// 获取建议列表
    pub fn get_suggestions(&self, category: Option<SuggestionCategory>, acknowledged: Option<bool>) -> Vec<IntelligentSuggestion> {
        self.suggestions.read().unwrap()
            .iter()
            .filter(|s| category.map_or(true, |c| s.category == c))
            .filter(|s| acknowledged.map_or(true, |a| s.acknowledged == a))
            .cloned()
            .collect()
    }

    /// 获取统计
    pub fn stats(&self) -> DashboardStats {
        DashboardStats {
            total_anomalies: self.anomalies.read().unwrap().len(),
            total_root_cause_analyses: self.root_cause_results.read().unwrap().len(),
            total_scaling_recommendations: self.scaling_recommendations.read().unwrap().len(),
            total_suggestions: self.suggestions.read().unwrap().len(),
            acknowledged_suggestions: self.suggestions.read().unwrap().iter().filter(|s| s.acknowledged).count(),
            total_reports_generated: self.total_reports_generated.load(std::sync::atomic::Ordering::Relaxed),
            sla_target: *self.sla_target.read().unwrap(),
        }
    }
}

impl Default for AiopsDashboard {
    fn default() -> Self {
        Self::new()
    }
}

/// 仪表盘统计
#[derive(Debug, Clone, Serialize)]
pub struct DashboardStats {
    pub total_anomalies: usize,
    pub total_root_cause_analyses: usize,
    pub total_scaling_recommendations: usize,
    pub total_suggestions: usize,
    pub acknowledged_suggestions: usize,
    pub total_reports_generated: u64,
    pub sla_target: f64,
}
