// =============================================================================
// 向量嵌入（Embedding）
// =============================================================================

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// =============================================================================
// 嵌入结果
// =============================================================================

/// 嵌入结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// 向量
    pub vector: Vec<f32>,
    /// 维度
    pub dim: usize,
    /// 模型名称
    pub model: String,
    /// 输入 token 数
    pub prompt_tokens: Option<u32>,
    /// 耗时（毫秒）
    pub latency_ms: u64,
}

// =============================================================================
// 嵌入提供者 trait
// =============================================================================

/// 嵌入提供者 trait
///
/// 支持多种后端：OpenAI、本地模型、Mock 等。
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 嵌入单个文本
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, String>;

    /// 批量嵌入
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<EmbeddingResult>, String> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// 获取嵌入维度
    fn dimension(&self) -> usize;

    /// 获取模型名称
    fn model_name(&self) -> &str;
}

// =============================================================================
// Mock 嵌入提供者（用于测试）
// =============================================================================

/// Mock 嵌入提供者
///
/// 生成确定性的伪随机向量，用于测试和开发。
pub struct MockEmbeddingProvider {
    dim: usize,
    model: String,
}

impl MockEmbeddingProvider {
    pub fn new(dim: usize) -> Self {
        Self {
            dim,
            model: "mock-embedding".to_string(),
        }
    }

    /// 基于文本内容生成确定性向量
    fn deterministic_vector(&self, text: &str) -> Vec<f32> {
        let mut vector = vec![0.0f32; self.dim];
        let bytes = text.as_bytes();

        for (i, &byte) in bytes.iter().enumerate() {
            let idx = i % self.dim;
            vector[idx] += (byte as f32) / 255.0;
        }

        // 归一化
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vector.iter_mut() {
                *v /= norm;
            }
        }

        vector
    }
}

impl Default for MockEmbeddingProvider {
    fn default() -> Self {
        Self::new(1536)
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<EmbeddingResult, String> {
        let vector = self.deterministic_vector(text);
        Ok(EmbeddingResult {
            vector,
            dim: self.dim,
            model: self.model.clone(),
            prompt_tokens: Some(text.chars().count() as u32 / 4),
            latency_ms: 1,
        })
    }

    fn dimension(&self) -> usize {
        self.dim
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

// =============================================================================
// 余弦相似度
// =============================================================================

/// 计算两个向量的余弦相似度
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    (dot / (norm_a * norm_b)).clamp(-1.0, 1.0)
}

/// 计算两个向量的欧氏距离
pub fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::MAX;
    }

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f32>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_embedding_basic() {
        let provider = MockEmbeddingProvider::new(128);
        let result = provider.embed("测试文本").await.unwrap();

        assert_eq!(result.dim, 128);
        assert_eq!(result.vector.len(), 128);
        assert_eq!(result.model, "mock-embedding");
    }

    #[tokio::test]
    async fn mock_embedding_deterministic() {
        let provider = MockEmbeddingProvider::new(64);
        let r1 = provider.embed("相同文本").await.unwrap();
        let r2 = provider.embed("相同文本").await.unwrap();

        assert_eq!(r1.vector, r2.vector);
    }

    #[tokio::test]
    async fn mock_embedding_batch() {
        let provider = MockEmbeddingProvider::new(32);
        let texts = vec!["文本1".to_string(), "文本2".to_string(), "文本3".to_string()];
        let results = provider.embed_batch(&texts).await.unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &a);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn euclidean_distance_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let dist = euclidean_distance(&a, &b);
        assert!((dist - 5.0).abs() < 1e-6);
    }
}
