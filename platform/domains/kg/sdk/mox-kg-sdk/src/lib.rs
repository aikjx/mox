// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Mox SDK Graph — in-memory fake facade for CDC, Spark connectors,
//! graph projection operations, and the AC-15 fault-injection audit matrix.
//!
//! No network I/O. All state lives inside `GraphClient`.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::sync::Mutex;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GraphError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("service error: {0}")]
    Service(String),
    #[error("cdc end of stream: {0}")]
    CdcEndOfStream(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("disk full: {0}")]
    DiskFull(String),
    #[error("ac fault injected: {0}")]
    AcFault(String),
    #[error("audit callback: {0}")]
    AuditCallback(String),
    #[error("lock poison: {0}")]
    Lock(String),
}

pub type Result<T> = std::result::Result<T, GraphError>;

// ---------- DTOs ----------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Node {
    pub id: i64,
    pub label: String,
    pub typ: String,
    pub community: i64,
    pub attrs: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Edge {
    pub id: i64,
    pub src: i64,
    pub dst: i64,
    pub label: String,
    pub weight: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CdcRecord {
    pub offset: u64,
    pub op: String,   // "INSERT" | "UPDATE" | "DELETE"
    pub entity: String, // "node" | "edge"
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CdcConsumer {
    pub id: String,
    pub topic: String,
    pub offset: u64,
    pub dedup_count: u64,
    pub last_lag_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SparkPage<T> {
    pub items: Vec<T>,
    pub page: u32,
    pub page_size: u32,
    pub total: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct SparkStats {
    pub nodes_written: u64,
    pub edges_written: u64,
    pub upserts_applied: u64,
    pub idempotent_skips: u64,
    pub roundtrips: u64,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectionSpec {
    pub name: String,
    pub node_labels: Vec<String>,
    pub edge_labels: Vec<String>,
    pub attrs_out: Vec<String>,
    pub attrs_in: Vec<String>,
    pub min_degree_out: u32,
    pub community: Option<i64>,
    pub type_out: Option<String>,
    pub type_in: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectionResult {
    pub spec_name: String,
    pub node_count: u64,
    pub edge_count: u64,
    pub sample_node_ids: Vec<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct Ac15Report {
    pub fault_injected: bool,
    pub fault_tag: String,
    pub dedup_hits: u64,
    pub lag_spike_ms: u64,
    pub audit_entries: Vec<String>,
    pub callback_fired: bool,
    pub idempotent_verified: bool,
    pub diskfull_triggered: bool,
    pub timeout_hits: u64,
    pub lost_zero_count: u64,
    pub partial_writes: u64,
}

// ---------- GraphClient ----------

#[derive(Clone, Default)]
pub struct GraphClient {
    inner: std::sync::Arc<Mutex<GraphState>>,
}

#[derive(Default)]
struct GraphState {
    nodes: BTreeMap<i64, Node>,
    edges: BTreeMap<i64, Edge>,
    next_node_id: i64,
    next_edge_id: i64,
    cdc_log: VecDeque<CdcRecord>,
    cdc_next_offset: u64,
    cdc_consumers: HashMap<String, CdcConsumer>,
    spark_stats: SparkStats,
    projections: HashMap<String, ProjectionSpec>,
    ac15: Ac15Report,
    // fault toggles — when true the matching AC-xx API returns the injected fault
    faults: HashSet<&'static str>,
}

pub type Client = GraphClient;

pub const GRAPH_EXAMPLE_IDS: &[&str] = &[
    "graph-001_cdc_new",
    "graph-002_cdc_next_blocking",
    "graph-003_cdc_resume_offset",
    "graph-004_cdc_100k_via_writer",
    "graph-005_cdc_dedup_stats",
    "graph-006_cdc_lag_monitor",
    "graph-007_cdc_consumer_id_rotate",
    "graph-008_spark_reader_paged_nodes",
    "graph-009_spark_reader_paged_edges",
    "graph-010_spark_writer_bulk",
    "graph-011_spark_idempotent_upsert",
    "graph-012_spark_roundtrip_2k_3k",
    "graph-013_spark_roundtrip_5k_8k",
    "graph-014_spark_stats_accumulate",
    "graph-015_proj_type_out_1",
    "graph-016_proj_type_out_2",
    "graph-017_proj_community_in_1",
    "graph-018_proj_community_in_2",
    "graph-019_proj_attr_out",
    "graph-020_proj_attr_in",
    "graph-021_proj_degree_out_2",
    "graph-022_proj_label_in_1",
    "graph-023_ac15_f1_double_idempotent",
    "graph-024_ac15_f3_lost_zero",
    "graph-025_ac15_f6_partial",
    "graph-026_ac15_f7_diskfull_err",
    "graph-027_ac15_f8_cb_plus_audit",
    "graph-028_ac15_f12_timeout_dedup",
    "graph-029_ac15_f13_lag_spike",
    "graph-030_ac15_f14_audit_cb",
];

impl GraphClient {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, GraphState>> {
        self.inner.lock().map_err(|e| GraphError::Lock(e.to_string()))
    }

    // ---------- helpers for synthetic data ----------

    fn make_node(id: i64, label: &str, typ: &str, community: i64) -> Node {
        Node {
            id,
            label: label.into(),
            typ: typ.into(),
            community,
            attrs: HashMap::new(),
        }
    }

    fn make_edge(id: i64, src: i64, dst: i64, label: &str, weight: f64) -> Edge {
        Edge { id, src, dst, label: label.into(), weight }
    }

    // ========== CDC (7) ==========

    pub async fn cdc_new_consumer(&self, topic: &str, consumer_id: &str) -> Result<CdcConsumer> {
        let mut s = self.lock()?;
        let cc = CdcConsumer {
            id: consumer_id.into(),
            topic: topic.into(),
            offset: 0,
            dedup_count: 0,
            last_lag_ms: 0,
        };
        s.cdc_consumers.insert(consumer_id.into(), cc.clone());
        Ok(cc)
    }

    /// Pop one record from the in-memory CDC log for this consumer (blocking
    /// semantics are simulated by always returning if any are available).
    pub async fn cdc_next_blocking(&self, consumer_id: &str) -> Result<CdcRecord> {
        let mut s = self.lock()?;
        // Validate consumer exists first.
        if !s.cdc_consumers.contains_key(consumer_id) {
            return Err(GraphError::NotFound(format!("consumer {consumer_id}")));
        }
        let cid = consumer_id.to_string();
        // read-only: offset, len
        let cur_offset = s.cdc_consumers[&cid].offset;
        let log_len = s.cdc_log.len() as u64;
        if cur_offset >= log_len {
            return Err(GraphError::CdcEndOfStream(format!(
                "no more records for {consumer_id}, offset={cur_offset}"
            )));
        }
        let rec = s.cdc_log[cur_offset as usize].clone();
        // now mutable: bump offset
        let cons = s.cdc_consumers.get_mut(&cid).unwrap();
        cons.offset = rec.offset + 1;
        cons.last_lag_ms = 0;
        Ok(rec)
    }

    pub async fn cdc_resume_offset(&self, consumer_id: &str, offset: u64) -> Result<u64> {
        let mut s = self.lock()?;
        let cons = s
            .cdc_consumers
            .get_mut(consumer_id)
            .ok_or_else(|| GraphError::NotFound(format!("consumer {consumer_id}")))?;
        cons.offset = offset;
        Ok(cons.offset)
    }

    /// Simulate a CDC writer that produces N synthetic inserts into the log.
    /// Returns how many records are currently buffered.
    pub async fn cdc_write_records(&self, count: u64, prefix: &str) -> Result<u64> {
        let mut s = self.lock()?;
        for i in 0..count {
            let offset = s.cdc_next_offset;
            s.cdc_next_offset += 1;
            s.cdc_log.push_back(CdcRecord {
                offset,
                op: if i % 10 == 0 { "UPDATE" } else { "INSERT" }.into(),
                entity: "node".into(),
                payload: serde_json::json!({
                    "id": offset as i64,
                    "label": format!("{prefix}-{i}"),
                    "idx": i,
                }),
            });
        }
        Ok(s.cdc_log.len() as u64)
    }

    pub async fn cdc_dedup_bump(&self, consumer_id: &str, hits: u64) -> Result<u64> {
        let mut s = self.lock()?;
        let cons = s
            .cdc_consumers
            .get_mut(consumer_id)
            .ok_or_else(|| GraphError::NotFound(format!("consumer {consumer_id}")))?;
        cons.dedup_count += hits;
        Ok(cons.dedup_count)
    }

    pub async fn cdc_lag_sample(&self, consumer_id: &str, lag_ms: u64) -> Result<u64> {
        let mut s = self.lock()?;
        let cons = s
            .cdc_consumers
            .get_mut(consumer_id)
            .ok_or_else(|| GraphError::NotFound(format!("consumer {consumer_id}")))?;
        cons.last_lag_ms = lag_ms;
        Ok(cons.last_lag_ms)
    }

    pub async fn cdc_rotate_consumer(&self, old_id: &str, new_id: &str) -> Result<CdcConsumer> {
        let mut s = self.lock()?;
        let prev = s
            .cdc_consumers
            .remove(old_id)
            .ok_or_else(|| GraphError::NotFound(format!("consumer {old_id}")))?;
        let fresh = CdcConsumer { id: new_id.into(), ..prev };
        s.cdc_consumers.insert(new_id.into(), fresh.clone());
        Ok(fresh)
    }

    pub async fn cdc_get_consumer(&self, id: &str) -> Result<CdcConsumer> {
        let s = self.lock()?;
        s.cdc_consumers
            .get(id)
            .cloned()
            .ok_or_else(|| GraphError::NotFound(format!("consumer {id}")))
    }

    // ========== Spark (7) ==========

    pub async fn spark_seed_nodes(&self, count: u32) -> Result<u64> {
        let mut s = self.lock()?;
        for _ in 0..count {
            let id = s.next_node_id;
            s.next_node_id += 1;
            let community = (id % 7) as i64;
            let labels = ["User", "Product", "Order", "Account"];
            let typs = ["Person", "Item", "Event", "Org"];
            let n = Self::make_node(
                id,
                labels[(id as usize) % labels.len()],
                typs[(id as usize) % typs.len()],
                community,
            );
            s.nodes.insert(id, n);
        }
        Ok(s.nodes.len() as u64)
    }

    pub async fn spark_seed_edges(&self, count: u32) -> Result<u64> {
        let mut s = self.lock()?;
        if s.nodes.len() < 2 {
            return Err(GraphError::InvalidRequest(
                "seed at least 2 nodes before edges".into(),
            ));
        }
        let ids: Vec<i64> = s.nodes.keys().copied().collect();
        let labels = ["KNOWS", "BOUGHT", "FOLLOWS", "LINKED_TO", "AUTHOR_OF"];
        for i in 0..count {
            let id = s.next_edge_id;
            s.next_edge_id += 1;
            let src = ids[(i as usize) % ids.len()];
            let dst = ids[((i as usize) * 7 + 3) % ids.len()];
            let w = ((i % 99) as f64) / 10.0 + 0.5;
            let e = Self::make_edge(id, src, dst, labels[(i as usize) % labels.len()], w);
            s.edges.insert(id, e);
        }
        Ok(s.edges.len() as u64)
    }

    pub async fn spark_reader_nodes_paged(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<SparkPage<Node>> {
        let s = self.lock()?;
        let all: Vec<Node> = s.nodes.values().cloned().collect();
        let total = all.len() as u64;
        let start = (page.saturating_sub(1)) as usize * page_size as usize;
        let items: Vec<Node> = all.into_iter().skip(start).take(page_size as usize).collect();
        Ok(SparkPage { items, page, page_size, total })
    }

    pub async fn spark_reader_edges_paged(
        &self,
        page: u32,
        page_size: u32,
    ) -> Result<SparkPage<Edge>> {
        let s = self.lock()?;
        let all: Vec<Edge> = s.edges.values().cloned().collect();
        let total = all.len() as u64;
        let start = (page.saturating_sub(1)) as usize * page_size as usize;
        let items: Vec<Edge> = all.into_iter().skip(start).take(page_size as usize).collect();
        Ok(SparkPage { items, page, page_size, total })
    }

    pub async fn spark_writer_bulk(
        &self,
        nodes: Vec<Node>,
        edges: Vec<Edge>,
    ) -> Result<(u64, u64)> {
        let mut s = self.lock()?;
        let mut n_added = 0u64;
        let mut e_added = 0u64;
        for mut n in nodes {
            if n.id == 0 {
                n.id = s.next_node_id;
                s.next_node_id += 1;
            } else if n.id >= s.next_node_id {
                s.next_node_id = n.id + 1;
            }
            if s.nodes.insert(n.id, n).is_none() {
                n_added += 1;
            }
        }
        for mut e in edges {
            if e.id == 0 {
                e.id = s.next_edge_id;
                s.next_edge_id += 1;
            } else if e.id >= s.next_edge_id {
                s.next_edge_id = e.id + 1;
            }
            if s.edges.insert(e.id, e).is_none() {
                e_added += 1;
            }
        }
        s.spark_stats.nodes_written += n_added;
        s.spark_stats.edges_written += e_added;
        Ok((n_added, e_added))
    }

    pub async fn spark_upsert(&self, nodes: Vec<Node>) -> Result<(u64, u64)> {
        let mut s = self.lock()?;
        let mut applied = 0u64;
        let mut skipped = 0u64;
        for n in nodes {
            if let Some(existing) = s.nodes.get(&n.id) {
                if existing == &n {
                    skipped += 1;
                    continue;
                }
            }
            s.nodes.insert(n.id, n);
            applied += 1;
        }
        s.spark_stats.upserts_applied += applied;
        s.spark_stats.idempotent_skips += skipped;
        Ok((applied, skipped))
    }

    pub async fn spark_stats(&self) -> Result<SparkStats> {
        let s = self.lock()?;
        Ok(s.spark_stats.clone())
    }

    pub async fn spark_inc_roundtrip(&self, nodes: u64, edges: u64) -> Result<(u64, u64)> {
        // helper: seeds nodes+edges of given sizes, returns counts
        let _ = self.spark_seed_nodes(nodes as u32).await?;
        let e = self.spark_seed_edges(edges as u32).await?;
        let mut s = self.lock()?;
        s.spark_stats.roundtrips += 1;
        Ok((s.nodes.len() as u64, e))
    }

    // ========== Projection (8) ==========

    pub async fn projection_define(&self, spec: ProjectionSpec) -> Result<()> {
        if spec.name.is_empty() {
            return Err(GraphError::InvalidRequest("empty projection name".into()));
        }
        let mut s = self.lock()?;
        s.projections.insert(spec.name.clone(), spec);
        Ok(())
    }

    pub async fn projection_run(&self, name: &str) -> Result<ProjectionResult> {
        let s = self.lock()?;
        let spec = s
            .projections
            .get(name)
            .cloned()
            .ok_or_else(|| GraphError::NotFound(format!("projection {name}")))?;

        // Filter nodes per spec
        let mut nodes: Vec<&Node> = s.nodes.values().collect();
        if !spec.node_labels.is_empty() {
            nodes.retain(|n| spec.node_labels.iter().any(|l| l == &n.label));
        }
        if let Some(t) = &spec.type_out {
            nodes.retain(|n| &n.typ == t);
        }
        if let Some(t) = &spec.type_in {
            nodes.retain(|n| &n.typ == t);
        }
        if let Some(c) = spec.community {
            nodes.retain(|n| n.community == c);
        }
        // min_degree_out = only keep nodes with ≥ N outgoing edges
        if spec.min_degree_out > 0 {
            let out_deg: HashMap<i64, u32> =
                s.edges.values().fold(HashMap::new(), |mut acc, e| {
                    *acc.entry(e.src).or_insert(0) += 1;
                    acc
                });
            nodes.retain(|n| *out_deg.get(&n.id).unwrap_or(&0) >= spec.min_degree_out);
        }
        // attrs_out/attrs_in: filter to nodes that contain ALL listed attrs
        if !spec.attrs_out.is_empty() {
            nodes.retain(|n| spec.attrs_out.iter().all(|a| n.attrs.contains_key(a)));
        }
        if !spec.attrs_in.is_empty() {
            nodes.retain(|n| spec.attrs_in.iter().all(|a| n.attrs.contains_key(a)));
        }
        let node_ids: HashSet<i64> = nodes.iter().map(|n| n.id).collect();
        let edge_count = s
            .edges
            .values()
            .filter(|e| {
                node_ids.contains(&e.src)
                    && node_ids.contains(&e.dst)
                    && (spec.edge_labels.is_empty()
                        || spec.edge_labels.iter().any(|l| l == &e.label))
            })
            .count() as u64;
        let sample_node_ids: Vec<i64> = nodes.iter().take(10).map(|n| n.id).collect();
        Ok(ProjectionResult {
            spec_name: spec.name.clone(),
            node_count: nodes.len() as u64,
            edge_count,
            sample_node_ids,
        })
    }

    /// Convenience to bulk-tag a node's attributes (used by projection tests).
    pub async fn node_set_attrs(&self, node_id: i64, attrs: Vec<(String, String)>) -> Result<()> {
        let mut s = self.lock()?;
        let n = s
            .nodes
            .get_mut(&node_id)
            .ok_or_else(|| GraphError::NotFound(format!("node {node_id}")))?;
        for (k, v) in attrs {
            n.attrs.insert(k, v);
        }
        Ok(())
    }

    pub async fn list_nodes(&self) -> Result<Vec<Node>> {
        let s = self.lock()?;
        Ok(s.nodes.values().cloned().collect())
    }

    pub async fn list_edges(&self) -> Result<Vec<Edge>> {
        let s = self.lock()?;
        Ok(s.edges.values().cloned().collect())
    }

    // ========== AC-15 fault injection matrix (8) ==========

    /// Enable a named fault for the matching AC-xx check.
    pub async fn ac15_inject(&self, fault_tag: &'static str) -> Result<()> {
        let mut s = self.lock()?;
        s.faults.insert(fault_tag);
        Ok(())
    }

    pub async fn ac15_reset(&self) -> Result<()> {
        let mut s = self.lock()?;
        s.faults.clear();
        s.ac15 = Ac15Report::default();
        Ok(())
    }

    pub async fn ac15_report(&self) -> Result<Ac15Report> {
        let s = self.lock()?;
        Ok(s.ac15.clone())
    }

    /// AC-15 F1: Double-write idempotency check.
    /// Apply `data` twice; second run should be no-op → idempotent_verified=true.
    pub async fn ac15_f1_double_idempotent(
        &self,
        nodes: Vec<Node>,
    ) -> Result<(u64, Ac15Report)> {
        let (mut applied_total, mut skip_total) = (0u64, 0u64);
        for _ in 0..2 {
            let (a, s) = self.spark_upsert(nodes.clone()).await?;
            applied_total += a;
            skip_total += s;
        }
        let mut s = self.lock()?;
        s.ac15.fault_injected = s.faults.contains("f1");
        s.ac15.fault_tag = "f1".into();
        s.ac15.idempotent_verified = skip_total > 0 && applied_total == skip_total;
        // Actually idempotent means all second writes were skips:
        // First pass applied some, second pass all equal → skips only.
        let report = s.ac15.clone();
        Ok((skip_total, report))
    }

    /// AC-15 F3: lost-zero. When zero values are written to numeric attrs,
    /// the audit trail must not "lose" them (default attr map missing → 0 would be lost).
    /// This call writes nodes with explicit "zero" attrs and counts them.
    pub async fn ac15_f3_lost_zero(&self, node_count: u32) -> Result<(u64, Ac15Report)> {
        let mut s = self.lock()?;
        let mut zeros = 0u64;
        for _ in 0..node_count {
            let id = s.next_node_id;
            s.next_node_id += 1;
            let mut attrs = HashMap::new();
            // explicit zero-valued attrs
            attrs.insert("score".into(), "0".into());
            attrs.insert("balance".into(), "0".into());
            let n = Node {
                id,
                label: "ZeroAccount".into(),
                typ: "Org".into(),
                community: 0,
                attrs,
            };
            if n.attrs.get("score").map(|v| v.as_str()) == Some("0") {
                zeros += 1;
            }
            if n.attrs.get("balance").map(|v| v.as_str()) == Some("0") {
                zeros += 1;
            }
            s.nodes.insert(id, n);
        }
        s.ac15.lost_zero_count = zeros;
        s.ac15.fault_tag = "f3".into();
        let report = s.ac15.clone();
        Ok((zeros, report))
    }

    /// AC-15 F6: partial write. When a batch has a mix of valid/invalid rows,
    /// the valid subset must still land (partial writes enabled). Returns count written.
    pub async fn ac15_f6_partial(
        &self,
        nodes: Vec<Option<Node>>,
    ) -> Result<(u64, Ac15Report)> {
        let mut written = 0u64;
        let mut s = self.lock()?;
        for n in nodes {
            let Some(mut nn) = n else { continue };
            if nn.id == 0 {
                nn.id = s.next_node_id;
                s.next_node_id += 1;
            }
            s.nodes.insert(nn.id, nn);
            written += 1;
        }
        s.ac15.partial_writes = written;
        s.ac15.fault_tag = "f6".into();
        let report = s.ac15.clone();
        Ok((written, report))
    }

    /// AC-15 F7: diskfull. If the "f7" fault is injected, returns DiskFull error.
    /// Otherwise succeeds and updates report.
    pub async fn ac15_f7_diskfull(&self, write_bytes: u64) -> Result<(u64, Ac15Report)> {
        let mut s = self.lock()?;
        if s.faults.contains("f7") {
            s.ac15.diskfull_triggered = true;
            s.ac15.fault_tag = "f7".into();
            let _r = s.ac15.clone();
            drop(s); // release before Err
            return Err(GraphError::DiskFull(format!(
                "simulated disk full on write of {write_bytes} bytes"
            )));
        }
        s.ac15.fault_tag = "f7".into();
        let r = s.ac15.clone();
        Ok((write_bytes, r))
    }

    /// AC-15 F8: callback plus audit. Fire an in-memory "callback" (append to
    /// audit_entries), and return fired=true.
    pub async fn ac15_f8_cb_audit(&self, tag: &str) -> Result<(bool, Ac15Report)> {
        let mut s = self.lock()?;
        s.ac15
            .audit_entries
            .push(format!("F8-callback fired: {tag}"));
        s.ac15.callback_fired = true;
        s.ac15.fault_tag = "f8".into();
        let r = s.ac15.clone();
        Ok((r.callback_fired, r))
    }

    /// AC-15 F12: timeout + dedup. If "f12" fault is set → Timeout after
    /// incrementing dedup count. Otherwise success.
    pub async fn ac15_f12_timeout_dedup(&self, dedup_hits: u64) -> Result<(u64, Ac15Report)> {
        let mut s = self.lock()?;
        s.ac15.dedup_hits += dedup_hits;
        s.ac15.fault_tag = "f12".into();
        if s.faults.contains("f12") {
            s.ac15.timeout_hits += 1;
            let _r = s.ac15.clone();
            drop(s);
            return Err(GraphError::Timeout(format!(
                "f12 simulated timeout after dedup={dedup_hits}"
            )));
        }
        let r = s.ac15.clone();
        Ok((s.ac15.dedup_hits, r))
    }

    /// AC-15 F13: lag spike. Record a lag_spike_ms sample and return it.
    pub async fn ac15_f13_lag_spike(&self, ms: u64) -> Result<(u64, Ac15Report)> {
        let mut s = self.lock()?;
        s.ac15.lag_spike_ms = ms;
        s.ac15.fault_tag = "f13".into();
        let r = s.ac15.clone();
        Ok((ms, r))
    }

    /// AC-15 F14: audit-only callback. Appends entries, never fails, returns count.
    pub async fn ac15_f14_audit_cb(&self, entries: &[&str]) -> Result<(u64, Ac15Report)> {
        let mut s = self.lock()?;
        for e in entries {
            s.ac15.audit_entries.push(format!("F14-audit: {e}"));
        }
        s.ac15.callback_fired = true;
        s.ac15.fault_tag = "f14".into();
        let n = s.ac15.audit_entries.len() as u64;
        let r = s.ac15.clone();
        Ok((n, r))
    }
}
