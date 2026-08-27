// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! CDC stage: TagSet + object URI -> ObjectTagged event + AuditEvent.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::audit_sync::{AuditEvent, AuditRecordKind};
use crate::tag_parser::{Tag, TagSet};

/// CDC event emitted when a PutObject has been staged for graph projection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectTagged {
    pub uri: String,
    pub tags: Vec<Tag>,
    pub ts_ms: i64,
    pub dedup_id: String,
}

/// Build an `ObjectTagged` event for the given object and tags.
///
/// Returns the event plus an `AuditEvent` (TagApplied) suitable for appending
/// to the audit hash chain.
pub fn tag_cdc_graph_stage(obj_uri: &str, mut tags: TagSet) -> (ObjectTagged, AuditEvent) {
    let _alarm = tags.normalize();
    let tags = tags.0;

    let ts_ms = now_ms();

    // dedup_id: sha256(uri + ts_ms_salted + concat tags k|v). Using millisecond
    // timestamp makes near-simultaneous reuploads unique while tag content
    // inclusion guarantees same-content upserts within same ms dedupe.
    let mut hasher = Sha256::new();
    hasher.update(obj_uri.as_bytes());
    hasher.update(b"|");
    hasher.update(ts_ms.to_le_bytes());
    hasher.update(b"|");
    for t in &tags {
        hasher.update(t.k.as_bytes());
        hasher.update(b"=");
        hasher.update(t.v.as_bytes());
        hasher.update(b";");
    }
    let digest = hasher.finalize();
    let dedup_id = hex::encode(digest);

    let event = ObjectTagged {
        uri: obj_uri.to_string(),
        tags,
        ts_ms,
        dedup_id: dedup_id.clone(),
    };

    let audit = AuditEvent::new(
        AuditRecordKind::TagApplied,
        obj_uri.to_string(),
        Some(dedup_id),
        ts_ms,
    );

    (event, audit)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_roundtrip_has_dedup_and_tags() {
        let headers: &[(String, String)] = &[
            (String::from("x-amz-meta-project"), String::from("finance")),
            (String::from("Content-Type"), String::from("application/pdf")),
        ];
        let tags = TagSet::from_s3_headers(headers, true, None, 1024);
        let (ev, audit) = tag_cdc_graph_stage("s3://b1/r.pdf", tags);
        assert!(!ev.dedup_id.is_empty());
        assert!(ev.tags.iter().any(|t| t.k == "project"));
        assert!(ev.tags.iter().any(|t| t.k == "size_bucket"));
        assert_eq!(audit.obj_uri, "s3://b1/r.pdf");
    }
}
