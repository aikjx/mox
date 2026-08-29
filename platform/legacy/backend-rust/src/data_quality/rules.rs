// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 数据质量规则引擎
//!
//! 六大质量维度：
//! - Completeness（完整性）：非空率、必填字段
//! - Accuracy（准确性）：值范围、格式校验、参照完整性
//! - Consistency（一致性）：跨数据集一致、枚举值一致
//! - Timeliness（时效性）：数据新鲜度、更新延迟
//! - Uniqueness（唯一性）：主键唯一、去重率
//! - Validity（有效性）：数据类型、业务规则

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// 质量维度
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QualityDimension {
    Completeness,
    Accuracy,
    Consistency,
    Timeliness,
    Uniqueness,
    Validity,
}

impl QualityDimension {
    pub fn all() -> [QualityDimension; 6] {
        [
            QualityDimension::Completeness,
            QualityDimension::Accuracy,
            QualityDimension::Consistency,
            QualityDimension::Timeliness,
            QualityDimension::Uniqueness,
            QualityDimension::Validity,
        ]
    }

    pub fn description(&self) -> &'static str {
        match self {
            QualityDimension::Completeness => "数据完整性：非空率、必填字段覆盖率",
            QualityDimension::Accuracy => "数据准确性：值范围、格式校验、参照完整性",
            QualityDimension::Consistency => "数据一致性：跨数据集一致、枚举值一致",
            QualityDimension::Timeliness => "数据时效性：数据新鲜度、更新延迟",
            QualityDimension::Uniqueness => "数据唯一性：主键唯一、去重率",
            QualityDimension::Validity => "数据有效性：数据类型、业务规则",
        }
    }
}

/// 规则严重级别
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum RuleSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// 质量规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRule {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dimension: QualityDimension,
    pub severity: RuleSeverity,
    pub asset_id: String,
    pub column_name: Option<String>,
    pub rule_type: RuleType,
    pub threshold: f64,
    pub enabled: bool,
    pub created_at: String,
    pub last_run_at: Option<String>,
    pub last_result: Option<RuleResult>,
}

/// 规则类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleType {
    NotNull,
    NotNullPercentage { min_percentage: f64 },
    Range { min: f64, max: f64 },
    Regex { pattern: String },
    Enum { allowed_values: Vec<String> },
    Unique,
    UniquePercentage { min_percentage: f64 },
    Freshness { max_age_seconds: u64 },
    ReferentialIntegrity { reference_asset: String, reference_column: String },
    Custom { expression: String },
    RowCount { min: u64, max: u64 },
    DataTypeMatch { expected_type: String },
}

/// 规则执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleResult {
    pub rule_id: String,
    pub rule_name: String,
    pub dimension: QualityDimension,
    pub severity: RuleSeverity,
    pub passed: bool,
    pub score: f64,
    pub threshold: f64,
    pub actual_value: f64,
    pub total_records: u64,
    pub failed_records: u64,
    pub sample_failures: Vec<String>,
    pub executed_at: String,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// 质量规则引擎
pub struct QualityRuleEngine {
    rules: HashMap<String, QualityRule>,
    asset_rules: HashMap<String, Vec<String>>,
    total_executions: AtomicU64,
    total_passes: AtomicU64,
    total_failures: AtomicU64,
}

impl QualityRuleEngine {
    /// 创建规则引擎
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            asset_rules: HashMap::new(),
            total_executions: AtomicU64::new(0),
            total_passes: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
        }
    }

    /// 注册规则
    pub fn register(&mut self, mut rule: QualityRule) -> String {
        if rule.id.is_empty() {
            rule.id = Uuid::new_v4().to_string();
        }
        if rule.created_at.is_empty() {
            rule.created_at = chrono::Utc::now().to_rfc3339();
        }
        let id = rule.id.clone();
        let asset_id = rule.asset_id.clone();
        self.rules.insert(id.clone(), rule);
        self.asset_rules.entry(asset_id).or_default().push(id.clone());
        id
    }

    /// 获取规则
    pub fn get(&self, id: &str) -> Option<&QualityRule> {
        self.rules.get(id)
    }

    /// 获取资产的所有规则
    pub fn get_asset_rules(&self, asset_id: &str) -> Vec<&QualityRule> {
        self.asset_rules.get(asset_id)
            .map(|ids| ids.iter().filter_map(|id| self.rules.get(id)).collect())
            .unwrap_or_default()
    }

    /// 按维度获取规则
    pub fn get_rules_by_dimension(&self, dimension: QualityDimension) -> Vec<&QualityRule> {
        self.rules.values().filter(|r| r.dimension == dimension).collect()
    }

    /// 执行单条规则
    pub fn execute_rule(&self, rule_id: &str, data: &RuleData) -> RuleResult {
        let start = std::time::Instant::now();
        let rule = match self.rules.get(rule_id) {
            Some(r) => r.clone(),
            None => return RuleResult {
                rule_id: rule_id.to_string(),
                rule_name: "unknown".to_string(),
                dimension: QualityDimension::Validity,
                severity: RuleSeverity::High,
                passed: false,
                score: 0.0,
                threshold: 0.0,
                actual_value: 0.0,
                total_records: 0,
                failed_records: 0,
                sample_failures: vec![],
                executed_at: chrono::Utc::now().to_rfc3339(),
                duration_ms: 0,
                error: Some("规则不存在".to_string()),
            },
        };

        self.total_executions.fetch_add(1, Ordering::Relaxed);

        let (passed, score, actual, failed, samples) = self.evaluate(&rule, data);

        if passed {
            self.total_passes.fetch_add(1, Ordering::Relaxed);
        } else {
            self.total_failures.fetch_add(1, Ordering::Relaxed);
        }

        let result = RuleResult {
            rule_id: rule.id.clone(),
            rule_name: rule.name.clone(),
            dimension: rule.dimension,
            severity: rule.severity,
            passed,
            score,
            threshold: rule.threshold,
            actual_value: actual,
            total_records: data.total_records,
            failed_records: failed,
            sample_failures: samples,
            executed_at: chrono::Utc::now().to_rfc3339(),
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
        };

        result
    }

    /// 执行资产的所有规则
    pub fn execute_asset_rules(&self, asset_id: &str, data: &RuleData) -> Vec<RuleResult> {
        let rule_ids = self.asset_rules.get(asset_id).cloned().unwrap_or_default();
        rule_ids.iter().map(|id| self.execute_rule(id, data)).collect()
    }

    /// 计算资产的综合质量分数
    pub fn calculate_asset_score(&self, asset_id: &str, data: &RuleData) -> AssetQualityScore {
        let results = self.execute_asset_rules(asset_id, data);
        let mut dimension_scores: HashMap<QualityDimension, Vec<f64>> = HashMap::new();

        for result in &results {
            dimension_scores.entry(result.dimension).or_default().push(result.score);
        }

        let dimension_avg: HashMap<QualityDimension, f64> = dimension_scores.iter()
            .map(|(dim, scores)| (*dim, scores.iter().sum::<f64>() / scores.len() as f64))
            .collect();

        let overall_score = if !dimension_avg.is_empty() {
            dimension_avg.values().sum::<f64>() / dimension_avg.len() as f64
        } else {
            100.0
        };

        AssetQualityScore {
            asset_id: asset_id.to_string(),
            overall_score,
            dimension_scores: dimension_avg,
            total_rules: results.len(),
            passed_rules: results.iter().filter(|r| r.passed).count(),
            failed_rules: results.iter().filter(|r| !r.passed).count(),
            critical_failures: results.iter().filter(|r| !r.passed && r.severity == RuleSeverity::Critical).count(),
            results,
            calculated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn evaluate(&self, rule: &QualityRule, data: &RuleData) -> (bool, f64, f64, u64, Vec<String>) {
        match &rule.rule_type {
            RuleType::NotNull => {
                let column = rule.column_name.as_deref().unwrap_or("");
                let non_null = data.column_values.get(column)
                    .map(|vals| vals.iter().filter(|v| !v.is_null).count())
                    .unwrap_or(0);
                let total = data.total_records;
                let pct = if total > 0 { non_null as f64 / total as f64 * 100.0 } else { 100.0 };
                let passed = pct >= rule.threshold;
                (passed, pct, pct, total - non_null as u64, vec![])
            }
            RuleType::Unique => {
                let column = rule.column_name.as_deref().unwrap_or("");
                let values = data.column_values.get(column).cloned().unwrap_or_default();
                let unique: std::collections::HashSet<_> = values.iter().map(|v| &v.value).collect();
                let dup_count = values.len() - unique.len();
                let pct = if !values.is_empty() { unique.len() as f64 / values.len() as f64 * 100.0 } else { 100.0 };
                let passed = dup_count == 0;
                (passed, pct, pct, dup_count as u64, vec![])
            }
            RuleType::Range { min, max } => {
                let column = rule.column_name.as_deref().unwrap_or("");
                let values = data.column_values.get(column).cloned().unwrap_or_default();
                let mut failed = 0;
                let mut samples = Vec::new();
                for v in &values {
                    if let Ok(num) = v.value.parse::<f64>() {
                        if num < *min || num > *max {
                            failed += 1;
                            if samples.len() < 5 { samples.push(v.value.clone()); }
                        }
                    } else {
                        failed += 1;
                    }
                }
                let pct = if !values.is_empty() { (values.len() - failed) as f64 / values.len() as f64 * 100.0 } else { 100.0 };
                let passed = pct >= rule.threshold;
                (passed, pct, pct, failed as u64, samples)
            }
            RuleType::Regex { pattern } => {
                let column = rule.column_name.as_deref().unwrap_or("");
                let values = data.column_values.get(column).cloned().unwrap_or_default();
                let regex = regex::Regex::new(pattern).unwrap_or(regex::Regex::new(r".*").unwrap());
                let mut failed = 0;
                let mut samples = Vec::new();
                for v in &values {
                    if !regex.is_match(&v.value) {
                        failed += 1;
                        if samples.len() < 5 { samples.push(v.value.clone()); }
                    }
                }
                let pct = if !values.is_empty() { (values.len() - failed) as f64 / values.len() as f64 * 100.0 } else { 100.0 };
                let passed = pct >= rule.threshold;
                (passed, pct, pct, failed as u64, samples)
            }
            RuleType::Freshness { max_age_seconds } => {
                let age = data.last_updated_age_seconds.unwrap_or(0);
                let pct = if *max_age_seconds > 0 {
                    (1.0 - age.min(*max_age_seconds) as f64 / *max_age_seconds as f64) * 100.0
                } else { 100.0 };
                let passed = age <= *max_age_seconds;
                (passed, pct.max(0.0), age as f64, if passed { 0 } else { 1 }, vec![])
            }
            RuleType::RowCount { min, max } => {
                let count = data.total_records;
                let passed = count >= *min && count <= *max;
                let pct = if passed { 100.0 } else { 0.0 };
                (passed, pct, count as f64, 0, vec![])
            }
            _ => {
                // 其他规则类型默认通过
                (true, 100.0, 100.0, 0, vec![])
            }
        }
    }

    /// 启用/禁用规则
    pub fn set_enabled(&mut self, rule_id: &str, enabled: bool) -> bool {
        if let Some(rule) = self.rules.get_mut(rule_id) {
            rule.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// 删除规则
    pub fn remove(&mut self, rule_id: &str) -> bool {
        if let Some(rule) = self.rules.remove(rule_id) {
            if let Some(ids) = self.asset_rules.get_mut(&rule.asset_id) {
                ids.retain(|id| id != rule_id);
            }
            true
        } else {
            false
        }
    }

    /// 获取统计
    pub fn stats(&self) -> RuleEngineStats {
        RuleEngineStats {
            total_rules: self.rules.len(),
            enabled_rules: self.rules.values().filter(|r| r.enabled).count(),
            by_dimension: QualityDimension::all().iter().map(|d| {
                (format!("{:?}", d), self.rules.values().filter(|r| r.dimension == *d).count())
            }).collect(),
            by_severity: self.rules.iter().fold(HashMap::new(), |mut acc, (_, r)| {
                *acc.entry(format!("{:?}", r.severity)).or_insert(0) += 1;
                acc
            }),
            total_executions: self.total_executions.load(Ordering::Relaxed),
            total_passes: self.total_passes.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            pass_rate: if self.total_executions.load(Ordering::Relaxed) > 0 {
                self.total_passes.load(Ordering::Relaxed) as f64 / self.total_executions.load(Ordering::Relaxed) as f64 * 100.0
            } else { 100.0 },
        }
    }
}

impl Default for QualityRuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// 规则输入数据
#[derive(Debug, Clone, Default)]
pub struct RuleData {
    pub total_records: u64,
    pub column_values: HashMap<String, Vec<ColumnValue>>,
    pub last_updated_age_seconds: Option<u64>,
}

/// 列值
#[derive(Debug, Clone)]
pub struct ColumnValue {
    pub value: String,
    pub is_null: bool,
}

/// 资产质量分数
#[derive(Debug, Clone, Serialize)]
pub struct AssetQualityScore {
    pub asset_id: String,
    pub overall_score: f64,
    pub dimension_scores: HashMap<QualityDimension, f64>,
    pub total_rules: usize,
    pub passed_rules: usize,
    pub failed_rules: usize,
    pub critical_failures: usize,
    pub results: Vec<RuleResult>,
    pub calculated_at: String,
}

/// 规则引擎统计
#[derive(Debug, Clone, Serialize)]
pub struct RuleEngineStats {
    pub total_rules: usize,
    pub enabled_rules: usize,
    pub by_dimension: HashMap<String, usize>,
    pub by_severity: HashMap<String, usize>,
    pub total_executions: u64,
    pub total_passes: u64,
    pub total_failures: u64,
    pub pass_rate: f64,
}
