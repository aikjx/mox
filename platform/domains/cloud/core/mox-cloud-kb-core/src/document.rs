// =============================================================================
// 文档模型（Document Model）
// =============================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

// =============================================================================
// 文档实体
// =============================================================================

/// 文档实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// 文档 ID
    pub id: Uuid,
    /// 知识库 ID
    pub kb_id: String,
    /// 文档标题
    pub title: String,
    /// 文档内容（纯文本）
    pub content: String,
    /// 文档来源（url/file_path/manual）
    pub source: String,
    /// 文档类型（pdf/md/txt/html/...）
    pub doc_type: String,
    /// 文档元数据
    #[serde(default)]
    pub metadata: DocumentMetadata,
    /// 自定义属性
    #[serde(default)]
    pub custom_fields: BTreeMap<String, String>,
    /// 文档标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 版本号
    pub version: u32,
    /// 文档状态
    pub status: DocumentStatus,
    /// 分块数量
    pub chunk_count: u32,
    /// 字符数
    pub char_count: usize,
    /// 词数
    pub token_count: Option<u32>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 最后索引时间
    pub indexed_at: Option<DateTime<Utc>>,
}

/// 文档状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// 待处理
    Pending,
    /// 处理中（分块/嵌入中）
    Processing,
    /// 已索引
    Indexed,
    /// 处理失败
    Failed,
    /// 已归档
    Archived,
}

impl Default for DocumentStatus {
    fn default() -> Self {
        DocumentStatus::Pending
    }
}

impl Document {
    /// 创建新文档
    pub fn new(kb_id: impl Into<String>, title: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let char_count = content.chars().count();
        Self {
            id: Uuid::new_v4(),
            kb_id: kb_id.into(),
            title: title.into(),
            content,
            source: "manual".to_string(),
            doc_type: "txt".to_string(),
            metadata: DocumentMetadata::default(),
            custom_fields: BTreeMap::new(),
            tags: vec![],
            version: 1,
            status: DocumentStatus::Pending,
            chunk_count: 0,
            char_count,
            token_count: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            indexed_at: None,
        }
    }

    /// 标记为已索引
    pub fn mark_indexed(&mut self, chunk_count: u32) {
        self.status = DocumentStatus::Indexed;
        self.chunk_count = chunk_count;
        self.indexed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// 标记为处理失败
    pub fn mark_failed(&mut self, error: impl Into<String>) {
        self.status = DocumentStatus::Failed;
        self.metadata.index_error = Some(error.into());
        self.updated_at = Utc::now();
    }
}

// =============================================================================
// 文档元数据
// =============================================================================

/// 文档元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetadata {
    /// 作者
    pub author: Option<String>,
    /// 创建日期（原始文档）
    pub original_date: Option<String>,
    /// 来源 URL
    pub source_url: Option<String>,
    /// 文件路径
    pub file_path: Option<String>,
    /// 文件大小（字节）
    pub file_size: Option<u64>,
    /// 语言
    pub language: Option<String>,
    /// 索引错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_error: Option<String>,
    /// 处理耗时（毫秒）
    pub processing_ms: Option<u64>,
}

// =============================================================================
// 文档分块
// =============================================================================

/// 文档分块
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// 分块 ID
    pub id: Uuid,
    /// 文档 ID
    pub document_id: Uuid,
    /// 知识库 ID
    pub kb_id: String,
    /// 分块序号（从 0 开始）
    pub chunk_index: u32,
    /// 分块内容
    pub content: String,
    /// 分块字符数
    pub char_count: usize,
    /// 分块元数据
    pub metadata: ChunkMetadata,
    /// 向量嵌入（索引时填充）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 分块元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// 在原文中的起始字符位置
    pub start_offset: usize,
    /// 在原文中的结束字符位置
    pub end_offset: usize,
    /// 分块字符数
    pub char_count: usize,
    /// 分块词数
    pub token_count: Option<u32>,
    /// 分块标题（从上下文提取）
    pub section_title: Option<String>,
    /// 分块层级（标题层级）
    pub heading_level: Option<u8>,
    /// 前一个分块 ID（用于上下文窗口）
    pub prev_chunk_id: Option<Uuid>,
    /// 后一个分块 ID
    pub next_chunk_id: Option<Uuid>,
}

impl DocumentChunk {
    /// 创建新分块
    pub fn new(
        document_id: Uuid,
        kb_id: impl Into<String>,
        chunk_index: u32,
        content: impl Into<String>,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        let content = content.into();
        Self {
            id: Uuid::new_v4(),
            document_id,
            kb_id: kb_id.into(),
            chunk_index,
            char_count: content.chars().count(),
            content,
            metadata: ChunkMetadata {
                start_offset,
                end_offset,
                char_count: 0,
                token_count: None,
                section_title: None,
                heading_level: None,
                prev_chunk_id: None,
                next_chunk_id: None,
            },
            embedding: None,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_creation() {
        let doc = Document::new("kb1", "测试文档", "这是测试内容");
        assert_eq!(doc.title, "测试文档");
        assert_eq!(doc.char_count, 6);
        assert_eq!(doc.status, DocumentStatus::Pending);
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn document_mark_indexed() {
        let mut doc = Document::new("kb1", "测试", "内容");
        doc.mark_indexed(5);
        assert_eq!(doc.status, DocumentStatus::Indexed);
        assert_eq!(doc.chunk_count, 5);
        assert!(doc.indexed_at.is_some());
    }

    #[test]
    fn document_chunk_creation() {
        let doc_id = Uuid::new_v4();
        let chunk = DocumentChunk::new(doc_id, "kb1", 0, "分块内容", 0, 4);
        assert_eq!(chunk.chunk_index, 0);
        assert_eq!(chunk.metadata.start_offset, 0);
        assert_eq!(chunk.metadata.end_offset, 4);
        assert!(chunk.embedding.is_none());
    }

    #[test]
    fn document_status_serialization() {
        let json = serde_json::to_string(&DocumentStatus::Indexed).unwrap();
        assert_eq!(json, "\"indexed\"");
        let parsed: DocumentStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(parsed, DocumentStatus::Failed);
    }
}
