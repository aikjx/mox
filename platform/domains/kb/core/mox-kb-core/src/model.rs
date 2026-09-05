// =============================================================================
// KB 数据模型
// =============================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 知识库文档
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID（UUIDv7）
    pub id: String,
    /// 文档标题
    pub title: String,
    /// 文档内容（Markdown / 纯文本）
    pub content: String,
    /// 文档类型：article / note / spec / faq / manual
    pub doc_type: String,
    /// 所属分类/标签
    pub tags: Vec<String>,
    /// 作者
    pub author: String,
    /// 版本号
    pub version: u32,
    /// 状态：draft / published / archived
    pub status: String,
    /// 创建时间（RFC3339）
    pub created_at: String,
    /// 更新时间（RFC3339）
    pub updated_at: String,
}

impl Document {
    /// 创建新文档
    pub fn new(title: impl Into<String>, content: impl Into<String>, author: impl Into<String>) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: Uuid::now_v7().to_string(),
            title: title.into(),
            content: content.into(),
            doc_type: "article".to_string(),
            tags: vec![],
            author: author.into(),
            version: 1,
            status: "draft".to_string(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// 发布文档
    pub fn publish(&mut self) {
        self.status = "published".to_string();
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    /// 增加版本
    pub fn bump_version(&mut self) {
        self.version += 1;
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

/// 文档版本记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub version: u32,
    pub title: String,
    pub content_snapshot: String,
    pub changed_by: String,
    pub change_note: String,
    pub created_at: String,
}

/// 搜索查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// 搜索关键词
    pub keyword: String,
    /// 文档类型过滤
    pub doc_type: Option<String>,
    /// 标签过滤
    pub tags: Vec<String>,
    /// 页码（从 1 开始）
    pub page: u32,
    /// 每页大小
    pub page_size: u32,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            keyword: String::new(),
            doc_type: None,
            tags: vec![],
            page: 1,
            page_size: 20,
        }
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub items: Vec<Document>,
    pub total: u64,
    pub page: u32,
    pub page_size: u32,
    /// 搜索耗时（毫秒）
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_document() {
        let doc = Document::new("测试标题", "测试内容", "tester");
        assert_eq!(doc.title, "测试标题");
        assert_eq!(doc.version, 1);
        assert_eq!(doc.status, "draft");
        assert!(!doc.id.is_empty());
    }

    #[test]
    fn test_publish_document() {
        let mut doc = Document::new("t", "c", "a");
        doc.publish();
        assert_eq!(doc.status, "published");
    }

    #[test]
    fn test_bump_version() {
        let mut doc = Document::new("t", "c", "a");
        doc.bump_version();
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_search_query_default() {
        let q = SearchQuery::default();
        assert_eq!(q.page, 1);
        assert_eq!(q.page_size, 20);
    }
}
