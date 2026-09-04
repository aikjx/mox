// =============================================================================
// 重排序器（Reranker）
// =============================================================================

use crate::index::SearchResult;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// =============================================================================
// 重排序结果
// =============================================================================

/// 重排序结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    /// 原始排名
    pub original_rank: usize,
    /// 新排名
    pub new_rank: usize,
    /// 重排序分数
    pub rerank_score: f64,
    /// 搜索结果
    pub result: SearchResult,
}

// =============================================================================
// 重排序器 trait
// =============================================================================

/// 重排序器 trait
#[async_trait]
pub trait Reranker: Send + Sync {
    /// 对搜索结果进行重排序
    async fn rerank(&self, query: &str, results: Vec<SearchResult>) -> Result<Vec<RerankResult>, String>;

    /// 是否启用
    fn is_enabled(&self) -> bool {
        true
    }
}

// =============================================================================
// Cross-Encoder 重排序器
// =============================================================================

/// Cross-Encoder 重排序器
///
/// 使用交叉编码器模型对 query-document 对进行打分。
/// 实际生产环境应调用外部模型服务（如 bge-reranker）。
pub struct CrossEncoderReranker {
    /// 模型端点
    endpoint: String,
    /// API Key
    api_key: Option<String>,
    /// 模型名称
    model: String,
    /// 批处理大小
    batch_size: usize,
    /// 是否启用
    enabled: bool,
}

impl CrossEncoderReranker {
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: None,
            model: model.into(),
            batch_size: 32,
            enabled: true,
        }
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.max(1);
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    /// 计算单对分数（模拟实现）
    /// 实际应调用外部模型服务
    async fn score_pair(&self, query: &str, document: &str) -> f64 {
        // 简化实现：基于词重叠率的分数
        // 生产环境应替换为真实的 cross-encoder 模型调用
        let query_terms: std::collections::HashSet<&str> = query.split_whitespace().collect();
        let doc_terms: std::collections::HashSet<&str> = document.split_whitespace().collect();

        if query_terms.is_empty() {
            return 0.0;
        }

        let overlap = query_terms.intersection(&doc_terms).count();
        let score = overlap as f64 / query_terms.len() as f64;

        score.clamp(0.0, 1.0)
    }
}

#[async_trait]
impl Reranker for CrossEncoderReranker {
    async fn rerank(&self, query: &str, results: Vec<SearchResult>) -> Result<Vec<RerankResult>, String> {
        if !self.enabled || results.is_empty() {
            return Ok(results
                .into_iter()
                .enumerate()
                .map(|(i, result)| RerankResult {
                    original_rank: i,
                    new_rank: i,
                    rerank_score: result.score,
                    result,
                })
                .collect());
        }

        // 批量计算重排序分数
        let mut scored: Vec<(usize, f64, SearchResult)> = Vec::with_capacity(results.len());

        for (i, result) in results.iter().enumerate() {
            let score = self.score_pair(query, &result.content).await;
            // 融合原始分数和重排序分数
            let fused_score = 0.4 * result.score + 0.6 * score;
            scored.push((i, fused_score, result.clone()));
        }

        // 按重排序分数降序
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let reranked: Vec<RerankResult> = scored
            .into_iter()
            .enumerate()
            .map(|(new_rank, (original_rank, rerank_score, result))| RerankResult {
                original_rank,
                new_rank,
                rerank_score,
                result,
            })
            .collect();

        Ok(reranked)
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// =============================================================================
// Mock 重排序器（用于测试）
// =============================================================================

/// Mock 重排序器
///
/// 不改变顺序，直接返回原始结果。用于测试和开发。
pub struct MockReranker {
    enabled: bool,
}

impl MockReranker {
    pub fn new() -> Self {
        Self { enabled: true }
    }
}

impl Default for MockReranker {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Reranker for MockReranker {
    async fn rerank(&self, _query: &str, results: Vec<SearchResult>) -> Result<Vec<RerankResult>, String> {
        Ok(results
            .into_iter()
            .enumerate()
            .map(|(i, result)| RerankResult {
                original_rank: i,
                new_rank: i,
                rerank_score: result.score,
                result,
            })
            .collect())
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::SearchResult;
    use uuid::Uuid;

    fn make_result(rank: usize, content: &str, score: f64) -> SearchResult {
        SearchResult {
            chunk_id: Uuid::new_v4(),
            document_id: Uuid::new_v4(),
            content: content.to_string(),
            score,
            rank,
            chunk_index: rank as u32,
        }
    }

    #[tokio::test]
    async fn mock_reranker_preserves_order() {
        let reranker = MockReranker::new();
        let results = vec![
            make_result(0, "文档1", 0.9),
            make_result(1, "文档2", 0.8),
            make_result(2, "文档3", 0.7),
        ];

        let reranked = reranker.rerank("查询", results).await.unwrap();

        assert_eq!(reranked.len(), 3);
        assert_eq!(reranked[0].original_rank, 0);
        assert_eq!(reranked[0].new_rank, 0);
        assert_eq!(reranked[2].original_rank, 2);
    }

    #[tokio::test]
    async fn cross_encoder_reranker_reorders() {
        let reranker = CrossEncoderReranker::new("http://localhost:8000", "bge-reranker");

        // 原始顺序：文档1分数最高，但内容与查询不相关
        // 文档3分数最低，但内容与查询相关
        let results = vec![
            make_result(0, "完全不相关的内容", 0.95),
            make_result(1, "部分相关 Rust 编程", 0.80),
            make_result(2, "Rust 编程教程详细内容", 0.60),
        ];

        let reranked = reranker.rerank("Rust 编程", results).await.unwrap();

        assert_eq!(reranked.len(), 3);
        // 重排序后，相关文档应该排名靠前
        assert!(reranked[0].result.content.contains("Rust"));
        assert!(reranked[0].new_rank == 0);
    }

    #[tokio::test]
    async fn cross_encoder_disabled() {
        let reranker = CrossEncoderReranker::new("http://localhost:8000", "model").disabled();

        let results = vec![make_result(0, "测试", 0.5)];
        let reranked = reranker.rerank("查询", results).await.unwrap();

        assert_eq!(reranked.len(), 1);
        assert_eq!(reranked[0].new_rank, 0);
    }

    #[tokio::test]
    async fn reranker_empty_results() {
        let reranker = MockReranker::new();
        let reranked = reranker.rerank("查询", vec![]).await.unwrap();
        assert!(reranked.is_empty());
    }
}
