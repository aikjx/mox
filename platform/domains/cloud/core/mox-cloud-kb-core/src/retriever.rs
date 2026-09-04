// =============================================================================
// 检索器（Retriever）
// =============================================================================

use crate::embedding::EmbeddingProvider;
use crate::index::{SearchResult, VectorIndex};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// 检索查询
// =============================================================================

/// 检索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQuery {
    /// 查询文本
    pub query: String,
    /// 返回数量
    pub top_k: usize,
    /// 过滤条件（文档ID、标签等）
    #[serde(default)]
    pub filters: HashMap<String, String>,
    /// 是否启用混合检索
    pub hybrid: bool,
}

impl RetrievalQuery {
    pub fn new(query: impl Into<String>, top_k: usize) -> Self {
        Self {
            query: query.into(),
            top_k,
            filters: HashMap::new(),
            hybrid: true,
        }
    }
}

// =============================================================================
// 检索结果
// =============================================================================

/// 检索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// 查询文本
    pub query: String,
    /// 搜索结果
    pub results: Vec<SearchResult>,
    /// 语义检索结果数
    pub semantic_count: usize,
    /// 关键词检索结果数
    pub keyword_count: usize,
    /// 检索耗时（毫秒）
    pub latency_ms: u64,
}

// =============================================================================
// 检索器 trait
// =============================================================================

/// 检索器 trait
#[async_trait]
pub trait Retriever: Send + Sync {
    /// 执行检索
    async fn retrieve(&self, query: &RetrievalQuery) -> Result<RetrievalResult, String>;
}

// =============================================================================
// 混合检索器（语义 + 关键词 RRF 融合）
// =============================================================================

/// 混合检索器
///
/// 结合语义检索（向量相似度）和关键词检索（BM25/TF-IDF），
/// 使用 RRF（Reciprocal Rank Fusion）融合排序。
pub struct HybridRetriever<E: EmbeddingProvider, I: VectorIndex> {
    embedding_provider: E,
    vector_index: I,
    /// 语义检索权重
    semantic_weight: f64,
    /// 关键词检索权重
    keyword_weight: f64,
    /// RRF 常数 k
    rrf_k: f64,
}

impl<E: EmbeddingProvider, I: VectorIndex> HybridRetriever<E, I> {
    pub fn new(embedding_provider: E, vector_index: I) -> Self {
        Self {
            embedding_provider,
            vector_index,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
            rrf_k: 60.0,
        }
    }

    pub fn with_weights(mut self, semantic_weight: f64, keyword_weight: f64) -> Self {
        self.semantic_weight = semantic_weight;
        self.keyword_weight = keyword_weight;
        self
    }

    /// 语义检索
    async fn semantic_search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>, String> {
        let embedding = self.embedding_provider.embed(query).await?;
        self.vector_index.search(&embedding.vector, top_k).await
    }

    /// 关键词检索（简单的 TF-IDF 风格）
    async fn keyword_search(&self, query: &str, top_k: usize, all_results: &[SearchResult]) -> Vec<SearchResult> {
        let query_terms: Vec<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(f64, SearchResult)> = all_results
            .iter()
            .map(|result| {
                let mut score = 0.0;
                for term in &query_terms {
                    if result.content.to_lowercase().contains(&term.to_lowercase()) {
                        score += 1.0;
                    }
                }
                (score, result.clone())
            })
            .filter(|(score, _)| *score > 0.0)
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        scored
            .into_iter()
            .take(top_k)
            .enumerate()
            .map(|(rank, (score, mut result))| {
                result.score = score;
                result.rank = rank;
                result
            })
            .collect()
    }

    /// RRF 融合
    fn rrf_fuse(&self, semantic: &[SearchResult], keyword: &[SearchResult]) -> Vec<SearchResult> {
        let mut fused: HashMap<uuid::Uuid, (f64, SearchResult)> = HashMap::new();

        // 语义检索 RRF 分数
        for (rank, result) in semantic.iter().enumerate() {
            let rrf_score = self.semantic_weight / (self.rrf_k + rank as f64 + 1.0);
            let entry = fused.entry(result.chunk_id).or_insert((0.0, result.clone()));
            entry.0 += rrf_score;
        }

        // 关键词检索 RRF 分数
        for (rank, result) in keyword.iter().enumerate() {
            let rrf_score = self.keyword_weight / (self.rrf_k + rank as f64 + 1.0);
            let entry = fused.entry(result.chunk_id).or_insert((0.0, result.clone()));
            entry.0 += rrf_score;
        }

        let mut results: Vec<(f64, SearchResult)> = fused.into_values().collect();
        results.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        results
            .into_iter()
            .enumerate()
            .map(|(rank, (score, mut result))| {
                result.score = score;
                result.rank = rank;
                result
            })
            .collect()
    }
}

#[async_trait]
impl<E: EmbeddingProvider + Sync, I: VectorIndex + Sync> Retriever for HybridRetriever<E, I> {
    async fn retrieve(&self, query: &RetrievalQuery) -> Result<RetrievalResult, String> {
        let start = std::time::Instant::now();

        // 语义检索（获取更多候选用于融合）
        let semantic_top_k = (query.top_k as f64 * 2.0) as usize;
        let semantic_results = self.semantic_search(&query.query, semantic_top_k).await?;

        // 关键词检索
        let keyword_results = if query.hybrid {
            self.keyword_search(&query.query, semantic_top_k, &semantic_results).await
        } else {
            vec![]
        };

        // RRF 融合
        let fused = if query.hybrid && !keyword_results.is_empty() {
            self.rrf_fuse(&semantic_results, &keyword_results)
        } else {
            semantic_results.clone()
        };

        // 取 top_k
        let results: Vec<SearchResult> = fused.into_iter().take(query.top_k).collect();

        Ok(RetrievalResult {
            query: query.query.clone(),
            results,
            semantic_count: semantic_results.len(),
            keyword_count: keyword_results.len(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentChunk;
    use crate::embedding::MockEmbeddingProvider;
    use crate::index::{IndexConfig, InMemoryVectorIndex};
    use uuid::Uuid;

    #[tokio::test]
    async fn hybrid_retriever_basic() {
        let embedding = MockEmbeddingProvider::new(64);
        let config = IndexConfig { dimension: 64, ..Default::default() };
        let index = InMemoryVectorIndex::new(config);

        // 添加一些文档
        for i in 0..5 {
            let chunk = DocumentChunk::new(
                Uuid::new_v4(),
                "kb1",
                i,
                format!("文档内容 {} 关于 Rust 编程", i),
                0,
                100,
            );
            let emb = embedding.embed(&chunk.content).await.unwrap();
            index.add(&chunk, &emb.vector).await.unwrap();
        }

        let retriever = HybridRetriever::new(embedding, index);
        let query = RetrievalQuery::new("Rust 编程", 3);
        let result = retriever.retrieve(&query).await.unwrap();

        assert_eq!(result.results.len(), 3);
        assert!(result.latency_ms >= 0);
        assert_eq!(result.semantic_count, 5);
    }

    #[test]
    fn retrieval_query_creation() {
        let query = RetrievalQuery::new("测试", 5);
        assert_eq!(query.query, "测试");
        assert_eq!(query.top_k, 5);
        assert!(query.hybrid);
    }
}
