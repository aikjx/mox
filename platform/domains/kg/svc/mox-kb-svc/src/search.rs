// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 知识库检索器：文档关键词检索（标题加权）+ 图谱检索（节点命中）
//!
//! 评分：标题命中 3 分 / 标签命中 2 分 / 分类命中 1 分 / 正文命中 1 分，按分倒序截断。

use crate::model::{KbDocument, SearchHit, SearchRequest};
use crate::KbState;
use mox_kg_storage_svc::GraphStore;
use serde_json::Value;

/// 检索器
#[derive(Clone)]
pub struct KbSearcher;

impl KbSearcher {
    /// 文档关键词检索
    pub async fn search_docs(&self, state: &KbState, req: &SearchRequest) -> crate::Result<Vec<SearchHit>> {
        let index = state.docs.read_index().await?;
        let q = req.query.to_lowercase();
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
            let mut matched_field = String::new();
            if doc.title.to_lowercase().contains(&q) {
                score += 3.0;
                matched_field = "title".into();
            }
            if doc.content.to_lowercase().contains(&q) {
                score += 1.0;
                if matched_field.is_empty() {
                    matched_field = "content".into();
                }
            }
            if doc.category.to_lowercase().contains(&q) {
                score += 1.0;
            }
            for t in &doc.tags {
                if t.to_lowercase().contains(&q) {
                    score += 2.0;
                    if matched_field.is_empty() {
                        matched_field = "tag".into();
                    }
                }
            }
            if score > 0.0 {
                hits.push(SearchHit {
                    id: doc.id.clone(),
                    title: doc.title.clone(),
                    category: doc.category.clone(),
                    snippet: build_snippet(&doc, &q),
                    score,
                    tags: doc.tags.clone(),
                });
            }
        }
        hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
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
fn build_snippet(doc: &KbDocument, q: &str) -> String {
    if let Some(idx) = doc.content.to_lowercase().find(q) {
        let chars: Vec<char> = doc.content.chars().collect();
        // find 返回字节偏移；先转字符索引再切，避免多字节(中文)越界
        let char_idx = doc.content[..idx].chars().count();
        let start = char_idx.saturating_sub(20);
        let end = (char_idx + q.chars().count() + 40).min(chars.len());
        let s: String = chars[start..end].iter().collect();
        if !s.is_empty() {
            return s;
        }
    }
    if doc.summary.is_empty() {
        doc.title.clone()
    } else {
        doc.summary.clone()
    }
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

