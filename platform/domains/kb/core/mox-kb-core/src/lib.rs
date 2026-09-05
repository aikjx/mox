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
    use std::sync::Mutex;

    #[test]
    fn test_crate_metadata() {
        assert_eq!(CRATE_ID, "mox-kb-core");
        assert!(!CRATE_VERSION.is_empty());
    }

    // ── Mock 存储实现 ──────────────────────────────────────────────────────

    struct MockKbStore {
        docs: Mutex<Vec<Document>>,
        deleted: Mutex<Vec<String>>,
    }

    impl MockKbStore {
        fn new() -> Self {
            Self {
                docs: Mutex::new(vec![]),
                deleted: Mutex::new(vec![]),
            }
        }

        fn with_docs(docs: Vec<Document>) -> Self {
            Self {
                docs: Mutex::new(docs),
                deleted: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl KbStore for MockKbStore {
        async fn save_document(&self, doc: &Document) -> KbResult<()> {
            let mut docs = self.docs.lock().unwrap();
            if let Some(existing) = docs.iter_mut().find(|d| d.id == doc.id) {
                *existing = doc.clone();
            } else {
                docs.push(doc.clone());
            }
            Ok(())
        }

        async fn get_document(&self, doc_id: &str) -> KbResult<Option<Document>> {
            let docs = self.docs.lock().unwrap();
            Ok(docs.iter().find(|d| d.id == doc_id).cloned())
        }

        async fn search_documents(&self, query: &SearchQuery) -> KbResult<SearchResult> {
            let docs = self.docs.lock().unwrap();
            let filtered: Vec<Document> = docs
                .iter()
                .filter(|d| {
                    if !query.keyword.is_empty() {
                        d.title.contains(&query.keyword) || d.content.contains(&query.keyword)
                    } else {
                        true
                    }
                })
                .filter(|d| {
                    if let Some(dt) = &query.doc_type {
                        d.doc_type == *dt
                    } else {
                        true
                    }
                })
                .cloned()
                .collect();
            let total = filtered.len() as u64;
            let start = ((query.page - 1) * query.page_size) as usize;
            let items: Vec<Document> = filtered
                .into_iter()
                .skip(start)
                .take(query.page_size as usize)
                .collect();
            Ok(SearchResult {
                items,
                total,
                page: query.page,
                page_size: query.page_size,
                duration_ms: 0,
            })
        }

        async fn delete_document(&self, doc_id: &str) -> KbResult<()> {
            let mut docs = self.docs.lock().unwrap();
            docs.retain(|d| d.id != doc_id);
            self.deleted.lock().unwrap().push(doc_id.to_string());
            Ok(())
        }

        async fn list_versions(&self, _doc_id: &str) -> KbResult<Vec<DocumentVersion>> {
            Ok(vec![])
        }
    }

    // ── KbManager 集成测试 ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_create_and_get_document() {
        let store = Box::new(MockKbStore::new());
        let manager = KbManager::new(store);

        let doc = Document::new("测试文档", "测试内容", "tester");
        let created = manager.create_document(doc.clone()).await.unwrap();
        assert_eq!(created.id, doc.id);
        assert_eq!(created.title, "测试文档");

        let fetched = manager.get_document(&doc.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "测试文档");
    }

    #[tokio::test]
    async fn test_get_nonexistent_document() {
        let store = Box::new(MockKbStore::new());
        let manager = KbManager::new(store);
        let result = manager.get_document("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_documents() {
        let docs = vec![
            Document::new("Rust 编程", "Rust 语言教程", "a1"),
            Document::new("Python 编程", "Python 语言教程", "a2"),
            Document::new("Rust 高级", "Rust 高级特性", "a3"),
        ];
        let store = Box::new(MockKbStore::with_docs(docs));
        let manager = KbManager::new(store);

        // 搜索 "Rust" 应返回 2 个结果
        let query = SearchQuery {
            keyword: "Rust".to_string(),
            ..Default::default()
        };
        let result = manager.search(&query).await.unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.items.len(), 2);

        // 搜索 "Python" 应返回 1 个结果
        let query = SearchQuery {
            keyword: "Python".to_string(),
            ..Default::default()
        };
        let result = manager.search(&query).await.unwrap();
        assert_eq!(result.total, 1);
    }

    #[tokio::test]
    async fn test_delete_document() {
        let doc = Document::new("待删除", "内容", "tester");
        let store = Box::new(MockKbStore::with_docs(vec![doc.clone()]));
        let manager = KbManager::new(store);

        manager.delete_document(&doc.id).await.unwrap();
        let result = manager.get_document(&doc.id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_search_pagination() {
        let docs: Vec<Document> = (0..25)
            .map(|i| Document::new(format!("文档{i}"), format!("内容{i}"), "tester"))
            .collect();
        let store = Box::new(MockKbStore::with_docs(docs));
        let manager = KbManager::new(store);

        // 第一页（page_size=10）
        let query = SearchQuery {
            page: 1,
            page_size: 10,
            ..Default::default()
        };
        let result = manager.search(&query).await.unwrap();
        assert_eq!(result.total, 25);
        assert_eq!(result.items.len(), 10);

        // 第三页（应返回 5 个）
        let query = SearchQuery {
            page: 3,
            page_size: 10,
            ..Default::default()
        };
        let result = manager.search(&query).await.unwrap();
        assert_eq!(result.items.len(), 5);
    }
}
