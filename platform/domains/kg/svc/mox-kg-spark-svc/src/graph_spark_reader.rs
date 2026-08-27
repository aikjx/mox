// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Reader side: paged nodes/edges frames.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphSchemaField {
    pub name: String,
    pub data_type: String, // "Long" / "String" / "Map<String,String>" / "Double"
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GraphSchema {
    pub fields: Vec<GraphSchemaField>,
}

impl GraphSchema {
    /// Standard node schema: id, label, type_, attr map.
    pub fn standard_node() -> Self {
        Self {
            fields: vec![
                GraphSchemaField { name: "id".into(), data_type: "Long".into(), nullable: false },
                GraphSchemaField { name: "label".into(), data_type: "String".into(), nullable: false },
                GraphSchemaField { name: "type_".into(), data_type: "String".into(), nullable: false },
                GraphSchemaField {
                    name: "attr".into(),
                    data_type: "Map<String,String>".into(),
                    nullable: true,
                },
            ],
        }
    }
    pub fn standard_edge() -> Self {
        Self {
            fields: vec![
                GraphSchemaField { name: "source".into(), data_type: "Long".into(), nullable: false },
                GraphSchemaField { name: "target".into(), data_type: "Long".into(), nullable: false },
                GraphSchemaField { name: "label".into(), data_type: "String".into(), nullable: false },
                GraphSchemaField {
                    name: "props".into(),
                    data_type: "Map<String,String>".into(),
                    nullable: true,
                },
            ],
        }
    }
}

/// A single node row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeRow {
    pub id: i64,
    pub label: String,
    pub type_: String,
    pub attr: BTreeMap<String, String>,
}

/// A single edge row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct EdgeRow {
    pub source: i64,
    pub target: i64,
    pub label: String,
    pub props: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeFrame {
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub schema: GraphSchema,
    pub rows: Vec<NodeRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EdgeFrame {
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub schema: GraphSchema,
    pub rows: Vec<EdgeRow>,
}

pub struct GraphSparkReader {
    nodes: Mutex<BTreeMap<i64, NodeRow>>,
    edges: Mutex<BTreeSet<(i64, i64, String, EdgeRow)>>,
}

impl GraphSparkReader {
    pub fn new() -> Self {
        Self {
            nodes: Mutex::new(BTreeMap::new()),
            edges: Mutex::new(BTreeSet::new()),
        }
    }

    // Internal bulk load — populated via the writer (shared store in integration tests).
    pub(crate) fn load_node(&self, n: NodeRow) {
        self.nodes.lock().insert(n.id, n);
    }
    pub(crate) fn load_edge(&self, e: EdgeRow) {
        let key = (e.source, e.target, e.label.clone(), e);
        self.edges.lock().insert(key);
    }

    /// Paged nodes: 1-indexed page.
    pub fn paged_nodes(&self, page: usize, size: usize) -> NodeFrame {
        let size = size.max(1);
        let page = page.max(1);
        let n = self.nodes.lock();
        let total = n.len();
        let rows: Vec<NodeRow> = n
            .values()
            .skip((page - 1) * size)
            .take(size)
            .cloned()
            .collect();
        NodeFrame {
            page,
            page_size: size,
            total,
            schema: GraphSchema::standard_node(),
            rows,
        }
    }

    pub fn paged_edges(&self, page: usize, size: usize) -> EdgeFrame {
        let size = size.max(1);
        let page = page.max(1);
        let e = self.edges.lock();
        let total = e.len();
        let rows: Vec<EdgeRow> = e
            .iter()
            .skip((page - 1) * size)
            .take(size)
            .map(|(_, _, _, r)| r.clone())
            .collect();
        EdgeFrame {
            page,
            page_size: size,
            total,
            schema: GraphSchema::standard_edge(),
            rows,
        }
    }

    pub fn count_nodes(&self) -> usize {
        self.nodes.lock().len()
    }
    pub fn count_edges(&self) -> usize {
        self.edges.lock().len()
    }

    pub fn snapshot_nodes_set(&self) -> BTreeSet<NodeRow> {
        self.nodes.lock().values().cloned().collect()
    }
    pub fn snapshot_edges_set(&self) -> BTreeSet<EdgeRow> {
        self.edges.lock().iter().map(|(_, _, _, r)| r.clone()).collect()
    }
}

impl Default for GraphSparkReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(n: usize) -> GraphSparkReader {
        let r = GraphSparkReader::new();
        for i in 1..=(n as i64) {
            r.load_node(NodeRow {
                id: i,
                label: format!("Person{i}"),
                type_: "Entity".into(),
                attr: BTreeMap::from([("age".into(), ((i * 3) % 80).to_string())]),
            });
        }
        for i in 1..=((n / 2) as i64) {
            r.load_edge(EdgeRow {
                source: i,
                target: n as i64 - i + 1,
                label: "knows".into(),
                props: BTreeMap::from([("w".into(), (i % 7).to_string())]),
            });
        }
        r
    }

    #[test]
    fn b3_reader_schema_has_required_fields() {
        let s = GraphSchema::standard_node();
        let names: Vec<&str> = s.fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"id"));
        assert!(names.contains(&"label"));
        assert!(names.contains(&"type_"));
        assert!(names.contains(&"attr"));
        let idf = s.fields.iter().find(|f| f.name == "id").unwrap();
        assert_eq!(idf.data_type, "Long");
    }

    #[test]
    fn b3_paged_nodes_total_and_pagination() {
        let r = seed(30);
        let p1 = r.paged_nodes(1, 10);
        let p2 = r.paged_nodes(2, 10);
        assert_eq!(p1.total, 30);
        assert_eq!(p1.rows.len(), 10);
        assert_eq!(p2.rows.len(), 10);
        // distinct
        let set1: BTreeSet<i64> = p1.rows.iter().map(|r| r.id).collect();
        let set2: BTreeSet<i64> = p2.rows.iter().map(|r| r.id).collect();
        assert!(set1.is_disjoint(&set2));
    }
}
