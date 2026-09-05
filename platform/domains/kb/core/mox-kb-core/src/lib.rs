// =============================================================================
// mox-kb-core: 知识库核心（独立域归一化）
// =============================================================================
//
// 从 kg/svc/mox-kb-svc 与 cloud/core/mox-cloud-kb-core 归一化迁出。
// 提供：文档管理 / 版本控制 / 全文检索 / 知识分析 / 关联链接 / 专家门禁
//
// 设计原则：
// - 复用 base 域统一基座（model/store/query）
// - 与 KG 解耦：KB 独立域，通过 SDK 与 KG 交互
// - 与 Cloud 解耦：文件存储通过抽象 trait，不直接依赖 cloud-kernel
// =============================================================================

pub mod document;
pub mod version;
pub mod search;
pub mod analyze;
pub mod link;
pub mod model;
pub mod error;

pub use error::{KbError, KbResult};
pub use model::*;

pub const CRATE_ID: &str = "mox-kb-core";
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

use async_trait::async_trait;

/// 知识库存储 trait（抽象，可接入 SQLite/PostgreSQL/Cloud）
#[async_trait]
pub trait KbStore: Send + Sync {
    /// 保存文档
    async fn save_document(&self, doc: &Document) -> KbResult<()>;
    /// 获取文档
    async fn get_document(&self, doc_id: &str) -> KbResult<Option<Document>>;
    /// 搜索文档
    async fn search_documents(&self, query: &SearchQuery) -> KbResult<SearchResult>;
    /// 删除文档（软删除）
    async fn delete_document(&self, doc_id: &str) -> KbResult<()>;
    /// 列出文档版本
    async fn list_versions(&self, doc_id: &str) -> KbResult<Vec<DocumentVersion>>;
}

/// 知识库管理器（高层 API）
pub struct KbManager {
    store: Box<dyn KbStore>,
}

impl KbManager {
    /// 创建知识库管理器
    pub fn new(store: Box<dyn KbStore>) -> Self {
        Self { store }
    }

    /// 创建文档
    pub async fn create_document(&self, doc: Document) -> KbResult<Document> {
        self.store.save_document(&doc).await?;
        tracing::info!(doc_id = %doc.id, title = %doc.title, "知识库文档已创建");
        Ok(doc)
    }

    /// 获取文档
    pub async fn get_document(&self, doc_id: &str) -> KbResult<Option<Document>> {
        self.store.get_document(doc_id).await
    }

    /// 搜索文档
    pub async fn search(&self, query: &SearchQuery) -> KbResult<SearchResult> {
        self.store.search_documents(query).await
    }

    /// 删除文档
    pub async fn delete_document(&self, doc_id: &str) -> KbResult<()> {
        self.store.delete_document(doc_id).await?;
        tracing::info!(doc_id = %doc_id, "知识库文档已删除（软删除）");
        Ok(())
    }

    /// 列出文档版本历史
    pub async fn list_versions(&self, doc_id: &str) -> KbResult<Vec<DocumentVersion>> {
        self.store.list_versions(doc_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_metadata() {
        assert_eq!(CRATE_ID, "mox-kb-core");
        assert!(!CRATE_VERSION.is_empty());
    }
}
