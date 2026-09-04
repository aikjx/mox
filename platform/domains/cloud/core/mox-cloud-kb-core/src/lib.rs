// =============================================================================
// MOX 云盘知识库核心（mox-cloud-kb-core）
// =============================================================================
//
// RAG（Retrieval-Augmented Generation）系统的核心组件，提供：
//
// 1. **文档模型**（document）— 文档实体、元数据、版本管理
// 2. **分块器**（chunker）— 多种分块策略（固定大小/语义/递归）
// 3. **向量嵌入**（embedding）— EmbeddingProvider trait，支持多后端
// 4. **向量索引**（index）— VectorIndex trait，支持内存/HNSW/FAISS
// 5. **检索器**（retriever）— 语义检索 + 关键词检索融合（RRF）
// 6. **重排序器**（reranker）— Cross-encoder 重排序
//
// 设计原则：
// - 依赖注入：所有外部依赖（Embedding/Index）通过 trait 注入
// - 异步优先：所有 IO 操作都是 async
// - 可观测：所有操作都有 tracing 埋点
// - 归一化：所有分数都归一化到 [0, 1]
// =============================================================================

pub mod document;
pub mod chunker;
pub mod embedding;
pub mod index;
pub mod retriever;
pub mod reranker;

// ── 重导出 ────────────────────────────────────────────────────────────────

pub use document::{Document, DocumentMetadata, DocumentChunk, ChunkMetadata};
pub use chunker::{Chunker, ChunkingStrategy, FixedSizeChunker, RecursiveChunker};
pub use embedding::{EmbeddingProvider, EmbeddingResult, MockEmbeddingProvider};
pub use index::{VectorIndex, IndexConfig, InMemoryVectorIndex, SearchResult};
pub use retriever::{Retriever, RetrievalQuery, RetrievalResult, HybridRetriever};
pub use reranker::{Reranker, RerankResult, CrossEncoderReranker, MockReranker};

// ── Crate 元数据 ──────────────────────────────────────────────────────────

pub const CRATE_ID: &str = "mox-cloud-kb-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 知识库配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    /// 知识库 ID
    pub kb_id: String,
    /// 知识库名称
    pub name: String,
    /// 分块策略
    pub chunking_strategy: ChunkingStrategy,
    /// 向量维度
    pub embedding_dim: usize,
    /// 检索时返回的候选数量
    pub top_k: usize,
    /// 重排序后返回的最终数量
    pub final_k: usize,
    /// 是否启用重排序
    pub enable_rerank: bool,
    /// 语义检索权重（RRF 融合用）
    pub semantic_weight: f64,
    /// 关键词检索权重（RRF 融合用）
    pub keyword_weight: f64,
}

impl Default for KnowledgeBaseConfig {
    fn default() -> Self {
        Self {
            kb_id: "default".to_string(),
            name: "默认知识库".to_string(),
            chunking_strategy: ChunkingStrategy::FixedSize { chunk_size: 512, overlap: 64 },
            embedding_dim: 1536,
            top_k: 20,
            final_k: 5,
            enable_rerank: true,
            semantic_weight: 0.7,
            keyword_weight: 0.3,
        }
    }
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_valid() {
        let config = KnowledgeBaseConfig::default();
        assert_eq!(config.embedding_dim, 1536);
        assert_eq!(config.top_k, 20);
        assert!(config.enable_rerank);
        assert!((config.semantic_weight + config.keyword_weight - 1.0).abs() < f64::EPSILON);
    }
}
