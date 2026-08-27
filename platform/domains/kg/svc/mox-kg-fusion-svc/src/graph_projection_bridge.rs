// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Bridge between GraphWriter and the projection 2.0 SimpleGraph.
//!
//! Maintains a string↔i64 bijection so GraphWriter string-keyed objects
//! and tags can be materialised as integer-keyed vertices in SimpleGraph.

use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

use mox_kg_service_svc::projection_20::SimpleGraph;

/// A single projection mapping entry: vertex id ↔ (object_id, layer, creation time).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MappingEntry {
    pub vertex_id: i64,
    pub object_id: String,
    pub layer: String,
    pub created_unix_ms: u64,
}

/// Bridge state: bijection tables + SimpleGraph accumulator.
pub struct ProjectionBridge {
    next_id: i64,
    pub s2i: BTreeMap<String, i64>,
    pub i2s: BTreeMap<i64, String>,
    pub graph: SimpleGraph,
    /// Per-object_id layer metadata (populated by register() and upsert helpers).
    pub meta: BTreeMap<String, (String, u64)>, // object_id -> (layer, created_unix_ms)
}

impl std::fmt::Debug for ProjectionBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectionBridge")
            .field("next_id", &self.next_id)
            .field("s2i_len", &self.s2i.len())
            .field("i2s_len", &self.i2s.len())
            .field("vertices", &self.graph.vertices.len())
            .field("fwd_edges", &self.graph.fwd.len())
            .field("bwd_edges", &self.graph.bwd.len())
            .finish()
    }
}

impl Default for ProjectionBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectionBridge {
    /// Construct an empty bridge with an empty SimpleGraph.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            s2i: BTreeMap::new(),
            i2s: BTreeMap::new(),
            graph: SimpleGraph {
                vertices: BTreeMap::new(),
                fwd: BTreeMap::new(),
                bwd: BTreeMap::new(),
            },
            meta: BTreeMap::new(),
        }
    }

    /// Now timestamp helper (ms since unix epoch).
    fn now_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    /// Register (or re-lookup) a projection mapping. If the object_id already
    /// exists in the bijection, returns its existing vertex_id; otherwise
    /// allocates a new id and persists the (layer, created_at) metadata.
    pub fn register(&mut self, object_id: &str, layer: &str) -> i64 {
        if let Some(&id) = self.s2i.get(object_id) {
            // ensure meta exists for revived entries
            self.meta.entry(object_id.to_string()).or_insert_with(|| {
                (layer.to_string(), Self::now_ms())
            });
            // ensure graph vertex present
            if !self.graph.vertices.contains_key(&id) {
                self.graph.add_vertex_with(id, layer, layer, 0, BTreeMap::new());
            }
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.s2i.insert(object_id.to_string(), id);
        self.i2s.insert(id, object_id.to_string());
        let ts = Self::now_ms();
        self.meta.insert(object_id.to_string(), (layer.to_string(), ts));
        let mut attr = BTreeMap::new();
        attr.insert("layer".into(), layer.to_string());
        self.graph.add_vertex_with(id, layer, layer, 0, attr);
        id
    }

    /// Return all known mappings as a list of `MappingEntry`. The returned
    /// list length is capped at 20 entries (deterministic: smallest 20
    /// vertex_ids first) per the T23-2 "20 entries" contract, but callers may
    /// truncate further.
    pub fn all_mappings(&self) -> Vec<MappingEntry> {
        let mut out: Vec<MappingEntry> = Vec::with_capacity(self.i2s.len().min(20));
        for (&vid, oid) in self.i2s.iter().take(20) {
            let (layer, created_ms) = self.meta.get(oid).cloned()
                .unwrap_or_else(|| ("default".to_string(), 0));
            out.push(MappingEntry {
                vertex_id: vid,
                object_id: oid.clone(),
                layer,
                created_unix_ms: created_ms,
            });
        }
        out
    }

    /// Look up a mapping by vertex id; returns None if unknown id.
    pub fn lookup_vertex(&self, id: i64) -> Option<MappingEntry> {
        let oid = self.i2s.get(&id)?;
        let (layer, created_ms) = self.meta.get(oid).cloned()
            .unwrap_or_else(|| ("default".to_string(), 0));
        Some(MappingEntry {
            vertex_id: id,
            object_id: oid.clone(),
            layer,
            created_unix_ms: created_ms,
        })
    }

    /// Reverse lookup by object_id; returns None if unknown.
    pub fn lookup_object(&self, object_id: &str) -> Option<MappingEntry> {
        let &vid = self.s2i.get(object_id)?;
        let (layer, created_ms) = self.meta.get(object_id).cloned()
            .unwrap_or_else(|| ("default".to_string(), 0));
        Some(MappingEntry {
            vertex_id: vid,
            object_id: object_id.to_string(),
            layer,
            created_unix_ms: created_ms,
        })
    }

    fn intern(
        &mut self,
        s: &str,
        type_: &str,
        label: &str,
        community: i64,
        attr: BTreeMap<String, String>,
    ) -> i64 {
        if let Some(&id) = self.s2i.get(s) {
            // Reuse the existing id (per TR `remove_then_revive` requirement)
            // but re-insert the vertex into the live graph in case a prior
            // `remove_object` had pruned it from vertices/fwd/bwd while
            // deliberately keeping the bijection entry.
            if !self.graph.vertices.contains_key(&id) {
                self.graph
                    .add_vertex_with(id, label, type_, community, attr);
            }
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.s2i.insert(s.to_string(), id);
        self.i2s.insert(id, s.to_string());
        self.graph
            .add_vertex_with(id, label, type_, community, attr);
        id
    }

    pub fn upsert_object_vertex(&mut self, uri: &str, meta: &crate::graph_writer::ObjectMeta) {
        let mut attr = BTreeMap::new();
        attr.insert("bucket".into(), meta.bucket.clone());
        attr.insert("key".into(), meta.key.clone());
        attr.insert("size".into(), meta.size_bytes.to_string());
        attr.insert("etag".into(), meta.etag.clone());
        attr.insert("crc".into(), format!("{:016x}", meta.crc64_ecma));
        attr.insert(
            "miji".into(),
            meta.miji_level.unwrap_or(0).to_string(),
        );
        attr.insert(
            "hold".into(),
            meta.hold_until_ms
                .map(|x| x.to_string())
                .unwrap_or_else(|| "none".into()),
        );
        let label: String = uri
            .chars()
            .rev()
            .take(32)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        self.intern(uri, "object", &label, 0, attr);
    }

    pub fn upsert_tag_vertex(&mut self, tag_key: &str, tag_value: &str, usage: u64) {
        let uri = format!("tag://{tag_key}:{tag_value}");
        let mut attr = BTreeMap::new();
        attr.insert("tag_key".into(), tag_key.into());
        attr.insert("tag_value".into(), tag_value.into());
        attr.insert("usage_count".into(), usage.to_string());
        self.intern(&uri, "tag", tag_key, 0, attr);
    }

    /// Add a labelled directed edge between two string-keyed vertices.
    /// Returns true when at least one endpoint was auto-interned as an
    /// empty/placeholder vertex (attr-only vertex); returns false if both
    /// endpoints already existed.  Edges are added bidirectionally via
    /// `SimpleGraph::add_edge` (fwd + bwd adjacency tables).
    pub fn add_edge(&mut self, from_s: &str, to_s: &str, label: &str) -> bool {
        let mut any_new = false;
        let from_id = if let Some(&id) = self.s2i.get(from_s) {
            id
        } else {
            any_new = true;
            self.intern(from_s, "", "", 0, BTreeMap::new())
        };
        let to_id = if let Some(&id) = self.s2i.get(to_s) {
            id
        } else {
            any_new = true;
            self.intern(to_s, "", "", 0, BTreeMap::new())
        };
        self.graph.add_edge(from_id, to_id, label);
        !any_new
    }

    /// Legacy helper for the graph-writer hook; equivalent to
    /// `add_edge(obj, tag, "HAS_TAG")` when both endpoints are known.
    pub fn add_has_tag_edge(&mut self, obj_s: &str, tag_s: &str, label: &str) {
        let s_id = *self.s2i.get(obj_s).expect("obj vertex missing");
        let t_id = *self.s2i.get(tag_s).expect("tag vertex missing");
        self.graph.add_edge(s_id, t_id, label);
    }

    /// Resolve a string URI to its integer id (bijection lookup).
    pub fn s_to_i(&self, s: &str) -> Option<i64> {
        self.s2i.get(s).copied()
    }

    /// Resolve an integer id back to its string URI (bijection lookup).
    pub fn i_to_s(&self, i: i64) -> Option<&str> {
        self.i2s.get(&i).map(|s| s.as_str())
    }

    pub fn remove_object(&mut self, obj_s: &str) {
        let Some(&id) = self.s2i.get(obj_s) else {
            return;
        };
        self.graph.vertices.remove(&id);
        if let Some(fwd_list) = self.graph.fwd.remove(&id) {
            for (to, _) in fwd_list {
                if let Some(bucket) = self.graph.bwd.get_mut(&to) {
                    bucket.retain(|(from, _)| from != &id);
                    if bucket.is_empty() {
                        self.graph.bwd.remove(&to);
                    }
                }
            }
        }
        if let Some(bwd_list) = self.graph.bwd.remove(&id) {
            for (from, _) in bwd_list {
                if let Some(bucket) = self.graph.fwd.get_mut(&from) {
                    bucket.retain(|(to, _)| to != &id);
                    if bucket.is_empty() {
                        self.graph.fwd.remove(&from);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_meta(key: &str) -> crate::graph_writer::ObjectMeta {
        crate::graph_writer::ObjectMeta {
            bucket: "bkt".into(),
            key: key.into(),
            size_bytes: 1024,
            etag: "deadbeef".into(),
            crc64_ecma: 0xDEADBEEF_CAFEBABE,
            miji_level: Some(3),
            hold_until_ms: Some(1_700_000_000_000),
        }
    }

    #[test]
    fn t23_bridge_1k_bijection() {
        let mut b = ProjectionBridge::new();
        for i in 0..1000 {
            let uri = format!("s3://b/i{i}");
            let meta = crate::graph_writer::ObjectMeta {
                bucket: "b".into(),
                key: format!("i{i}"),
                size_bytes: i as u64,
                etag: format!("e{i}"),
                crc64_ecma: i as u64,
                miji_level: None,
                hold_until_ms: None,
            };
            b.upsert_object_vertex(&uri, &meta);
        }
        for i in 0..500 {
            b.upsert_tag_vertex(&format!("tk{i}"), &format!("tv{i}"), i as u64);
        }
        assert_eq!(b.s2i.len(), 1500);
        assert_eq!(b.i2s.len(), 1500);
        let ids: std::collections::BTreeSet<i64> = b.s2i.values().copied().collect();
        assert_eq!(ids.len(), 1500);
        for id in 1..=1500i64 {
            assert!(ids.contains(&id));
            assert!(b.i2s.contains_key(&id));
        }
        let id0 = b.s2i["s3://b/i0"];
        assert_eq!(b.i2s[&id0], "s3://b/i0");
    }

    #[test]
    fn t23_bridge_remove_revive() {
        let mut b = ProjectionBridge::new();
        let uri_a = "s3://b/A";
        let id_first = {
            b.upsert_object_vertex(uri_a, &dummy_meta("A"));
            b.s2i[uri_a]
        };
        let tag_uri = "tag://k:v";
        b.upsert_tag_vertex("k", "v", 1);
        b.add_has_tag_edge(uri_a, &tag_uri, "HAS_TAG");
        assert!(b.graph.vertices.contains_key(&id_first));
        assert!(b.graph.fwd.contains_key(&id_first));

        b.remove_object(uri_a);
        assert!(!b.graph.vertices.contains_key(&id_first));
        assert!(!b.graph.fwd.contains_key(&id_first));
        assert!(b.s2i.contains_key(uri_a));
        assert_eq!(b.s2i[uri_a], id_first);
        // i64 id still maps back to the original string after remove.
        assert_eq!(b.i_to_s(id_first), Some(uri_a));
        assert_eq!(b.s_to_i(uri_a), Some(id_first));

        let id_revived = {
            b.upsert_object_vertex(uri_a, &dummy_meta("A"));
            b.s2i[uri_a]
        };
        assert_eq!(id_revived, id_first, "revive must reuse original id");
        assert!(b.graph.vertices.contains_key(&id_first));
        let v = &b.graph.vertices[&id_first];
        assert_eq!(v.type_, "object");
        assert_eq!(v.attr.get("miji"), Some(&"3".to_string()));
    }

    #[test]
    fn t23_graphwriter_hook_passthrough() {
        use crate::graph_writer::GraphWriter;
        use crate::tag_parser::Tag;
        use std::sync::Arc;

        let bridge = Arc::new(parking_lot::Mutex::new(ProjectionBridge::new()));
        let gw = GraphWriter::new().with_projection_bridge(Arc::clone(&bridge));
        let tags = vec![
            Tag::new("project", "alpha"),
            Tag::new("team", "risk"),
            Tag::new("level", "3"),
        ];
        let uri = "s3://bkt/report.q2.pdf";
        gw.upsert_obj_and_tags(uri, "bkt", 4096, "etag-abc", &tags, None)
            .expect("upsert ok");

        let locked = bridge.lock();
        let obj_id = locked
            .s_to_i(uri)
            .expect("obj uri should be interned in bridge");
        let fwd = locked
            .graph
            .fwd
            .get(&obj_id)
            .cloned()
            .unwrap_or_default();
        assert!(
            fwd.len() >= 3,
            "expected >= 3 HAS_TAG edges from obj vertex, got {}",
            fwd.len()
        );
    }
}
