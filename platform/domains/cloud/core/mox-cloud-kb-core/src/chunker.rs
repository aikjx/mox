// =============================================================================
// 分块器（Chunker）
// =============================================================================

use crate::document::{Document, DocumentChunk};
use serde::{Deserialize, Serialize};

// =============================================================================
// 分块策略
// =============================================================================

/// 分块策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkingStrategy {
    /// 固定大小分块
    FixedSize {
        /// 分块大小（字符数）
        chunk_size: usize,
        /// 重叠大小（字符数）
        overlap: usize,
    },
    /// 递归分块（按段落→句子→词递归切分）
    Recursive {
        /// 目标分块大小
        chunk_size: usize,
        /// 重叠大小
        overlap: usize,
        /// 分隔符优先级
        separators: Vec<String>,
    },
    /// 语义分块（按语义相似度切分，需要 embedding）
    Semantic {
        /// 目标分块大小
        chunk_size: usize,
        /// 语义相似度阈值（低于此值则切分）
        similarity_threshold: f64,
    },
}

impl Default for ChunkingStrategy {
    fn default() -> Self {
        ChunkingStrategy::FixedSize {
            chunk_size: 512,
            overlap: 64,
        }
    }
}

// =============================================================================
// 分块器 trait
// =============================================================================

/// 分块器 trait
pub trait Chunker: Send + Sync {
    /// 对文档进行分块
    fn chunk(&self, document: &Document) -> Vec<DocumentChunk>;

    /// 获取分块策略
    fn strategy(&self) -> ChunkingStrategy;
}

// =============================================================================
// 固定大小分块器
// =============================================================================

/// 固定大小分块器
pub struct FixedSizeChunker {
    chunk_size: usize,
    overlap: usize,
}

impl FixedSizeChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size: chunk_size.max(64),
            overlap: overlap.min(chunk_size / 2),
        }
    }
}

impl Chunker for FixedSizeChunker {
    fn chunk(&self, document: &Document) -> Vec<DocumentChunk> {
        let content = &document.content;
        let chars: Vec<char> = content.chars().collect();
        let total = chars.len();

        if total == 0 {
            return vec![];
        }

        let mut chunks: Vec<DocumentChunk> = Vec::new();
        let mut start = 0;
        let mut index = 0u32;

        while start < total {
            let end = (start + self.chunk_size).min(total);
            let chunk_content: String = chars[start..end].iter().collect();
            let mut chunk = DocumentChunk::new(
                document.id,
                document.kb_id.clone(),
                index,
                chunk_content,
                start,
                end,
            );
            chunk.metadata.char_count = end - start;

            // 链接前后分块
            if index > 0 {
                chunk.metadata.prev_chunk_id = Some(chunks[index as usize - 1].id);
            }

            chunks.push(chunk);

            if end >= total {
                break;
            }

            start = end - self.overlap;
            index += 1;
        }

        // 设置 next_chunk_id
        for i in 0..chunks.len().saturating_sub(1) {
            chunks[i].metadata.next_chunk_id = Some(chunks[i + 1].id);
        }

        chunks
    }

    fn strategy(&self) -> ChunkingStrategy {
        ChunkingStrategy::FixedSize {
            chunk_size: self.chunk_size,
            overlap: self.overlap,
        }
    }
}

// =============================================================================
// 递归分块器
// =============================================================================

/// 递归分块器
///
/// 按分隔符优先级递归切分：先按大分隔符（段落），如果块太大，
/// 再按小分隔符（句子），直到满足目标大小。
pub struct RecursiveChunker {
    chunk_size: usize,
    overlap: usize,
    separators: Vec<String>,
}

impl RecursiveChunker {
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            chunk_size: chunk_size.max(64),
            overlap: overlap.min(chunk_size / 2),
            separators: vec![
                "\n\n".to_string(),  // 段落
                "\n".to_string(),    // 行
                "。".to_string(),    // 中文句号
                "！".to_string(),    // 中文感叹
                "？".to_string(),    // 中文问号
                ". ".to_string(),    // 英文句号
                "! ".to_string(),    // 英文感叹
                "? ".to_string(),    // 英文问号
                " ".to_string(),     // 空格
                "".to_string(),      // 字符
            ],
        }
    }

    fn split_text(&self, text: &str, separator: &str) -> Vec<String> {
        if separator.is_empty() {
            return text.chars().map(|c| c.to_string()).collect();
        }
        text.split(separator)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    fn recursive_split(&self, text: &str, sep_idx: usize) -> Vec<String> {
        if text.chars().count() <= self.chunk_size || sep_idx >= self.separators.len() {
            return vec![text.to_string()];
        }

        let separator = &self.separators[sep_idx];
        let parts = self.split_text(text, separator);

        if parts.len() <= 1 {
            return self.recursive_split(text, sep_idx + 1);
        }

        let mut result = Vec::new();
        let mut current = String::new();

        for part in parts {
            if current.chars().count() + part.chars().count() > self.chunk_size && !current.is_empty() {
                result.push(current.clone());
                // 重叠：保留最后一部分
                if self.overlap > 0 {
                    let chars: Vec<char> = current.chars().collect();
                    let overlap_start = chars.len().saturating_sub(self.overlap);
                    current = chars[overlap_start..].iter().collect();
                } else {
                    current.clear();
                }
            }
            if !current.is_empty() && !separator.is_empty() {
                current.push_str(separator);
            }
            current.push_str(&part);
        }

        if !current.is_empty() {
            result.push(current);
        }

        // 对仍然太大的块继续递归切分
        result
            .into_iter()
            .flat_map(|chunk| {
                if chunk.chars().count() > self.chunk_size * 2 {
                    self.recursive_split(&chunk, sep_idx + 1)
                } else {
                    vec![chunk]
                }
            })
            .collect()
    }
}

impl Chunker for RecursiveChunker {
    fn chunk(&self, document: &Document) -> Vec<DocumentChunk> {
        let pieces = self.recursive_split(&document.content, 0);
        let mut chunks = Vec::new();
        let mut offset = 0usize;

        for (index, piece) in pieces.iter().enumerate() {
            let start = offset;
            let end = start + piece.chars().count();
            let mut chunk = DocumentChunk::new(
                document.id,
                document.kb_id.clone(),
                index as u32,
                piece.clone(),
                start,
                end,
            );
            chunk.metadata.char_count = piece.chars().count();
            chunks.push(chunk);
            offset = end;
        }

        // 链接前后分块
        for i in 0..chunks.len() {
            if i > 0 {
                chunks[i].metadata.prev_chunk_id = Some(chunks[i - 1].id);
            }
            if i + 1 < chunks.len() {
                chunks[i].metadata.next_chunk_id = Some(chunks[i + 1].id);
            }
        }

        chunks
    }

    fn strategy(&self) -> ChunkingStrategy {
        ChunkingStrategy::Recursive {
            chunk_size: self.chunk_size,
            overlap: self.overlap,
            separators: self.separators.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_doc(content: &str) -> Document {
        Document::new("kb1", "测试文档", content)
    }

    #[test]
    fn fixed_size_chunker_basic() {
        let doc = make_test_doc(&"a".repeat(1000));
        let chunker = FixedSizeChunker::new(300, 50);
        let chunks = chunker.chunk(&doc);

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].metadata.char_count, 300);
        assert!(chunks.last().unwrap().metadata.char_count <= 300);

        // 验证前后链接
        assert!(chunks[0].metadata.prev_chunk_id.is_none());
        assert!(chunks[0].metadata.next_chunk_id.is_some());
        assert!(chunks.last().unwrap().metadata.next_chunk_id.is_none());
    }

    #[test]
    fn fixed_size_chunker_empty() {
        let doc = make_test_doc("");
        let chunker = FixedSizeChunker::new(300, 50);
        let chunks = chunker.chunk(&doc);
        assert!(chunks.is_empty());
    }

    #[test]
    fn fixed_size_chunker_small() {
        let doc = make_test_doc("小文本");
        let chunker = FixedSizeChunker::new(300, 50);
        let chunks = chunker.chunk(&doc);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].metadata.char_count, 3);
    }

    #[test]
    fn recursive_chunker_paragraphs() {
        let content = "第一段。\n\n第二段。\n\n第三段。";
        let doc = make_test_doc(content);
        let chunker = RecursiveChunker::new(100, 0);
        let chunks = chunker.chunk(&doc);

        assert!(!chunks.is_empty());
        // 验证偏移量连续
        for i in 1..chunks.len() {
            assert_eq!(chunks[i].metadata.start_offset, chunks[i - 1].metadata.end_offset);
        }
    }

    #[test]
    fn chunking_strategy_default() {
        let strategy = ChunkingStrategy::default();
        match strategy {
            ChunkingStrategy::FixedSize { chunk_size, overlap } => {
                assert_eq!(chunk_size, 512);
                assert_eq!(overlap, 64);
            }
            _ => panic!("默认策略应为 FixedSize"),
        }
    }
}
