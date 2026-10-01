// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库检索器：文档关键词检索（标题加权）+ 图谱检索（节点命中）
//!
//! 评分：标题命中 3 分 / 标签命中 2 分 / 分类命中 1 分 / 正文命中 1 分，按分倒序截断。

use crate::{
    model::{KbDocument, SearchHit, SearchRequest, SourceCitation},
    KbState,
};
use mox_kg_storage_svc::GraphStore;
use serde_json::Value;

/// 检索器
#[derive(Clone)]
pub struct KbSearcher;

impl KbSearcher {
    /// 文档关键词检索
    pub async fn search_docs(
        &self,
        state: &KbState,
        req: &SearchRequest,
    ) -> crate::Result<Vec<SearchHit>> {
        let index = state.docs.read_index().await?;
        let q = req.query.trim().to_lowercase();
        let mut hits = Vec::new();
        for summary in index {
            if let Some(cat) = &req.category {
                if &summary.category != cat {
                    continue;
                }
            }
            let Ok(doc) = state.docs.get(&summary.id).await else {
                continue;
            };
            let mut score = 0.0_f64;
            if doc.title.to_lowercase().contains(&q) {
                score += 3.0;
            }
            if doc.content.to_lowercase().contains(&q) {
                score += 1.0;
            }
            if doc.category.to_lowercase().contains(&q) {
                score += 1.0;
            }
            for t in &doc.tags {
                if t.to_lowercase().contains(&q) {
                    score += 2.0;
                }
            }
            if score > 0.0 {
                let (snippet, field, start_char, end_char) = build_snippet(&doc, &q);
                hits.push(SearchHit {
                    id: doc.id.clone(),
                    title: doc.title.clone(),
                    category: doc.category.clone(),
                    snippet,
                    citation: Some(SourceCitation {
                        document_id: doc.id.clone(),
                        version: doc.current_version.clone(),
                        field: field.into(),
                        start_char,
                        end_char,
                    }),
                    score,
                    tags: doc.tags.clone(),
                });
            }
        }
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        hits.truncate(req.limit);
        Ok(hits)
    }

    /// 图谱检索（节点标签/属性命中）
    pub fn search_graph(graph: &GraphStore, query: &str, limit: usize) -> Vec<Value> {
        graph
            .search(query, limit)
            .into_iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "node_type": n.node_type,
                    "label": n.label,
                    "properties": n.properties,
                })
            })
            .collect()
    }
}

/// 构建命中片段：正文首现处附近 60 字符
fn build_snippet(doc: &KbDocument, q: &str) -> (String, &'static str, usize, usize) {
    if let Some(byte_idx) = doc.content.to_lowercase().find(q) {
        // Lowercase may expand characters (for example U+0130). Its byte offsets
        // cannot be used to slice the original text. Map to original characters.
        let mut lower_end = 0usize;
        let mut char_idx = 0usize;
        for (index, ch) in doc.content.chars().enumerate() {
            lower_end += ch.to_lowercase().map(char::len_utf8).sum::<usize>();
            if byte_idx < lower_end {
                char_idx = index;
                break;
            }
        }
        let chars: Vec<char> = doc.content.chars().collect();
        let start = char_idx.saturating_sub(20);
        let end = (char_idx + q.chars().count() + 40).min(chars.len());
        let snippet: String = chars[start..end].iter().collect();
        if !snippet.is_empty() {
            return (snippet, "content", start, end);
        }
    }
    // Title is retained in content version snapshots; generated summary is not.
    let snippet: String = doc.title.chars().take(80).collect();
    let end = snippet.chars().count();
    (snippet, "title", 0, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::kb_state;

    #[tokio::test]
    async fn search_ranks_title_over_content() {
        let state = kb_state();
        state
            .docs
            .create("云盘存储架构", "内容寻址去重与纠删码详解", Some("cat-tech"))
            .await
            .unwrap();
        state
            .docs
            .create("数据库优化", "云盘查询性能优化实战", Some("cat-tech"))
            .await
            .unwrap();
        let hits = KbSearcher
            .search_docs(&state, &SearchRequest { query: "云盘".into(), limit: 10, category: None })
            .await
            .unwrap();
        assert_eq!(hits.len(), 2);
        // 标题命中（云盘存储架构）应排第一
        assert_eq!(hits[0].title, "云盘存储架构");
        assert!(hits[0].score > hits[1].score);
    }

    #[tokio::test]
    async fn search_category_filter() {
        let state = kb_state();
        state
            .docs
            .create("业务文档", "云盘存储采购方案", Some("cat-business"))
            .await
            .unwrap();
        let hits = KbSearcher
            .search_docs(
                &state,
                &SearchRequest {
                    query: "云盘".into(),
                    limit: 10,
                    category: Some("cat-tech".into()),
                },
            )
            .await
            .unwrap();
        assert!(hits.is_empty(), "分类过滤应排除 business 文档");
    }
}
