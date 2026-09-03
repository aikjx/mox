// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! 图谱挂图器：把文档/分块/实体/关系落为 kg-storage-svc 的节点边
//!
//! 节点命名空间（避免跨文档冲突）：
//! - Document：`kb-{docId}`
//! - Chunk：`kb-{docId}-chunk-{i}`
//! - Entity：`kb-{docId}-ent-{i}`
//!
//! 边：
//! - Document → Chunk：`contains`
//! - Document → Entity：`mentions`
//! - Entity → Entity：`relates`（来自分析产出的共现关系）

use crate::model::{KbDocument, STATUS_LINKED};
use mox_kg_storage_svc::{GraphEdge, GraphNode, GraphStore};
use serde_json::json;

/// 挂图结果
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkResult {
    pub doc_id: String,
    pub status: String,
    pub nodes_added: usize,
    pub edges_added: usize,
    pub graph_nodes: usize,
    pub graph_edges: usize,
    pub node_types: std::collections::BTreeMap<String, usize>,
}

/// 图谱挂图器
#[derive(Clone)]
pub struct GraphLinker;

/// 文档节点 id 命名空间
pub fn doc_node_id(doc_id: &str) -> String {
    format!("kb-{doc_id}")
}

impl GraphLinker {
    /// 挂图：重建文档子图（幂等：节点已存在则更新）
    pub fn link(&self, graph: &GraphStore, doc: &KbDocument, chunks: &[String]) -> LinkResult {
        // 1. Document 节点
        upsert_node(
            graph,
            &doc_node_id(&doc.id),
            "document",
            &doc.title,
            json!({
                "category": doc.category,
                "status": doc.status,
                "summary": doc.summary,
                "tags": doc.tags,
                "current_version": doc.current_version,
            }),
        );

        // 2. Chunk 节点 + Document → Chunk 边
        let mut nodes_added = 1;
        let mut edges_added = 0;
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_id = format!("kb-{}-chunk-{i}", doc.id);
            upsert_node(
                graph,
                &chunk_id,
                "chunk",
                &format!("{} · 片段 {}", doc.title, i + 1),
                json!({ "content": chunk, "index": i }),
            );
            nodes_added += 1;
            upsert_edge(
                graph,
                &format!("kb-{}-e-dc-{i}", doc.id),
                &doc_node_id(&doc.id),
                &chunk_id,
                "contains",
                1.0,
            );
            edges_added += 1;
        }

        // 3. Entity 节点 + Document → Entity 边
        for ent in &doc.entities {
            let ent_id = format!("kb-{}-{}", doc.id, ent.id);
            upsert_node(
                graph,
                &ent_id,
                &ent.entity_type,
                &ent.name,
                json!({ "frequency": ent.frequency, "snippet": ent.snippet }),
            );
            nodes_added += 1;
            upsert_edge(
                graph,
                &format!("kb-{}-e-de-{}", doc.id, ent.id),
                &doc_node_id(&doc.id),
                &ent_id,
                "mentions",
                ent.frequency as f64,
            );
            edges_added += 1;
        }

        // 4. Entity → Entity 关系边
        for rel in &doc.relations {
            let src = format!("kb-{}-{}", doc.id, rel.source);
            let dst = format!("kb-{}-{}", doc.id, rel.target);
            if graph.get_node(&src).is_some() && graph.get_node(&dst).is_some() {
                upsert_edge(
                    graph,
                    &format!("kb-{}-{}", doc.id, rel.id),
                    &src,
                    &dst,
                    &rel.relation,
                    rel.weight,
                );
                edges_added += 1;
            }
        }

        // 5. 节点类型分布
        let mut node_types = std::collections::BTreeMap::new();
        for node in graph.list_nodes() {
            *node_types.entry(node.node_type).or_insert(0) += 1;
        }

        LinkResult {
            doc_id: doc.id.clone(),
            status: STATUS_LINKED.into(),
            nodes_added,
            edges_added,
            graph_nodes: graph.node_count(),
            graph_edges: graph.edge_count(),
            node_types,
        }
    }

    /// 移除文档子图（删除时反挂图）
    pub fn unlink(&self, graph: &GraphStore, doc_id: &str) -> usize {
        let doc_nid = doc_node_id(doc_id);
        let related: Vec<String> = graph
            .list_nodes()
            .into_iter()
            .filter(|n| n.id == doc_nid || n.id.starts_with(&format!("kb-{doc_id}-")))
            .map(|n| n.id)
            .collect();
        let mut removed = 0usize;
        for id in related {
            if graph.delete_node(&id).is_ok() {
                removed += 1;
            }
        }
        removed
    }
}

/// 节点 upsert：不存在则 add，存在则 update
fn upsert_node(graph: &GraphStore, id: &str, node_type: &str, label: &str, props: serde_json::Value) {
    if graph.get_node(id).is_some() {
        let _ = graph.update_node(id, Some(label), Some(props));
        return;
    }
    let mut node = GraphNode::new(id, node_type, label);
    node.properties = props;
    let _ = graph.add_node(node);
}

/// 边 upsert：存在则删后重建（保持最新权重）
fn upsert_edge(
    graph: &GraphStore,
    id: &str,
    source: &str,
    target: &str,
    edge_type: &str,
    weight: f64,
) {
    if graph.get_edge(id).is_some() {
        let _ = graph.delete_edge(id);
    }
    let mut edge = GraphEdge::new(id, source, target, edge_type);
    edge.weight = weight;
    let _ = graph.add_edge(edge);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{KbDocument, KbEntity, KbRelation};

    fn analyzed_doc() -> KbDocument {
        let mut doc = KbDocument::new("kb-1".into(), "云盘架构".into(), "内容寻址去重与纠删码".into(), "cat-tech".into());
        doc.entities = vec![
            KbEntity {
                id: "ent-0".into(),
                name: "内容寻址".into(),
                entity_type: "tech".into(),
                frequency: 3,
                snippet: "内容寻址去重".into(),
            },
            KbEntity {
                id: "ent-1".into(),
                name: "纠删码".into(),
                entity_type: "tech".into(),
                frequency: 2,
                snippet: "纠删码".into(),
            },
        ];
        doc.relations = vec![KbRelation {
            id: "rel-0-1".into(),
            source: "ent-0".into(),
            target: "ent-1".into(),
            relation: "co_occur".into(),
            weight: 0.5,
        }];
        doc.status = crate::model::STATUS_ANALYZED.into();
        doc
    }

    #[test]
    fn link_creates_document_chunk_entity_nodes_and_edges() {
        let graph = GraphStore::new();
        let doc = analyzed_doc();
        let chunks = vec!["片段一".to_string(), "片段二".to_string()];
        let result = GraphLinker.link(&graph, &doc, &chunks);
        assert_eq!(result.status, STATUS_LINKED);
        assert!(result.nodes_added >= 1 + 2 + 2); // doc + 2 chunk + 2 entity
        assert!(result.edges_added > 2 + 2); // 2 contains + 2 mentions + 1 relates
        assert_eq!(graph.node_count(), result.graph_nodes);
        assert_eq!(graph.edge_count(), result.graph_edges);

        // Document 节点可检索
        let found = graph.search("云盘架构", 10);
        assert!(found.iter().any(|n| n.id == doc_node_id(&doc.id)));
        // 节点类型分布正确
        assert_eq!(result.node_types.get("document"), Some(&1));
        assert_eq!(result.node_types.get("chunk"), Some(&2));
        assert_eq!(result.node_types.get("tech"), Some(&2));
    }

    #[test]
    fn link_is_idempotent_and_unlink_removes() {
        let graph = GraphStore::new();
        let doc = analyzed_doc();
        let chunks = vec!["片段一".to_string()];
        GraphLinker.link(&graph, &doc, &chunks);
        let before = graph.node_count();
        // 二次挂图不报错（幂等 upsert）
        GraphLinker.link(&graph, &doc, &chunks);
        assert_eq!(graph.node_count(), before, "幂等：节点数不增长");
        // 反挂图
        let removed = GraphLinker.unlink(&graph, &doc.id);
        assert!(removed >= 4);
        assert!(graph.get_node(&doc_node_id(&doc.id)).is_none());
    }
}



