//! AI 核心类型定义

use serde::{Deserialize, Serialize};

/// 模型角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 完成度设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionOptions {
    /// 温度 0.0-2.0
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大生成 token 数
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Top P 采样
    #[serde(default = "default_top_p")]
    pub top_p: f32,
    /// 停止序列
    #[serde(default)]
    pub stop: Vec<String>,
    /// 是否流式输出
    #[serde(default)]
    pub stream: bool,
}

fn default_temperature() -> f32 { 0.7 }
fn default_max_tokens() -> u32 { 2048 }
fn default_top_p() -> f32 { 1.0 }

impl Default for CompletionOptions {
    fn default() -> Self {
        Self {
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            top_p: default_top_p(),
            stop: vec![],
            stream: false,
        }
    }
}

/// Embedding 结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub vector: Vec<f32>,
    pub model: String,
    pub dimensions: usize,
}

/// 余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_same_vector() {
        let v = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }
}
