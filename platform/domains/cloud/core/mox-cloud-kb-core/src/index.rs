// =============================================================================
// 向量索引（Vector Index）
// =============================================================================

use crate::document::DocumentChunk;
use crate::embedding::cosine_similarity;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// 索引配置
// =============================================================================

/// 索引配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// 向量维度
    pub dimension: usize,
    /// 相似度度量
    pub metric: SimilarityMetric,
    /// 索引类型
    pub index_type: IndexType,
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            dimension: 1536,
            metric: SimilarityMetric::Cosine,
            index_type: IndexType::Flat,
        }
    }
}

/// 相似度度量
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimilarityMetric {
    /// 余弦相似度
    Cosine,
    /// 欧氏距离
    Euclidean,
    /// 点积
    DotProduct,
}

/// 索引类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndexType {
    /// 扁平索引（暴力搜索，适合小规模）
    Flat,
    /// HNSW 索引（近似最近邻，适合大规模）
    Hnsw,
}

// =============================================================================
// 搜索结果
// =============================================================================

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// 分块 ID
    pub chunk_id: Uuid,
    /// 文档 ID
    pub document_id: Uuid,
    /// 分块内容
    pub content: String,
    /// 相似度分数（0-1，越高越相似）
    pub score: f64,
    /// 排名（从 0 开始）
    pub rank: usize,
    /// 分块元数据
    pub chunk_index: u32,
}

// =============================================================================
// 向量索引 trait
// =============================================================================

/// 向量索引 trait
#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// 添加向量
    async fn add(&self, chunk: &DocumentChunk, embedding: &[f32]) -> Result<(), String>;

    /// 批量添加
    async fn add_batch(&self, items: &[(DocumentChunk, Vec<f32>)]) -> Result<(), String> {
        for (chunk, embedding) in items {
            self.add(chunk, embedding).await?;
        }
        Ok(())
    }

    /// 搜索最近邻
    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>, String>;

    /// 删除向量
    async fn remove(&self, chunk_id: Uuid) -> Result<(), String>;

    /// 获取索引大小
    async fn size(&self) -> usize;

    /// 清空索引
    async fn clear(&self) -> Result<(), String>;
}

// =============================================================================
// 内存向量索引（Flat 暴力搜索）
// =============================================================================

/// 内存向量索引
///
/// 使用 HashMap 存储向量，搜索时暴力计算相似度。
/// 适合小规模数据（< 10万条）和开发测试。
pub struct InMemoryVectorIndex {
    config: IndexConfig,
    data: parking_lot::RwLock<HashMap<Uuid, IndexEntry>>,
}

struct IndexEntry {
    chunk: DocumentChunk,
    embedding: Vec<f32>,
}

impl InMemoryVectorIndex {
    pub fn new(config: IndexConfig) -> Self {
        Self {
            config,
            data: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    fn compute_score(&self, query: &[f32], embedding: &[f32]) -> f64 {
        match self.config.metric {
            SimilarityMetric::Cosine => cosine_similarity(query, embedding) as f64,
            SimilarityMetric::Euclidean => {
                let dist = crate::embedding::euclidean_distance(query, embedding);
                (1.0 / (1.0 + dist as f64)).clamp(0.0, 1.0)
            }
            SimilarityMetric::DotProduct => {
                let dot: f32 = query.iter().zip(embedding.iter()).map(|(x, y)| x * y).sum();
                (dot as f64).clamp(0.0, 1.0)
            }
        }
    }
}

impl Default for InMemoryVectorIndex {
    fn default() -> Self {
        Self::new(IndexConfig::default())
    }
}

#[async_trait]
impl VectorIndex for InMemoryVectorIndex {
    async fn add(&self, chunk: &DocumentChunk, embedding: &[f32]) -> Result<(), String> {
        if embedding.len() != self.config.dimension {
            return Err(format!(
                "向量维度不匹配：期望 {}，实际 {}",
                self.config.dimension,
                embedding.len()
            ));
        }
        let mut data = self.data.write();
        data.insert(
            chunk.id,
            IndexEntry {
                chunk: chunk.clone(),
                embedding: embedding.to_vec(),
            },
        );
        Ok(())
    }

    async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>, String> {
        if query_embedding.len() != self.config.dimension {
            return Err(format!(
                "查询向量维度不匹配：期望 {}，实际 {}",
                self.config.dimension,
                query_embedding.len()
            ));
        }

        let data = self.data.read();
        let mut scores: Vec<(f64, &IndexEntry)> = data
            .values()
            .map(|entry| {
                let score = self.compute_score(query_embedding, &entry.embedding);
                (score, entry)
            })
            .collect();

        // 按分数降序排序
        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let results: Vec<SearchResult> = scores
            .into_iter()
            .take(top_k)
            .enumerate()
            .map(|(rank, (score, entry))| SearchResult {
                chunk_id: entry.chunk.id,
                document_id: entry.chunk.document_id,
                content: entry.chunk.content.clone(),
                score,
                rank,
                chunk_index: entry.chunk.chunk_index,
            })
            .collect();

        Ok(results)
    }

    async fn remove(&self, chunk_id: Uuid) -> Result<(), String> {
        let mut data = self.data.write();
        data.remove(&chunk_id);
        Ok(())
    }

    async fn size(&self) -> usize {
        self.data.read().len()
    }

    async fn clear(&self) -> Result<(), String> {
        self.data.write().clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentChunk;

    fn make_chunk(id: u8, content: &str) -> DocumentChunk {
        DocumentChunk::new(
            Uuid::new_v4(),
            "kb1",
            id as u32,
            content,
            0,
            content.chars().count(),
        )
    }

    #[tokio::test]
    async fn in_memory_index_add_and_search() {
        let config = IndexConfig {
            dimension: 4,
            ..Default::default()
        };
        let index = InMemoryVectorIndex::new(config);

        let chunk1 = make_chunk(0, "测试1");
        let chunk2 = make_chunk(1, "测试2");
        let emb1 = vec![1.0, 0.0, 0.0, 0.0];
        let emb2 = vec![0.0, 1.0, 0.0, 0.0];

        index.add(&chunk1, &emb1).await.unwrap();
        index.add(&chunk2, &emb2).await.unwrap();

        assert_eq!(index.size().await, 2);

        // 搜索与 chunk1 最相似
        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = index.search(&query, 10).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].chunk_id, chunk1.id);
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn in_memory_index_remove() {
        let config = IndexConfig { dimension: 2, ..Default::default() };
        let index = InMemoryVectorIndex::new(config);

        let chunk = make_chunk(0, "测试");
        index.add(&chunk, &[1.0, 0.0]).await.unwrap();
        assert_eq!(index.size().await, 1);

        index.remove(chunk.id).await.unwrap();
        assert_eq!(index.size().await, 0);
    }

    #[tokio::test]
    async fn in_memory_index_clear() {
        let config = IndexConfig { dimension: 2, ..Default::default() };
        let index = InMemoryVectorIndex::new(config);

        for i in 0..5 {
            let chunk = make_chunk(i, &format!("测试{}", i));
            index.add(&chunk, &[1.0, 0.0]).await.unwrap();
        }

        assert_eq!(index.size().await, 5);
        index.clear().await.unwrap();
        assert_eq!(index.size().await, 0);
    }

    #[tokio::test]
    async fn in_memory_index_dimension_mismatch() {
        let config = IndexConfig { dimension: 4, ..Default::default() };
        let index = InMemoryVectorIndex::new(config);

        let chunk = make_chunk(0, "测试");
        let result = index.add(&chunk, &[1.0, 0.0]).await;
        assert!(result.is_err());
    }
}
