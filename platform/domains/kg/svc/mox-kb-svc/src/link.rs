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
    /// 完整子图先验证后一次发布，来源文档与版本贯穿所有投影。
    pub fn link(
        &self,
        graph: &GraphStore,
        doc: &KbDocument,
        chunks: &[String],
    ) -> Result<LinkResult, mox_kg_storage_svc::StorageError> {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let document = doc_node_id(&doc.id);
        let node = |id: &str, kind: &str, label: &str, mut props: serde_json::Value| {
            props["source_doc_id"] = json!(doc.id);
            props["source_version"] = json!(doc.current_version);
            GraphNode::new(id, kind, label).with_properties(props)
        };
        let edge = |id: &str, source: &str, target: &str, kind: &str, weight: f64| {
            let mut edge = GraphEdge::new(id, source, target, kind).with_weight(weight);
            edge.properties = json!({"source_doc_id":doc.id,"source_version":doc.current_version});
            edge
        };
        nodes.push(node(&document,"document",&doc.title,json!({"category":doc.category,"status":STATUS_LINKED,"summary":doc.summary,"tags":doc.tags,"current_version":doc.current_version})));
        for (i, chunk) in chunks.iter().enumerate() {
            let id = format!("kb-{}-chunk-{i}", doc.id);
            nodes.push(node(
                &id,
                "chunk",
                &format!("{} · 片段 {}", doc.title, i + 1),
                json!({"content":chunk,"index":i}),
            ));
            edges.push(edge(&format!("kb-{}-e-dc-{i}", doc.id), &document, &id, "contains", 1.0));
        }
        for entity in &doc.entities {
            let id = format!("kb-{}-{}", doc.id, entity.id);
            nodes.push(node(
                &id,
                &entity.entity_type,
                &entity.name,
                json!({"frequency":entity.frequency,"snippet":entity.snippet}),
            ));
            edges.push(edge(
                &format!("kb-{}-e-de-{}", doc.id, entity.id),
                &document,
                &id,
                "mentions",
                entity.frequency as f64,
            ));
        }
        for relation in &doc.relations {
            edges.push(edge(
                &format!("kb-{}-{}", doc.id, relation.id),
                &format!("kb-{}-{}", doc.id, relation.source),
                &format!("kb-{}-{}", doc.id, relation.target),
                &relation.relation,
                relation.weight,
            ));
        }
        let nodes_added = nodes.len();
        let edges_added = edges.len();
        graph.replace_document_projection(&doc.id, nodes, edges)?;
        let snapshot = graph.snapshot();
        let mut node_types = std::collections::BTreeMap::new();
        for node in snapshot.nodes {
            *node_types.entry(node.node_type).or_insert(0) += 1;
        }
        Ok(LinkResult {
            doc_id: doc.id.clone(),
            status: STATUS_LINKED.into(),
            nodes_added,
            edges_added,
            graph_nodes: snapshot.node_count,
            graph_edges: snapshot.edge_count,
            node_types,
        })
    }
    pub fn unlink(
        &self,
        graph: &GraphStore,
        doc_id: &str,
    ) -> Result<usize, mox_kg_storage_svc::StorageError> {
        graph.replace_document_projection(doc_id, Vec::new(), Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{KbDocument, KbEntity, KbRelation};

    fn analyzed_doc() -> KbDocument {
        let mut doc = KbDocument::new(
            "kb-1".into(),
            "云盘架构".into(),
            "内容寻址去重与纠删码".into(),
            "cat-tech".into(),
        );
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
        let result = GraphLinker.link(&graph, &doc, &chunks).unwrap();
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
        GraphLinker.link(&graph, &doc, &chunks).unwrap();
        let before = graph.node_count();
        // 二次挂图不报错（幂等 upsert）
        GraphLinker.link(&graph, &doc, &chunks).unwrap();
        assert_eq!(graph.node_count(), before, "幂等：节点数不增长");
        // 反挂图
        let removed = GraphLinker.unlink(&graph, &doc.id).unwrap();
        assert!(removed >= 4);
        assert!(graph.get_node(&doc_node_id(&doc.id)).is_none());
    }
    #[test]
    fn relink_removes_obsolete_chunks_entities_and_evidence() {
        let graph = GraphStore::new();
        let mut doc = analyzed_doc();
        GraphLinker.link(&graph, &doc, &["first".into(), "obsolete".into()]).unwrap();
        doc.entities.clear();
        doc.relations.clear();
        GraphLinker.link(&graph, &doc, &["new".into()]).unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.search("obsolete", 10).is_empty());
        let node = graph.get_node(&format!("kb-{}-chunk-0", doc.id)).unwrap();
        assert_eq!(node.properties["content"], "new");
        assert_eq!(node.properties["source_doc_id"], doc.id);
        assert_eq!(node.properties["source_version"], doc.current_version);
    }
}
