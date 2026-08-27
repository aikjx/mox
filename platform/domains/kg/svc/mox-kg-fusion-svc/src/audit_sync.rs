// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! Audit sync: a tiny append-only hash chain of fusion audit events.
//!
//! Each block carries:
//! - `index`: block sequence number (0 = genesis)
//! - `prev_hash`: hex SHA-256 of the previous block header
//! - `ts_ms`: block creation timestamp
//! - `event`: `AuditEvent` (kind, obj_uri, dedup_id, ts_ms)
//! - `hash`: hex SHA-256(this block header) - used for chain verification.
//!
//! Intended strictly for unit tests of the fusion pipeline. Not durable.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::cdc_stage::ObjectTagged;

/// Classification of each audit event produced by the fusion stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditRecordKind {
    /// ObjectTagged event generated (stage output).
    TagApplied,
    /// Tag normalization truncated the tag set to the 50-item cap.
    TagTruncated,
    /// Object was soft-deleted via DELETE.
    ObjectDeleted,
    /// Generic catch-all for future kinds.
    Other,
}

impl AuditRecordKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TagApplied => "tag_applied",
            Self::TagTruncated => "tag_truncated",
            Self::ObjectDeleted => "object_deleted",
            Self::Other => "other",
        }
    }
}

/// An audit event: the logical record being appended to the chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditEvent {
    pub kind: AuditRecordKind,
    pub obj_uri: String,
    pub dedup_id: Option<String>,
    pub ts_ms: i64,
}

impl AuditEvent {
    pub fn new(
        kind: AuditRecordKind,
        obj_uri: String,
        dedup_id: Option<String>,
        ts_ms: i64,
    ) -> Self {
        Self { kind, obj_uri, dedup_id, ts_ms }
    }

    /// Convenience constructor from an ObjectTagged event.
    pub fn from_tagged(kind: AuditRecordKind, ev: &ObjectTagged) -> Self {
        Self {
            kind,
            obj_uri: ev.uri.clone(),
            dedup_id: Some(ev.dedup_id.clone()),
            ts_ms: ev.ts_ms,
        }
    }
}

/// A single block in the hash chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditBlock {
    pub index: u64,
    pub prev_hash: String,
    pub ts_ms: i64,
    pub event: AuditEvent,
    pub hash: String,
}

/// Append-only, SHA-256 linked audit chain.
#[derive(Debug, Clone, Default)]
pub struct AuditChain {
    blocks: Vec<AuditBlock>,
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex::encode(h.finalize())
}

fn compute_hash(b: &AuditBlock) -> String {
    let mut h = Sha256::new();
    h.update(b.index.to_le_bytes());
    h.update(b.prev_hash.as_bytes());
    h.update(b.ts_ms.to_le_bytes());
    h.update(b.event.kind.as_str().as_bytes());
    h.update(b.event.obj_uri.as_bytes());
    if let Some(d) = &b.event.dedup_id {
        h.update(d.as_bytes());
    }
    h.update(b.event.ts_ms.to_le_bytes());
    hex::encode(h.finalize())
}

impl AuditChain {
    /// Create a new empty chain (genesis not added until first append).
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    /// Length of the chain in blocks (excludes the implicit genesis of length 0).
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Reference to the ordered blocks.
    pub fn blocks(&self) -> &[AuditBlock] {
        &self.blocks
    }

    /// Append an `AuditEvent` as a new block, linked to the previous one.
    /// Returns the freshly-committed block.
    pub fn append(&mut self, event: AuditEvent) -> &AuditBlock {
        let index = self.blocks.len() as u64;
        let prev_hash = self
            .blocks
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| String::from("0".repeat(64)));
        let ts_ms = event.ts_ms;
        let mut block = AuditBlock {
            index,
            prev_hash,
            ts_ms,
            event,
            hash: String::new(),
        };
        block.hash = compute_hash(&block);
        self.blocks.push(block);
        self.blocks.last().unwrap()
    }

    /// Convenience: append a `TagApplied` event for an ObjectTagged CDC event.
    pub fn append_tagged(&mut self, tagged: &ObjectTagged) -> &AuditBlock {
        self.append(AuditEvent::from_tagged(AuditRecordKind::TagApplied, tagged))
    }

    /// Verify chain integrity:
    ///
    /// 1. Block 0 must have `prev_hash == 0*64`.
    /// 2. Every block `i>0` must have `prev_hash == blocks[i-1].hash`.
    /// 3. Every block must have `hash == compute_hash(block)`.
    /// 4. Indices must be strictly sequential `0..len-1`.
    pub fn verify(&self) -> bool {
        for (i, b) in self.blocks.iter().enumerate() {
            if b.index as usize != i {
                return false;
            }
            let expected_prev = if i == 0 {
                String::from("0".repeat(64))
            } else {
                self.blocks[i - 1].hash.clone()
            };
            if b.prev_hash != expected_prev {
                return false;
            }
            let h = compute_hash(b);
            if h != b.hash {
                return false;
            }
        }
        true
    }

    /// Hex SHA-256 of an arbitrary string (exposed for tests / callers that
    /// want to hash out-of-band data the same way this chain does).
    pub fn hash_hex(&self, data: &str) -> String {
        hex_digest(data.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_append_and_verify() {
        let mut c = AuditChain::new();
        for i in 0..10u64 {
            c.append(AuditEvent::new(
                AuditRecordKind::TagApplied,
                format!("u{}", i),
                Some(format!("d{}", i)),
                1_700_000_000_000 + (i as i64),
            ));
        }
        assert_eq!(c.len(), 10);
        assert!(c.verify());
    }

    #[test]
    fn tamper_is_detected() {
        let mut c = AuditChain::new();
        for i in 0..5u64 {
            c.append(AuditEvent::new(
                AuditRecordKind::TagApplied,
                format!("u{}", i),
                None,
                1_700_000_000_000 + (i as i64),
            ));
        }
        assert!(c.verify());
        let blocks = unsafe {
            let ptr = &mut c.blocks as *mut Vec<AuditBlock>;
            &mut *ptr
        };
        blocks[2].event.obj_uri = String::from("tamp3r3d");
        assert!(!c.verify());
    }
}
