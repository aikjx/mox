// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

/// Global deny counters for audit statistics (tr14).
pub static DENY_COUNTER_LH: AtomicU64 = AtomicU64::new(0);
pub static DENY_COUNTER_MIJI: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn bump_lh_deny() {
    DENY_COUNTER_LH.fetch_add(1, Ordering::SeqCst);
}
#[inline]
pub fn bump_miji_deny() {
    DENY_COUNTER_MIJI.fetch_add(1, Ordering::SeqCst);
}
pub fn reset_counters() {
    DENY_COUNTER_LH.store(0, Ordering::SeqCst);
    DENY_COUNTER_MIJI.store(0, Ordering::SeqCst);
}

/// A hashed, chained audit block used for integrity verification (tr13).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditBlock {
    pub serial: u64,
    pub timestamp_ms: i64,
    pub prev_hash: String,
    pub record: ComplianceRecord,
    pub this_hash: String,
}

/// Compute sha256 hex of a serialized block body (serial + timestamp + prev_hash + record JSON).
pub fn compute_block_hash(serial: u64, ts_ms: i64, prev_hash: &str, record_json: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(serial.to_le_bytes());
    hasher.update(ts_ms.to_le_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update(record_json.as_bytes());
    let digest = hasher.finalize();
    #[cfg(feature = "hex")]
    {
        hex::encode(digest)
    }
    #[cfg(not(feature = "hex"))]
    {
        // manual hex fallback
        let mut out = String::with_capacity(64);
        for b in digest.iter() {
            out.push_str(&format!("{:02x}", b));
        }
        out
    }
}

impl AuditBlock {
    pub fn new(serial: u64, timestamp_ms: i64, prev_hash: String, record: ComplianceRecord) -> Self {
        let record_json = serde_json::to_string(&record).unwrap_or_default();
        let this_hash = compute_block_hash(serial, timestamp_ms, &prev_hash, &record_json);
        Self { serial, timestamp_ms, prev_hash, record, this_hash }
    }

    pub fn verify_integrity(&self) -> bool {
        let record_json = match serde_json::to_string(&self.record) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let expected = compute_block_hash(self.serial, self.timestamp_ms, &self.prev_hash, &record_json);
        expected == self.this_hash
    }
}

/// Verify a chain of blocks: each (a) hash-integrity passes, (b) serial increments, (c) prev_hash links.
pub fn verify_chain(chain: &[AuditBlock]) -> usize {
    if chain.is_empty() {
        return 0;
    }
    let mut prev_hash: Option<&str> = None;
    let mut expected_serial: u64 = chain[0].serial;
    let mut pass = 0usize;
    for b in chain.iter() {
        if b.serial != expected_serial {
            break;
        }
        if let Some(ph) = prev_hash {
            if b.prev_hash != ph {
                break;
            }
        }
        if !b.verify_integrity() {
            break;
        }
        prev_hash = Some(&b.this_hash);
        expected_serial += 1;
        pass += 1;
    }
    pass
}

/// Compliance audit record variants. Each has a to_record_summary() String.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComplianceRecord {
    LegalHoldDenied {
        serial: u64,
        timestamp_ms: i64,
        actor: String,
        object: String,
        operation: String,
        held_by: String,
        hold_until_ms: i64,
        now_ms: i64,
    },
    MijiAccessDenied {
        serial: u64,
        timestamp_ms: i64,
        reason_code: String,
        actor: String,
        object: String,
        clearance: u8,
        miji_level: u8,
        operation: String,
    },
    LegalHoldPlaced {
        serial: u64,
        timestamp_ms: i64,
        placed_by: String,
        object: String,
        placed_at_ms: i64,
        hold_until_ms: i64,
    },
    MijiEnforceDisabled {
        serial: u64,
        timestamp_ms: i64,
        reason: String,
        operator: String,
    },
    TagApplied {
        serial: u64,
        timestamp_ms: i64,
        tag: String,
        object: String,
        actor: String,
    },
    TagTruncated {
        serial: u64,
        timestamp_ms: i64,
        object: String,
        original_len: u64,
        truncated_len: u64,
    },
    ChecksumMismatch {
        serial: u64,
        timestamp_ms: i64,
        object: String,
        expected: String,
        actual: String,
        algorithm: String,
    },
    MountpathFault {
        serial: u64,
        timestamp_ms: i64,
        mountpath: String,
        error: String,
    },
}

impl ComplianceRecord {
    pub fn to_record_summary(&self) -> String {
        match self {
            ComplianceRecord::LegalHoldDenied { serial, actor, object, operation, held_by, hold_until_ms, now_ms, .. } => {
                format!(
                    "[#{}] LegalHoldDenied actor={} op={} object={} held_by={} hold_until={}ms now={}ms",
                    serial, actor, operation, object, held_by, hold_until_ms, now_ms
                )
            }
            ComplianceRecord::MijiAccessDenied { serial, reason_code, actor, object, clearance, miji_level, operation, .. } => {
                format!(
                    "[#{}] MijiAccessDenied code={} actor={} op={} object={} clearance={} miji={}",
                    serial, reason_code, actor, operation, object, clearance, miji_level
                )
            }
            ComplianceRecord::LegalHoldPlaced { serial, placed_by, object, hold_until_ms, .. } => {
                format!(
                    "[#{}] LegalHoldPlaced by={} object={} until={}ms",
                    serial, placed_by, object, hold_until_ms
                )
            }
            ComplianceRecord::MijiEnforceDisabled { serial, reason, operator, .. } => {
                format!(
                    "[#{}] MijiEnforceDisabled operator={} reason={}",
                    serial, operator, reason
                )
            }
            ComplianceRecord::TagApplied { serial, tag, object, actor, .. } => {
                format!("[#{}] TagApplied tag={} object={} actor={}", serial, tag, object, actor)
            }
            ComplianceRecord::TagTruncated { serial, object, original_len, truncated_len, .. } => {
                format!(
                    "[#{}] TagTruncated object={} original={} truncated={}",
                    serial, object, original_len, truncated_len
                )
            }
            ComplianceRecord::ChecksumMismatch { serial, object, algorithm, expected, actual, .. } => {
                format!(
                    "[#{}] ChecksumMismatch object={} algo={} expected={:.8}.. actual={:.8}..",
                    serial, object, algorithm, expected, actual
                )
            }
            ComplianceRecord::MountpathFault { serial, mountpath, error, .. } => {
                format!("[#{}] MountpathFault path={} error={}", serial, mountpath, error)
            }
        }
    }

    /// Serial for sorting / chain ordering.
    pub fn serial(&self) -> u64 {
        match self {
            ComplianceRecord::LegalHoldDenied { serial, .. } => *serial,
            ComplianceRecord::MijiAccessDenied { serial, .. } => *serial,
            ComplianceRecord::LegalHoldPlaced { serial, .. } => *serial,
            ComplianceRecord::MijiEnforceDisabled { serial, .. } => *serial,
            ComplianceRecord::TagApplied { serial, .. } => *serial,
            ComplianceRecord::TagTruncated { serial, .. } => *serial,
            ComplianceRecord::ChecksumMismatch { serial, .. } => *serial,
            ComplianceRecord::MountpathFault { serial, .. } => *serial,
        }
    }
}

/// Build an HTTP-style "deny reason" header string. Must NOT contain the substring "exists" (tr17).
/// This function intentionally avoids any use of the word "exists".
pub fn format_deny_header(status_code: u16, reason: &str) -> String {
    format!("X-Compliance-Deny: status={}; reason={}; mode=forbid", status_code, reason)
}

/// Simulate an STS assume-role that never escalates clearance (tr18).
/// Returns a session token whose decoded clearance is <= user's original clearance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StsSessionToken {
    pub user_clearance: u8,
    pub assumed_clearance: u8,
    pub role_arn: String,
    pub issued_at_ms: i64,
    pub expires_at_ms: i64,
}

impl StsSessionToken {
    /// Issue token. `requested_clearance` is clamped to <= `user_clearance`.
    pub fn assume_role(
        user_clearance: u8,
        requested_clearance: u8,
        role_arn: &str,
        now_ms: i64,
        ttl_ms: i64,
    ) -> Self {
        let assumed = if requested_clearance <= user_clearance {
            requested_clearance
        } else {
            user_clearance
        };
        Self {
            user_clearance,
            assumed_clearance: assumed,
            role_arn: role_arn.to_string(),
            issued_at_ms: now_ms,
            expires_at_ms: now_ms.saturating_add(ttl_ms),
        }
    }

    /// Decode the effective clearance. Always <= original user clearance.
    pub fn decoded_clearance(&self) -> u8 {
        self.assumed_clearance.min(self.user_clearance)
    }
}
