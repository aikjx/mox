//! MOX 统一基座 · 统一索引层
//!
//! 定义"一次写入、三路检索"的统一索引契约：
//! - **元数据索引**（`MetadataIndex`）：按属性 / 标签精确过滤
//! - **全文索引**（`FullTextIndex`）：关键词搜索
//! - **向量索引**（`VectorIndex`）：语义相似度检索（RAG / 图谱语义召回）
//!
//! ## 设计原则
//! - 只定义 trait 契约，不内置后端（实现方可接入 Elasticsearch / OpenSearch / 向量库 / 内存实现）。
//! - data 域 mox-data-catalog-svc 写入元数据索引一份；ai 域经此做语义检索。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 统一索引错误
#[derive(Debug, Error)]
pub enum IndexError {
    #[error("索引条目不存在: {id}")]
    NotFound { id: String },
    #[error("IO 错误: {0}")]
    Io(String),
    #[error("向量维度不匹配: 期望 {expected} 实际 {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    #[error("其他错误: {0}")]
    Other(String),
}

/// 统一索引结果
pub type IndexResult<T> = Result<T, IndexError>;

/// 索引条目（统一模型的最小可检索单元）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// 统一 ID（对齐 mox-base-model-core 的 Node/Blob ID）
    pub id: String,
    /// 条目类型（node / blob）
    pub kind: String,
    /// 元数据属性
    pub props: std::collections::HashMap<String, serde_json::Value>,
    /// 全文内容（可空）
    pub text: Option<String>,
    /// 向量嵌入（可空）
    pub vector: Option<Vec<f32>>,
}

/// 元数据索引：按属性 / 标签过滤
#[async_trait]
pub trait MetadataIndex: Send + Sync {
    /// 写入 / 更新条目
    async fn put(&self, entry: IndexEntry) -> IndexResult<()>;

    /// 按属性精确匹配查询
    async fn filter(&self, prop: &str, value: &serde_json::Value) -> IndexResult<Vec<String>>;

    /// 按属性范围查询
    async fn range(&self, prop: &str, min: f64, max: f64) -> IndexResult<Vec<String>>;

    /// 删除条目
    async fn delete(&self, id: &str) -> IndexResult<()>;
}

/// 全文索引：关键词搜索
#[async_trait]
pub trait FullTextIndex: Send + Sync {
    /// 写入 / 更新条目
    async fn put(&self, id: &str, text: &str) -> IndexResult<()>;

    /// 关键词搜索，返回命中 ID 列表（按相关度排序）
    async fn search(&self, query: &str, limit: usize) -> IndexResult<Vec<String>>;

    /// 删除条目
    async fn delete(&self, id: &str) -> IndexResult<()>;
}

/// 向量索引：语义相似度检索
#[async_trait]
pub trait VectorIndex: Send + Sync {
    /// 向量维度
    fn dimension(&self) -> usize;

    /// 写入 / 更新向量
    async fn put(&self, id: &str, vector: &[f32]) -> IndexResult<()>;

    /// 查询最近邻，返回 (id, 相似度)
    async fn search(&self, vector: &[f32], limit: usize) -> IndexResult<Vec<(String, f32)>>;

    /// 删除条目
    async fn delete(&self, id: &str) -> IndexResult<()>;
}

/// 内存版元数据索引（默认实现 / 测试用；生产由 data 域替换）
pub struct InMemoryMetadataIndex {
    entries: std::sync::Mutex<std::collections::HashMap<String, IndexEntry>>,
}

impl Default for InMemoryMetadataIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryMetadataIndex {
    /// 新建内存元数据索引
    pub fn new() -> Self {
        Self {
            entries: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl MetadataIndex for InMemoryMetadataIndex {
    async fn put(&self, entry: IndexEntry) -> IndexResult<()> {
        self.entries
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .insert(entry.id.clone(), entry);
        Ok(())
    }

    async fn filter(&self, prop: &str, value: &serde_json::Value) -> IndexResult<Vec<String>> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?;
        Ok(entries
            .values()
            .filter(|e| e.props.get(prop) == Some(value))
            .map(|e| e.id.clone())
            .collect())
    }

    async fn range(&self, prop: &str, min: f64, max: f64) -> IndexResult<Vec<String>> {
        let entries = self
            .entries
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?;
        Ok(entries
            .values()
            .filter(|e| {
                e.props
                    .get(prop)
                    .and_then(|v| v.as_f64())
                    .map(|n| n >= min && n <= max)
                    .unwrap_or(false)
            })
            .map(|e| e.id.clone())
            .collect())
    }

    async fn delete(&self, id: &str) -> IndexResult<()> {
        self.entries
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .remove(id);
        Ok(())
    }
}

/// 内存版全文索引（默认实现 / 测试用）
pub struct InMemoryFullTextIndex {
    docs: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl Default for InMemoryFullTextIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryFullTextIndex {
    /// 新建内存全文索引
    pub fn new() -> Self {
        Self {
            docs: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl FullTextIndex for InMemoryFullTextIndex {
    async fn put(&self, id: &str, text: &str) -> IndexResult<()> {
        self.docs
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .insert(id.to_string(), text.to_string());
        Ok(())
    }

    async fn search(&self, query: &str, limit: usize) -> IndexResult<Vec<String>> {
        let docs = self
            .docs
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?;
        let q = query.to_lowercase();
        let mut hits: Vec<(String, usize)> = docs
            .iter()
            .filter_map(|(id, text)| {
                let count = text.to_lowercase().matches(&q).count();
                if count > 0 {
                    Some((id.clone(), count))
                } else {
                    None
                }
            })
            .collect();
        hits.sort_by_key(|x| std::cmp::Reverse(x.1));
        Ok(hits.into_iter().take(limit).map(|(id, _)| id).collect())
    }

    async fn delete(&self, id: &str) -> IndexResult<()> {
        self.docs
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .remove(id);
        Ok(())
    }
}

/// 内存版向量索引（默认实现 / 测试用，内积相似度）
pub struct InMemoryVectorIndex {
    dimension: usize,
    vectors: std::sync::Mutex<std::collections::HashMap<String, Vec<f32>>>,
}

impl InMemoryVectorIndex {
    /// 新建内存向量索引
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            vectors: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl VectorIndex for InMemoryVectorIndex {
    fn dimension(&self) -> usize {
        self.dimension
    }

    async fn put(&self, id: &str, vector: &[f32]) -> IndexResult<()> {
        if vector.len() != self.dimension {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }
        self.vectors
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .insert(id.to_string(), vector.to_vec());
        Ok(())
    }

    async fn search(&self, vector: &[f32], limit: usize) -> IndexResult<Vec<(String, f32)>> {
        if vector.len() != self.dimension {
            return Err(IndexError::DimensionMismatch {
                expected: self.dimension,
                actual: vector.len(),
            });
        }
        let vectors = self
            .vectors
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?;
        let mut scored: Vec<(String, f32)> = vectors
            .iter()
            .map(|(id, v)| {
                let sim: f32 = v
                    .iter()
                    .zip(vector.iter())
                    .map(|(a, b)| a * b)
                    .sum();
                (id.clone(), sim)
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    async fn delete(&self, id: &str) -> IndexResult<()> {
        self.vectors
            .lock()
            .map_err(|e| IndexError::Other(e.to_string()))?
            .remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn metadata_filter_works() {
        let idx = InMemoryMetadataIndex::new();
        let mut props = std::collections::HashMap::new();
        props.insert("domain".to_string(), serde_json::json!("kg"));
        idx.put(IndexEntry {
            id: "n1".into(),
            kind: "node".into(),
            props,
            text: None,
            vector: None,
        })
        .await
        .unwrap();
        let hits = idx
            .filter("domain", &serde_json::json!("kg"))
            .await
            .unwrap();
        assert_eq!(hits, vec!["n1".to_string()]);
    }

    #[tokio::test]
    async fn metadata_range_works() {
        let idx = InMemoryMetadataIndex::new();
        let mut props = std::collections::HashMap::new();
        props.insert("priority".to_string(), serde_json::json!(7));
        idx.put(IndexEntry {
            id: "n1".into(),
            kind: "node".into(),
            props,
            text: None,
            vector: None,
        })
        .await
        .unwrap();
        let hits = idx.range("priority", 5.0, 9.0).await.unwrap();
        assert_eq!(hits.len(), 1);
        let miss = idx.range("priority", 8.0, 9.0).await.unwrap();
        assert!(miss.is_empty());
    }

    #[tokio::test]
    async fn fulltext_search_works() {
        let idx = InMemoryFullTextIndex::new();
        idx.put("a", "人工智能与知识图谱").await.unwrap();
        idx.put("b", "机器学习基础").await.unwrap();
        let hits = idx.search("知识图谱", 10).await.unwrap();
        assert_eq!(hits, vec!["a".to_string()]);
    }

    #[tokio::test]
    async fn vector_search_ranks_by_similarity() {
        let idx = InMemoryVectorIndex::new(3);
        idx.put("q", &[1.0, 0.0, 0.0]).await.unwrap();
        idx.put("r", &[0.9, 0.1, 0.0]).await.unwrap();
        let hits = idx.search(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].0, "q");
    }

    #[tokio::test]
    async fn vector_dimension_mismatch_rejected() {
        let idx = InMemoryVectorIndex::new(3);
        let r = idx.put("x", &[1.0, 2.0]).await;
        assert!(matches!(r, Err(IndexError::DimensionMismatch { .. })));
    }
}
