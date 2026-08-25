//! Mock GraphWriter: emulates a small projection against a knowledge graph,
//! with idempotent upserts, edge archiving, soft-delete, failure injection
//! and a small reverse index.
//!
//! This is **NOT** a production graph; it is written specifically for the
//! Task 4 14-case integration test matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use parking_lot::Mutex;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::graph_projection_bridge::ProjectionBridge;
use thiserror::Error;

use crate::audit_sync::{AuditEvent, AuditRecordKind};
use crate::cdc_stage::ObjectTagged;
use crate::tag_parser::Tag;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("injected graph writer failure #{remaining}/{total}")]
    Injected { total: u32, remaining: u32 },
    #[error("graph writer internal: {0}")]
    Internal(String),
}

/// High-level `(objs, tags, edges)` tuple used by tests.
pub type GraphWriterStats = (usize, usize, usize);

/// Snapshot of an object-level metadata record, used as the input DTO for
/// projection bridge upserts.  Fields mirror the S3 + compliance record
/// shape: bucket/key identity, size + etag + crc64 for content verification,
/// a `miji_level` for confidentiality tier and an optional legal-hold
/// timestamp (`hold_until_ms`, epoch ms).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectMeta {
    pub bucket: String,
    pub key: String,
    pub size_bytes: u64,
    pub etag: String,
    pub crc64_ecma: u64,
    pub miji_level: Option<u8>,
    pub hold_until_ms: Option<i64>,
}

/// Tag-level metadata record (kept in lock-step with ObjectMeta for the
/// projection bridge export surface; serialised alongside audit exports).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagMeta {
    pub k: String,
    pub v: String,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjV {
    pub uri: String,
    pub bucket: String,
    pub size: u64,
    pub etag: String,
    pub miji_level: Option<u8>,
    pub tags: BTreeSet<String>, // tag_ids currently attached
    pub props: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagV {
    pub k: String,
    pub v: String,
}

#[derive(Debug)]
struct GraphState {
    objs: BTreeMap<String, ObjV>,
    tags: BTreeMap<String, TagV>,
    /// (obj_id, tag_id) HAS_TAG edges currently active.
    edges: BTreeSet<(String, String)>,
    soft_deleted: BTreeSet<String>,
    /// (obj_id, tag_id) -> archived_at millisecond timestamp.
    archived_edges: BTreeMap<(String, String), i64>,
    /// Failed CDC events land here; inspectable via `dlq()`.
    dlq: Vec<ObjectTagged>,
    /// Failure injection: remaining count. Each failure decrements.
    failure_remaining: u32,
    failure_total: u32,
    /// Truncation alarms surfaced by TagSet::normalize.
    truncation_audit: Vec<AuditEvent>,
}

/// Mock graph writer. Intended for the fusion-stage integration tests.
#[derive(Debug, Clone)]
pub struct GraphWriter {
    inner: Arc<Mutex<GraphState>>,
    /// Optional live bridge to a projection 2.0 SimpleGraph.  When `Some`,
    /// every successful `upsert_obj_and_tags` and `mark_deleted` call is
    /// mirrored into the wrapped SimpleGraph via the bijection tables kept
    /// inside `ProjectionBridge`.
    pub projection_bridge: Option<Arc<Mutex<ProjectionBridge>>>,
}

impl Default for GraphWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphWriter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(GraphState {
                objs: BTreeMap::new(),
                tags: BTreeMap::new(),
                edges: BTreeSet::new(),
                soft_deleted: BTreeSet::new(),
                archived_edges: BTreeMap::new(),
                dlq: Vec::new(),
                failure_remaining: 0,
                failure_total: 0,
                truncation_audit: Vec::new(),
            })),
            projection_bridge: None,
        }
    }

    // ------ Configuration / injection ------

    /// Configure `n` upcoming upserts to fail with `Error::Injected`.
    /// The failed events are appended to the DLQ; calls to upsert after `n`
    /// return to normal operation.
    pub fn inject_failures(&self, n: u32) {
        let mut s = self.inner.lock();
        s.failure_remaining = n;
        s.failure_total = n;
    }

    /// Builder-style: attach a projection bridge and return the writer.
    /// Every subsequent successful upsert or mark_deleted will be mirrored
    /// into the wrapped `SimpleGraph`.
    pub fn with_projection_bridge(
        mut self,
        b: ::std::sync::Arc<::parking_lot::Mutex<ProjectionBridge>>,
    ) -> Self {
        self.projection_bridge = Some(b);
        self
    }

    /// Attach a projection bridge in-place.  Every subsequent successful
    /// upsert or mark_deleted will be mirrored into the wrapped `SimpleGraph`.
    pub fn set_bridge(&mut self, bridge: ::std::sync::Arc<::parking_lot::Mutex<ProjectionBridge>>) {
        self.projection_bridge = Some(bridge);
    }

    /// Read the current DLQ snapshot.
    pub fn dlq(&self) -> Vec<ObjectTagged> {
        self.inner.lock().dlq.clone()
    }

    /// Truncation audit records produced during upserts.
    pub fn truncation_audit(&self) -> Vec<AuditEvent> {
        self.inner.lock().truncation_audit.clone()
    }

    // ------ Core operations ------

    /// Idempotently upsert an object together with its tag set.
    ///
    /// - If the object already exists and tags match exactly, the write is
    ///   skipped and `Ok(())` is returned.
    /// - Otherwise, any HAS_TAG edge present in the old set but absent from the
    ///   new one is moved into `archived_edges` with `archived_at = now_ms`.
    /// - New HAS_TAG edges are inserted; tags are deduplicated across objects.
    /// - When `miji_level = Some(l)`, the object is tagged with an automatic
    ///   `(level, l)` tag (idempotently) and `props["level"] = l`.
    /// - If the tag count exceeds 50, the tag list is truncated and a
    ///   `TagTruncated` audit event is recorded.
    pub fn upsert_obj_and_tags(
        &self,
        uri: &str,
        bucket: &str,
        size: u64,
        etag: &str,
        tags: &[Tag],
        miji_level: Option<u8>,
    ) -> Result<()> {
        // Handle injection **before** taking the lock so failures do not
        // partially modify state.
        {
            let mut s = self.inner.lock();
            if s.failure_remaining > 0 {
                let total = s.failure_total;
                let remain = s.failure_remaining;
                s.failure_remaining -= 1;
                // record DLQ
                let ev = ObjectTagged {
                    uri: uri.to_string(),
                    tags: tags.to_vec(),
                    ts_ms: now_ms(),
                    dedup_id: sha2_hex(&format!(
                        "{}|{}|{}|{}|{}",
                        uri,
                        bucket,
                        size,
                        etag,
                        total - remain
                    )),
                };
                s.dlq.push(ev);
                return Err(Error::Injected {
                    total,
                    remaining: remain,
                });
            }
            drop(s);
        }

        // Normalize/cap the incoming tags (truncate if > 50) and optionally
        // extend with the miji auto tag.
        let mut norm_tags = normalize_tags_for_graph(tags);
        let mut truncated = false;
        if norm_tags.len() > 50 {
            norm_tags.truncate(50);
            truncated = true;
        }

        if let Some(l) = miji_level {
            let auto = Tag::new("level", l.to_string());
            if !norm_tags.contains(&auto) {
                if norm_tags.len() >= 50 {
                    norm_tags[49] = auto;
                    truncated = true;
                } else {
                    norm_tags.push(auto);
                }
            }
        }

        let obj_id = obj_id_of(uri);

        // compute tag ids
        let mut new_tag_ids: BTreeSet<String> = BTreeSet::new();
        for t in &norm_tags {
            new_tag_ids.insert(tag_id_of(&t.k, &t.v));
        }

        let now = now_ms();
        let mut s = self.inner.lock();

        // Ensure tag vertices exist (dedup across objects).
        for t in &norm_tags {
            let tid = tag_id_of(&t.k, &t.v);
            s.tags.entry(tid).or_insert_with(|| TagV {
                k: t.k.clone(),
                v: t.v.clone(),
            });
        }

        let previous = s.objs.get(&obj_id).cloned();
        let (prev_tag_ids, is_redundant) = match &previous {
            Some(p) => {
                let ids: BTreeSet<String> = p.tags.iter().cloned().collect();
                let same_tags = ids == new_tag_ids;
                let same_level = p.miji_level == miji_level
                    && p.etag == etag
                    && p.bucket == bucket
                    && p.size == size;
                (ids, same_tags && same_level)
            }
            None => (BTreeSet::new(), false),
        };

        let is_soft_dead = s.soft_deleted.remove(&obj_id);
        if is_redundant && !is_soft_dead {
            // Common path: content identical and object not soft-deleted → no-op.
            if truncated {
                s.truncation_audit.push(AuditEvent::new(
                    AuditRecordKind::TagTruncated,
                    uri.to_string(),
                    None,
                    now,
                ));
            }
            self.apply_bridge_hook(uri, bucket, size, etag, miji_level, &norm_tags);
            return Ok(());
        }
        // If we were soft-deleted, treat the upsert as "revive": even with
        // content matching we must re-create HAS_TAG edges (they were
        // archived in mark_deleted).
        if is_soft_dead {
            for tid in &new_tag_ids {
                let key = (obj_id.clone(), tid.clone());
                // Clean up the archived entry for the revived edge so
                // archival counts don't grow unboundedly.
                s.archived_edges.remove(&key);
                s.edges.insert(key);
            }
            if truncated {
                s.truncation_audit.push(AuditEvent::new(
                    AuditRecordKind::TagTruncated,
                    uri.to_string(),
                    None,
                    now,
                ));
            }
            self.apply_bridge_hook(uri, bucket, size, etag, miji_level, &norm_tags);
            return Ok(());
        }

        // Archive edges no longer in the new set.
        for prev in &prev_tag_ids {
            if !new_tag_ids.contains(prev) {
                let key = (obj_id.clone(), prev.clone());
                s.edges.remove(&key);
                s.archived_edges.entry(key).or_insert(now);
            }
        }

        // Add new edges.
        for tid in &new_tag_ids {
            let key = (obj_id.clone(), tid.clone());
            s.edges.insert(key);
        }

        // Build / update ObjV
        let mut props: BTreeMap<String, String> = match &previous {
            Some(p) => p.props.clone(),
            None => BTreeMap::new(),
        };
        if let Some(l) = miji_level {
            props.insert(String::from("level"), l.to_string());
            props.insert(String::from("miji_level"), l.to_string());
        } else {
            props.remove("level");
            props.remove("miji_level");
        }

        let obj = ObjV {
            uri: uri.to_string(),
            bucket: bucket.to_string(),
            size,
            etag: etag.to_string(),
            miji_level,
            tags: new_tag_ids,
            props,
        };
        s.objs.insert(obj_id.clone(), obj);

        // Revive from soft-delete (re-write implies un-delete semantics).
        s.soft_deleted.remove(&obj_id);

        if truncated {
            s.truncation_audit.push(AuditEvent::new(
                AuditRecordKind::TagTruncated,
                uri.to_string(),
                None,
                now,
            ));
        }

        self.apply_bridge_hook(uri, bucket, size, etag, miji_level, &norm_tags);
        Ok(())
    }

    /// Reverse index search: return uris of objects carrying `(k, v)`,
    /// capped at `limit`.
    pub fn query_objects_by_tag(&self, k: &str, v: &str, limit: usize) -> Vec<String> {
        let s = self.inner.lock();
        let tid = tag_id_of(k, v);
        let mut out = Vec::new();
        for (obj_id, _tag_id) in s.edges.range(..) {
            if _tag_id == &tid {
                if let Some(o) = s.objs.get(obj_id) {
                    if out.len() >= limit {
                        break;
                    }
                    out.push(o.uri.clone());
                }
            }
        }
        out
    }

    /// Soft-delete an object: mark it in `soft_deleted` and archive every
    /// currently-active HAS_TAG edge with `archived_at = now_ms`.
    pub fn mark_deleted(&self, uri: &str) {
        let mut s = self.inner.lock();
        let obj_id = obj_id_of(uri);
        s.soft_deleted.insert(obj_id.clone());
        // Find outgoing edges for this object and archive them.
        let to_archive: Vec<(String, String)> = s
            .edges
            .range((obj_id.clone(), String::new())..)
            .take_while(|(o, _)| o == &obj_id)
            .cloned()
            .collect();
        let now = now_ms();
        for key in to_archive {
            s.edges.remove(&key);
            s.archived_edges.insert(key, now);
        }
        drop(s);
        if let Some(bridge_arc) = &self.projection_bridge {
            bridge_arc.lock().remove_object(uri);
        }
    }

    /// `(objs, tags, edges)` counts.
    pub fn stats(&self) -> GraphWriterStats {
        let s = self.inner.lock();
        (s.objs.len(), s.tags.len(), s.edges.len())
    }

    /// Inspect the soft-deleted obj_ids.
    pub fn soft_deleted_ids(&self) -> Vec<String> {
        self.inner.lock().soft_deleted.iter().cloned().collect()
    }

    /// Snapshot of archived edges: `((obj_id, tag_id), archived_at_ms)`.
    pub fn archived_edges(&self) -> Vec<((String, String), i64)> {
        self.inner
            .lock()
            .archived_edges
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    /// Read an object snapshot for test assertions (miji level / props).
    pub fn get_obj(&self, uri: &str) -> Option<ObjV> {
        self.inner.lock().objs.get(&obj_id_of(uri)).cloned()
    }

    /// Mirror the current state of the given URI into the attached bridge
    /// (no-op if no bridge is attached).  This acquires the state lock
    /// briefly to snapshot tag k/v pairs and per-tag usage counts, then
    /// releases it before locking the bridge (lock-order: state → bridge)
    /// so the two locks never deadlock.
    fn apply_bridge_hook(
        &self,
        uri: &str,
        bucket: &str,
        size: u64,
        etag: &str,
        miji_level: Option<u8>,
        norm_tags: &[crate::tag_parser::Tag],
    ) {
        let Some(bridge_arc) = &self.projection_bridge else {
            return;
        };
        // Phase 1: lock state ONLY; snapshot k/v + usage counts.
        let (meta, tag_snaps): (ObjectMeta, Vec<(String, String, u64)>) = {
            let s = self.inner.lock();
            let meta = ObjectMeta {
                bucket: bucket.to_string(),
                key: uri.to_string(),
                size_bytes: size,
                etag: etag.to_string(),
                crc64_ecma: 0,
                miji_level,
                hold_until_ms: None,
            };
            let mut snaps = Vec::with_capacity(norm_tags.len());
            for t in norm_tags {
                let tid = tag_id_of(&t.k, &t.v);
                let usage = s
                    .edges
                    .iter()
                    .filter(|(_, tag_id)| *tag_id == tid)
                    .count() as u64;
                snaps.push((t.k.clone(), t.v.clone(), usage));
            }
            (meta, snaps)
        };
        // Phase 2: lock bridge ONLY; apply upserts + edges.
        let mut bridge = bridge_arc.lock();
        bridge.upsert_object_vertex(uri, &meta);
        for (k, v, usage) in &tag_snaps {
            bridge.upsert_tag_vertex(k, v, *usage);
        }
        for (k, v, _) in &tag_snaps {
            let tag_uri = format!("tag://{k}:{v}");
            bridge.add_has_tag_edge(uri, &tag_uri, "HAS_TAG");
        }
    }
}

// ------ helpers ------

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn obj_id_of(uri: &str) -> String {
    String::from("obj:") + &sha2_hex(uri)
}

pub fn tag_id_of(k: &str, v: &str) -> String {
    format!(
        "tag:{}|{}",
        url_percent_encode(k),
        url_percent_encode(v)
    )
}

pub fn sha2_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn url_percent_encode(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Decode a tag_id produced by `tag_id_of` back into `(k, v)`. Returns `None`
/// if the string is malformed.
pub fn tag_id_decode(tag_id: &str) -> Option<(String, String)> {
    let rest = tag_id.strip_prefix("tag:")?;
    let (ek, ev) = rest.split_once('|')?;
    let k = percent_encoding::percent_decode_str(ek)
        .decode_utf8()
        .ok()?
        .to_string();
    let v = percent_encoding::percent_decode_str(ev)
        .decode_utf8()
        .ok()?
        .to_string();
    Some((k, v))
}

fn normalize_tags_for_graph(tags: &[Tag]) -> Vec<Tag> {
    // Key-normalization mirror of tag_parser for safety.
    use std::collections::HashSet;
    let mut out: Vec<Tag> = Vec::with_capacity(tags.len());
    let mut seen_kv: HashSet<(String, String)> = HashSet::new();
    let mut seen_k: HashSet<String> = HashSet::new();
    for t in tags {
        let k = {
            let mut s = String::new();
            for ch in t.k.chars() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    s.push(ch.to_ascii_lowercase());
                } else {
                    s.push('_');
                }
            }
            if s.chars().count() > 64 {
                s = s.chars().take(64).collect();
            }
            s
        };
        if k.is_empty() {
            continue;
        }
        let v = if t.v.is_empty() {
            String::from("(empty)")
        } else {
            t.v.clone()
        };
        if !seen_kv.insert((k.clone(), v.clone())) {
            continue;
        }
        if !seen_k.insert(k.clone()) {
            // Already seen this key with a different value: first wins.
            continue;
        }
        out.push(Tag { k, v });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_id_encode_decode_roundtrip() {
        for (k, v) in [
            ("content_type", "application/pdf"),
            ("size_bucket", "1MB..1GB"),
            ("a/b:c-d.e", "x y&z=1"),
        ] {
            let id = tag_id_of(k, v);
            let (dk, dv) = tag_id_decode(&id).expect("decode");
            assert_eq!(dk, k);
            assert_eq!(dv, v);
        }
    }

    #[test]
    fn basic_upsert_stats() {
        let g = GraphWriter::new();
        let tags = vec![
            Tag::new("project", "finance"),
            Tag::new("team", "risk"),
            Tag::new("level", "3"),
        ];
        g.upsert_obj_and_tags("s3://b1/f1.pdf", "b1", 1024, "etag1", &tags, None)
            .unwrap();
        let (o, t, e) = g.stats();
        assert_eq!((o, t, e), (1, 3, 3));
    }

    #[test]
    fn failure_injection_pushes_dlq() {
        let g = GraphWriter::new();
        g.inject_failures(2);
        let tags = vec![Tag::new("a", "1")];
        let r1 = g.upsert_obj_and_tags("u1", "b", 1, "e", &tags, None);
        let r2 = g.upsert_obj_and_tags("u2", "b", 1, "e", &tags, None);
        let r3 = g.upsert_obj_and_tags("u3", "b", 1, "e", &tags, None);
        assert!(r1.is_err());
        assert!(r2.is_err());
        assert!(r3.is_ok());
        assert_eq!(g.dlq().len(), 2);
    }
}
