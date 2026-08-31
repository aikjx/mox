//! 数据标准化/归一化
//!
//! Min-Max 归一化、Z-Score 标准化、对数变换等

use crate::types::DataSet;

/// 归一化方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizeMethod {
    /// Min-Max 归一化（缩放到 [0, 1]）
    MinMax,
    /// Z-Score 标准化（均值 0，标准差 1）
    ZScore,
    /// 最大绝对值归一化
    MaxAbs,
    /// 对数变换
    LogTransform,
    /// 稳健归一化（中位数 + 四分位距）
    RobustScaler,
}

/// 归一化配置
#[derive(Debug, Clone)]
pub struct NormalizeConfig {
    /// 归一化方法
    pub method: NormalizeMethod,
    /// 目标字段列表（空表示所有数值字段）
    pub fields: Vec<String>,
    /// Min-Max 的目标范围下限
    pub min_range: f64,
    /// Min-Max 的目标范围上限
    pub max_range: f64,
}

impl Default for NormalizeConfig {
    fn default() -> Self {
        Self {
            method: NormalizeMethod::MinMax,
            fields: vec![],
            min_range: 0.0,
            max_range: 1.0,
        }
    }
}

/// 归一化统计信息（用于反向转换）
#[derive(Debug, Clone, Default)]
pub struct NormalizeStats {
    /// 最小值
    pub min: f64,
    /// 最大值
    pub max: f64,
    /// 均值
    pub mean: f64,
    /// 标准差
    pub std_dev: f64,
    /// 中位数
    pub median: f64,
    /// 四分位距
    pub iqr: f64,
}

/// 对数据集进行归一化
///
/// # Arguments
/// * `dataset` - 输入数据集
/// * `config` - 归一化配置
///
/// # Returns
/// (归一化后的数据集, 各字段统计信息)
pub fn normalize(dataset: &DataSet, config: &NormalizeConfig) -> (DataSet, std::collections::HashMap<String, NormalizeStats>) {
    // TODO: 实现完整的归一化逻辑
    let _ = config;
    (dataset.clone(), std::collections::HashMap::new())
}

/// 计算字段的统计信息
pub fn compute_stats(dataset: &DataSet, field: &str) -> Option<NormalizeStats> {
    // TODO: 实现统计信息计算
    let _ = (dataset, field);
    Some(NormalizeStats::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_empty() {
        let ds = DataSet::new();
        let config = NormalizeConfig::default();
        let (result, stats) = normalize(&ds, &config);
        assert_eq!(result.row_count(), 0);
        assert!(stats.is_empty());
    }
}
