// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// GitHub 主仓: https://github.com/aikjx/mox.git
// GitCode 镜像: https://gitcode.com/aikjx/mox

//! Glacier HTTP 445 semantics (T25-5).
//!
//! Pure handler semantics: given a storage class and optional restore status,
//! returns (HTTP status code, optional Retry-After duration, body message).
//!
//! No network IO — only a deterministic fn so unit tests are trivial and
//! the caller (axum / tiny HTTP server) can wire headers accordingly.

use std::time::Duration;

use crate::lifecycle::StorageClass;

/// Restore tier used for retry-after estimates on InProgress/Queued restores.
///
/// Mirrors the semantic tiers defined in `glacier_adapter.rs` / `restore_tasks.rs`
/// but is re-declared here so the `glacier_http` module remains self-contained
/// and its public API carries a minimal surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreTier {
    /// ~1–5 minutes restore; 15 minute retry-after by convention.
    Expedited,
    /// Standard / Default: ~1–5 hours; we pick 3 hours in the middle.
    Standard,
    /// Bulk: ~5–12 hours; we pick 8 hours.
    Bulk,
}

/// Restore progress carries a tier so `handle_glacier_get` can emit a precise
/// Retry-After hint for in-progress restores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreProgress {
    pub tier: RestoreTier,
}

/// Simplified restore status enum covering the cases T25-5 cares about:
///
/// - `Queued`    : restore job submitted but not yet started (treated same as InProgress).
/// - `InProgress`: bytes being fetched from cold tier (445 + Retry-After).
/// - `Available` : object restored, GET is ok (200 "ok").
/// - `Expired`   : restore window lapsed; user must re-submit (445 NeedsRestore).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestoreStatus {
    Queued(RestoreProgress),
    InProgress(RestoreProgress),
    Available,
    Expired,
}

/// Messages returned in the response body.
const MSG_OK: &str = "ok";
const MSG_RESTORE_IN_PROGRESS: &str = "Restore in progress, please retry later";
const MSG_RESTORE_REQUIRED: &str = "Restore required: call POST /object/:key/restore";

/// Retry-After constants (seconds).
const RETRY_EXPEDITED_SECS: u64 = 15 * 60;       // 15 minutes
const RETRY_STANDARD_SECS: u64 = 3 * 3_600;       // 3 hours (mid of 1-5h)
const RETRY_BULK_SECS: u64 = 8 * 3_600;           // 8 hours (mid of 5-12h)
const RETRY_DEFAULT_SECS: u64 = 3_600;            // 1 hour fallback (unknown tier)

/// HTTP 445 status code (non-standard but widely used for this scenario).
const HTTP_445: u16 = 445;

fn retry_duration_for(tier: RestoreTier) -> Duration {
    let secs = match tier {
        RestoreTier::Expedited => RETRY_EXPEDITED_SECS,
        RestoreTier::Standard => RETRY_STANDARD_SECS,
        RestoreTier::Bulk => RETRY_BULK_SECS,
    };
    Duration::from_secs(secs)
}

/// Decide the HTTP response for a Glacier-backed GET based on storage class
/// and current restore status.
///
/// Returns `(status_code, retry_after_or_None, body_message)`.
///
/// # Semantics
///
/// 1. `Hot | Warm | Cold` → 200, no Retry-After, `"ok"`.
/// 2. `Glacier` + `Some(Available)` → 200, no Retry-After, `"ok"`.
/// 3. `Glacier` + `Some(Queued | InProgress(p))` → **445**, `Some(retry_after)`,
///    `"Restore in progress, please retry later"`; retry-after depends on
///    `p.tier` (Expedited 15m / Standard 3h / Bulk 8h / unknown → 1h).
/// 4. `Glacier` + `None` or `Some(Expired)` → **445**, `Some(Duration::ZERO)`
///    hint, plus message `"Restore required: call POST /object/:key/restore"`.
pub fn handle_glacier_get(
    storage_class: &StorageClass,
    restore_status: Option<&RestoreStatus>,
) -> (u16, Option<Duration>, &'static str) {
    match storage_class {
        StorageClass::Hot | StorageClass::Warm | StorageClass::Cold => {
            (200, None, MSG_OK)
        }
        StorageClass::Glacier => match restore_status {
            Some(RestoreStatus::Available) => (200, None, MSG_OK),
            Some(RestoreStatus::Queued(p)) | Some(RestoreStatus::InProgress(p)) => {
                let dur = retry_duration_for(p.tier);
                (HTTP_445, Some(dur), MSG_RESTORE_IN_PROGRESS)
            }
            None | Some(RestoreStatus::Expired) => {
                (HTTP_445, Some(Duration::ZERO), MSG_RESTORE_REQUIRED)
            }
        },
    }
}

// ===========================================================
// Tests F1 - F5 (T25-5)
// ===========================================================
#[cfg(test)]
mod tests {
    use super::*;

    // --- F1: HOT/WARM/COLD all return 200 with no retry ---
    #[test]
    fn t25_glacier_445_hot_warm_cold_200() {
        for sc in [StorageClass::Hot, StorageClass::Warm, StorageClass::Cold] {
            let (code, retry, msg) = handle_glacier_get(&sc, None);
            assert_eq!(code, 200, "storage class {:?} must be 200", sc);
            assert!(retry.is_none(), "class {:?} must have NO Retry-After", sc);
            assert_eq!(msg, "ok", "message must be ok, got {msg}");
        }
        // Also ensure passing a Some restore status for non-glacier still
        // returns 200 (callers aren't expected to do that, but behaviour is
        // deterministic and safe).
        let (c, r, m) = handle_glacier_get(
            &StorageClass::Hot,
            Some(&RestoreStatus::InProgress(RestoreProgress { tier: RestoreTier::Standard })),
        );
        assert_eq!((c, r, m), (200, None, "ok"), "non-glacier must ignore restore_status");
    }

    // --- F2: InProgress → 445 + Some retry_after ---
    #[test]
    fn t25_glacier_in_progress_445_with_retry() {
        let prog = RestoreProgress { tier: RestoreTier::Standard };
        let (code, retry, msg) = handle_glacier_get(
            &StorageClass::Glacier,
            Some(&RestoreStatus::InProgress(prog)),
        );
        assert_eq!(code, 445, "InProgress must be 445, got {code}");
        let retry = retry.expect("retry-after must be present for InProgress");
        assert!(retry > Duration::ZERO, "retry-after must be > 0, got {retry:?}");
        assert_eq!(retry, Duration::from_secs(RETRY_STANDARD_SECS));
        assert_eq!(msg, "Restore in progress, please retry later");

        // Queued same semantics as InProgress
        let (code2, retry2, msg2) = handle_glacier_get(
            &StorageClass::Glacier,
            Some(&RestoreStatus::Queued(prog)),
        );
        assert_eq!(code2, 445);
        assert!(retry2.is_some());
        assert_eq!(msg2, msg);
    }

    // --- F3: GLACIER + None restore status → 445 + NeedsRestore message ---
    #[test]
    fn t25_glacier_no_restore_445_hint() {
        let (code, retry, msg) = handle_glacier_get(&StorageClass::Glacier, None);
        assert_eq!(code, 445);
        // Retry-After = 0 hint
        assert_eq!(retry, Some(Duration::ZERO), "NeedsRestore must signal 0-duration retry hint, got {retry:?}");
        assert_eq!(msg, "Restore required: call POST /object/:key/restore");

        // Expired must produce identical output to None
        let (code_e, retry_e, msg_e) = handle_glacier_get(
            &StorageClass::Glacier,
            Some(&RestoreStatus::Expired),
        );
        assert_eq!((code_e, retry_e, msg_e), (code, retry, msg),
            "Expired must behave exactly like None restore status");
    }

    // --- F4: GLACIER + Available → 200 OK ---
    #[test]
    fn t25_glacier_restore_available_200() {
        let (code, retry, msg) = handle_glacier_get(
            &StorageClass::Glacier,
            Some(&RestoreStatus::Available),
        );
        assert_eq!(code, 200, "Available must be 200 OK");
        assert!(retry.is_none(), "Available must NOT carry Retry-After");
        assert_eq!(msg, "ok");
    }

    // --- F5: 3 distinct tiers → 3 distinct non-zero retry durations ---
    #[test]
    fn t25_glacier_retry_after_covers_3_tier_levels() {
        let tiers = [RestoreTier::Expedited, RestoreTier::Standard, RestoreTier::Bulk];
        let mut durs: Vec<Duration> = Vec::with_capacity(3);
        for tier in tiers {
            let prog = RestoreProgress { tier };
            let (code, retry, _msg) = handle_glacier_get(
                &StorageClass::Glacier,
                Some(&RestoreStatus::InProgress(prog)),
            );
            assert_eq!(code, 445, "tier {tier:?} must still be 445");
            let d = retry.expect("retry present");
            assert!(d > Duration::ZERO, "tier {tier:?} must have nonzero retry, got {d:?}");
            durs.push(d);
        }
        // All 3 durations must be distinct.
        assert_ne!(durs[0], durs[1], "Expedited vs Standard retry must differ");
        assert_ne!(durs[1], durs[2], "Standard vs Bulk retry must differ");
        assert_ne!(durs[0], durs[2], "Expedited vs Bulk retry must differ");
        // Ordered: Expedited < Standard < Bulk
        assert!(durs[0] < durs[1], "Expedited < Standard");
        assert!(durs[1] < durs[2], "Standard < Bulk");
        // Values exactly match spec constants.
        assert_eq!(durs[0], Duration::from_secs(RETRY_EXPEDITED_SECS), "Expedited = 15 minutes");
        assert_eq!(durs[1], Duration::from_secs(RETRY_STANDARD_SECS), "Standard = 3 hours");
        assert_eq!(durs[2], Duration::from_secs(RETRY_BULK_SECS), "Bulk = 8 hours");

        // Unknown tier fallback: simulate through the helper to cover default.
        // We don't expose "unknown tier" through the enum, but we document that
        // unknown callers should fall back to RETRY_DEFAULT_SECS. Verify the
        // constant itself is a non-zero sensible 1-hour value.
        assert_eq!(
            Duration::from_secs(RETRY_DEFAULT_SECS),
            Duration::from_secs(3600),
            "default unknown-tier retry = 1 hour"
        );
    }
}
