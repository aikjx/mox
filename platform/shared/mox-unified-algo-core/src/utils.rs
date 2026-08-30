// Copyright (c) 2026 璇玑 RelGraph · 开发专家联盟
// Licensed under the MIT License.

//! # 通用工具函数
//!
//! 算法模块共享的工具函数。

use std::collections::HashMap;

/// 归一化向量到 [0, 1] 范围（Min-Max 归一化）
pub fn min_max_normalize(values: &mut [f64]) {
    if values.is_empty() {
        return;
    }
    let min = values
        .iter()
        .cloned()
        .fold(f64::INFINITY, f64::min);
    let max = values
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;
    if range < 1e-10 {
        for v in values.iter_mut() {
            *v = 0.5;
        }
        return;
    }
    for v in values.iter_mut() {
        *v = (*v - min) / range;
    }
}

/// 归一化向量使其和为 1（Softmax 风格的线性归一化）
pub fn normalize_sum(values: &mut [f64]) {
    let sum: f64 = values.iter().sum();
    if sum.abs() < 1e-10 {
        let n = values.len() as f64;
        for v in values.iter_mut() {
            *v = 1.0 / n;
        }
        return;
    }
    for v in values.iter_mut() {
        *v /= sum;
    }
}

/// 计算 Top-K 索引（按值降序）
pub fn top_k_indices(scores: &[f64], k: usize) -> Vec<usize> {
    let mut indexed: Vec<(usize, f64)> = scores.iter().enumerate().map(|(i, s)| (i, *s)).collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.truncate(k);
    indexed.into_iter().map(|(i, _)| i).collect()
}

/// 安全的 f64 比较（处理 NaN）
pub fn safe_cmp(a: f64, b: f64) -> std::cmp::Ordering {
    if a.is_nan() && b.is_nan() {
        std::cmp::Ordering::Equal
    } else if a.is_nan() {
        std::cmp::Ordering::Less
    } else if b.is_nan() {
        std::cmp::Ordering::Greater
    } else {
        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// 哈希映射加权求和
pub fn weighted_sum(scores: &HashMap<String, f64>, weights: &[(String, f64)]) -> f64 {
    let mut total = 0.0;
    let mut weight_sum = 0.0;
    for (name, weight) in weights {
        if let Some(&score) = scores.get(name) {
            total += score * weight;
            weight_sum += weight;
        }
    }
    if weight_sum > 0.0 {
        total / weight_sum
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_normalize() {
        let mut v = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        min_max_normalize(&mut v);
        assert!((v[0] - 0.0).abs() < 1e-6);
        assert!((v[4] - 1.0).abs() < 1e-6);
        assert!((v[2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_sum() {
        let mut v = vec![1.0, 2.0, 3.0];
        normalize_sum(&mut v);
        let sum: f64 = v.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!((v[0] - 1.0 / 6.0).abs() < 1e-6);
    }

    #[test]
    fn test_top_k_indices() {
        let scores = vec![0.1, 0.5, 0.3, 0.9, 0.2];
        let top3 = top_k_indices(&scores, 3);
        assert_eq!(top3, vec![3, 1, 2]); // 0.9, 0.5, 0.3
    }

    #[test]
    fn test_safe_cmp() {
        use std::cmp::Ordering;
        assert_eq!(safe_cmp(1.0, 2.0), Ordering::Less);
        assert_eq!(safe_cmp(2.0, 1.0), Ordering::Greater);
        assert_eq!(safe_cmp(1.0, 1.0), Ordering::Equal);
        assert_eq!(safe_cmp(f64::NAN, 1.0), Ordering::Less);
    }
}
