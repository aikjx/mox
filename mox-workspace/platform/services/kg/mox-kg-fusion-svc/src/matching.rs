//! 实体匹配
//!
//! 属性匹配、相似度计算等核心匹配算法

use serde::{Deserialize, Serialize};

/// 字符串相似度算法
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StringSimilarityMethod {
    /// 编辑距离（Levenshtein）
    Levenshtein,
    /// Jaro-Winkler
    JaroWinkler,
    /// 余弦相似度
    Cosine,
    /// Jaccard 相似度
    Jaccard,
    /// 最长公共子串
    LongestCommonSubstring,
}

/// 计算 Levenshtein 编辑距离
///
/// 返回将字符串 a 转换为 b 所需的最小编辑操作数
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let (n, m) = (a_chars.len(), b_chars.len());

    if n == 0 {
        return m;
    }
    if m == 0 {
        return n;
    }

    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 0..=n {
        dp[i][0] = i;
    }
    for j in 0..=m {
        dp[0][j] = j;
    }

    for i in 1..=n {
        for j in 1..=m {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            dp[i][j] = *[
                dp[i - 1][j] + 1,      // 删除
                dp[i][j - 1] + 1,      // 插入
                dp[i - 1][j - 1] + cost, // 替换
            ].iter().min().unwrap();
        }
    }

    dp[n][m]
}

/// 计算字符串相似度（基于编辑距离归一化）
///
/// 返回值范围 0.0 - 1.0，1.0 表示完全相同
pub fn string_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let distance = levenshtein_distance(a, b) as f64;
    let max_len = a.chars().count().max(b.chars().count()) as f64;
    if max_len == 0.0 {
        1.0
    } else {
        1.0 - distance / max_len
    }
}

/// Jaccard 相似度
pub fn jaccard_similarity(a: &std::collections::HashSet<String>, b: &std::collections::HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_identical() {
        assert_eq!(levenshtein_distance("hello", "hello"), 0);
    }

    #[test]
    fn test_levenshtein_empty() {
        assert_eq!(levenshtein_distance("", "abc"), 3);
        assert_eq!(levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_levenshtein_kitten_sitting() {
        // 经典例子：kitten -> sitting 需要 3 步
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_string_similarity_identical() {
        assert!((string_similarity("test", "test") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_string_similarity_different() {
        let sim = string_similarity("abc", "xyz");
        assert!(sim >= 0.0 && sim < 1.0);
    }

    #[test]
    fn test_jaccard_identical() {
        let a: std::collections::HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        let b: std::collections::HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard_similarity(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_jaccard_disjoint() {
        let a: std::collections::HashSet<String> = ["a", "b"].iter().map(|s| s.to_string()).collect();
        let b: std::collections::HashSet<String> = ["c", "d"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard_similarity(&a, &b) - 0.0).abs() < f64::EPSILON);
    }
}
