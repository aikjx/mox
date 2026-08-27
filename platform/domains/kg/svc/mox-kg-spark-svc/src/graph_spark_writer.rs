// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Writer side: bulk upsert, idempotent key, WrittenStats with symmetric_diff helper.

use crate::graph_spark_reader::{EdgeRow, GraphSparkReader, NodeRow};
use anyhow::Result;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SparkRow {
    Node(NodeRow),
    Edge(EdgeRow),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdempotencyKey {
    Node(i64),
    Edge(i64, i64, String), // (source, target, label)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WrittenStats {
    pub nodes_inserted: u64,
    pub nodes_updated: u64,
    pub edges_inserted: u64,
    pub edges_updated: u64,
    pub duplicates_skipped: u64,
    pub failed_rows: u64,
}

impl std::ops::AddAssign for WrittenStats {
    fn add_assign(&mut self, rhs: Self) {
        self.nodes_inserted += rhs.nodes_inserted;
        self.nodes_updated += rhs.nodes_updated;
        self.edges_inserted += rhs.edges_inserted;
        self.edges_updated += rhs.edges_updated;
        self.duplicates_skipped += rhs.duplicates_skipped;
        self.failed_rows += rhs.failed_rows;
    }
}

/// Idempotent writer: shares internal storage with the paired reader via Arc<Reader>.
pub struct GraphSparkWriter {
    reader: std::sync::Arc<GraphSparkReader>,
    // Additional idempotency-tracking maps (key -> bool seen update)
    node_seen: Mutex<BTreeSet<i64>>,
    edge_seen: Mutex<BTreeSet<(i64, i64, String)>>,
}

impl GraphSparkWriter {
    pub fn new(reader: std::sync::Arc<GraphSparkReader>) -> Self {
        Self {
            reader,
            node_seen: Mutex::new(BTreeSet::new()),
            edge_seen: Mutex::new(BTreeSet::new()),
        }
    }

    /// Bulk write. Returns stats. If any row has empty fields → failed_rows.
    pub fn bulk(&self, rows: impl IntoIterator<Item = SparkRow>) -> Result<WrittenStats> {
        let mut s = WrittenStats::default();
        for row in rows {
            match row {
                SparkRow::Node(n) => {
                    if n.id <= 0 || n.label.is_empty() || n.type_.is_empty() {
                        s.failed_rows += 1;
                        continue;
                    }
                    let inserted = self.node_seen.lock().insert(n.id);
                    if inserted {
                        s.nodes_inserted += 1;
                    } else {
                        s.nodes_updated += 1;
                    }
                    self.reader.load_node(n);
                }
                SparkRow::Edge(e) => {
                    if e.source <= 0 || e.target <= 0 || e.label.is_empty() {
                        s.failed_rows += 1;
                        continue;
                    }
                    let k = (e.source, e.target, e.label.clone());
                    let inserted = self.edge_seen.lock().insert(k);
                    if inserted {
                        s.edges_inserted += 1;
                    } else {
                        s.edges_updated += 1;
                    }
                    self.reader.load_edge(e);
                }
            }
        }
        Ok(s)
    }

    /// Convenience: compare written set snapshot (reader) with expected set.
    pub fn nodes_vs(&self, expected: &BTreeSet<NodeRow>) -> (BTreeSet<NodeRow>, BTreeSet<NodeRow>) {
        let got = self.reader.snapshot_nodes_set();
        let only_in_got: BTreeSet<NodeRow> = got.difference(expected).cloned().collect();
        let only_in_exp: BTreeSet<NodeRow> = expected.difference(&got).cloned().collect();
        (only_in_got, only_in_exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn node(i: i64) -> NodeRow {
        NodeRow {
            id: i,
            label: format!("L{i}"),
            type_: "Person".into(),
            attr: BTreeMap::new(),
        }
    }
    fn edge(s: i64, t: i64, lab: &str) -> EdgeRow {
        EdgeRow { source: s, target: t, label: lab.into(), props: BTreeMap::new() }
    }

    #[test]
    fn b3_writer_bulk_inserts_counts_correct() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let rows = (1..=50).map(|i| SparkRow::Node(node(i)))
            .chain((1..=20).map(|i| SparkRow::Edge(edge(i, i + 50, "knows"))));
        let s = w.bulk(rows).unwrap();
        assert_eq!(s.nodes_inserted, 50);
        assert_eq!(s.edges_inserted, 20);
        assert_eq!(s.failed_rows, 0);
    }

    #[test]
    fn b3_idempotent_key_same_row_is_update_not_duplicate_error() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let s1 = w.bulk(vec![SparkRow::Edge(edge(1, 2, "k"))]).unwrap();
        let s2 = w.bulk(vec![SparkRow::Edge(edge(1, 2, "k"))]).unwrap();
        assert_eq!(s1.edges_inserted, 1);
        assert_eq!(s2.edges_updated, 1);
        // Still only 1 edge actually stored (load_edge overwrites via set)
        assert_eq!(r.count_edges(), 1);
    }

    #[test]
    fn b3_roundtrip_2000_nodes_3000_edges_set_symmetric_diff_empty() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let mut exp_nodes = BTreeSet::new();
        let mut exp_edges = BTreeSet::new();
        let mut rows = vec![];
        for i in 1..=2000 {
            let n = node(i);
            exp_nodes.insert(n.clone());
            rows.push(SparkRow::Node(n));
        }
        for i in 1..=3000 {
            let s = ((i * 13) % 2000) + 1;
            let t = ((i * 17) % 2000) + 1;
            let e = edge(s, t, &format!("e{i}"));
            exp_edges.insert(e.clone());
            rows.push(SparkRow::Edge(e));
        }
        let _ = w.bulk(rows).unwrap();
        let got_nodes = r.snapshot_nodes_set();
        let got_edges = r.snapshot_edges_set();
        let n_only_got: BTreeSet<_> = got_nodes.difference(&exp_nodes).collect();
        let n_only_exp: BTreeSet<_> = exp_nodes.difference(&got_nodes).collect();
        assert!(n_only_got.is_empty(), "only in got nodes: {}", n_only_got.len());
        assert!(n_only_exp.is_empty(), "only in expected nodes: {}", n_only_exp.len());
        let e_only_got: BTreeSet<_> = got_edges.difference(&exp_edges).collect();
        let e_only_exp: BTreeSet<_> = exp_edges.difference(&got_edges).collect();
        assert!(e_only_got.is_empty(), "only in got edges: {}", e_only_got.len());
        assert!(e_only_exp.is_empty(), "only in expected edges: {}", e_only_exp.len());
    }

    #[test]
    fn b3_failed_rows_counted_for_invalid_id_or_label() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let s = w
            .bulk(vec![
                SparkRow::Node(NodeRow {
                    id: 0,
                    label: "".into(),
                    type_: "".into(),
                    attr: BTreeMap::new(),
                }),
                SparkRow::Edge(EdgeRow {
                    source: 0,
                    target: 0,
                    label: "".into(),
                    props: BTreeMap::new(),
                }),
            ])
            .unwrap();
        assert_eq!(s.failed_rows, 2);
        assert_eq!(r.count_nodes(), 0);
        assert_eq!(r.count_edges(), 0);
    }

    #[test]
    fn b3_stats_add_assign_accumulates() {
        let mut a = WrittenStats {
            nodes_inserted: 2,
            edges_inserted: 3,
            duplicates_skipped: 1,
            ..Default::default()
        };
        a += WrittenStats { nodes_inserted: 1, edges_updated: 4, ..Default::default() };
        assert_eq!(a.nodes_inserted, 3);
        assert_eq!(a.edges_updated, 4);
        assert_eq!(a.duplicates_skipped, 1);
    }

    #[test]
    fn b3_large_page_read_does_not_overflow_total() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let _ = w.bulk((1..=137).map(|i| SparkRow::Node(node(i)))).unwrap();
        let frame = r.paged_nodes(1, 999);
        assert_eq!(frame.total, 137);
        assert_eq!(frame.rows.len(), 137);
    }

    #[test]
    fn b3_nodes_vs_helper_detects_difference() {
        let r = Arc::new(GraphSparkReader::new());
        let w = GraphSparkWriter::new(r.clone());
        let _ = w.bulk(vec![SparkRow::Node(node(1)), SparkRow::Node(node(2))]).unwrap();
        let mut exp = BTreeSet::new();
        exp.insert(node(1));
        exp.insert(node(3));
        let (got_only, exp_only) = w.nodes_vs(&exp);
        assert_eq!(got_only.iter().next().unwrap().id, 2);
        assert_eq!(exp_only.iter().next().unwrap().id, 3);
    }
}
