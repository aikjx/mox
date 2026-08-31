//! 数据质量评估
//!
//! 完整性、唯一性、一致性、准确性、时效性等维度评估

use serde::{Deserialize, Serialize};
use crate::types::DataSet;

/// 数据质量评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQualityScore {
    /// 完整性（0-100）：非空字段占比
    pub completeness: f64,
    /// 唯一性（0-100）：无重复记录占比
    pub uniqueness: f64,
    /// 一致性（0-100）：格式/约束符合度
    pub consistency: f64,
    /// 准确性（0-100）：值的合理程度
    pub accuracy: f64,
    /// 时效性（0-100）：数据的新鲜程度
    pub timeliness: f64,
    /// 综合得分（加权平均）
    pub overall: f64,
}

impl Default for DataQualityScore {
    fn default() -> Self {
        Self {
            completeness: 100.0,
            uniqueness: 100.0,
            consistency: 100.0,
            accuracy: 100.0,
            timeliness: 100.0,
            overall: 100.0,
        }
    }
}

/// 数据质量报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityReport {
    /// 总体评分
    pub score: DataQualityScore,
    /// 字段级质量详情
    pub field_details: std::collections::HashMap<String, FieldQuality>,
    /// 问题记录数
    pub issue_count: usize,
}

/// 字段级质量信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldQuality {
    /// 字段名
    pub field_name: String,
    /// 非空率
    pub non_null_rate: f64,
    /// 唯一率
    pub unique_rate: f64,
    /// 异常值数量
    pub outlier_count: usize,
}

/// 评估数据集质量
///
/// # Arguments
/// * `dataset` - 输入数据集
///
/// # Returns
/// 数据质量报告
pub fn assess_quality(dataset: &DataSet) -> QualityReport {
    // TODO: 实现完整的数据质量评估
    let total_fields = dataset.schema.len() as f64;
    let completeness = if dataset.rows.is_empty() {
        100.0
    } else {
        let total_cells = dataset.rows.len() as f64 * total_fields.max(1.0);
        let mut non_null = 0usize;
        for row in &dataset.rows {
            for field in &dataset.schema {
                if row.has(field) {
                    non_null += 1;
                }
            }
        }
        if total_cells > 0.0 {
            non_null as f64 / total_cells * 100.0
        } else {
            100.0
        }
    };

    QualityReport {
        score: DataQualityScore {
            completeness,
            ..Default::default()
        },
        field_details: std::collections::HashMap::new(),
        issue_count: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_empty_dataset() {
        let ds = DataSet::new();
        let report = assess_quality(&ds);
        assert_eq!(report.score.overall, 100.0);
        assert_eq!(report.issue_count, 0);
    }
}
