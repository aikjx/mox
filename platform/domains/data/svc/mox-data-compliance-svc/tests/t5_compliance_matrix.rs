// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

use chrono::Utc;
use std::convert::TryFrom;
use mox_data_compliance_svc::audit_record::{
    bump_lh_deny, bump_miji_deny, reset_counters, AuditBlock, ComplianceRecord, StsSessionToken,
    DENY_COUNTER_LH, DENY_COUNTER_MIJI,
};
use mox_data_compliance_svc::legal_hold::{check_delete, check_overwrite, parse_cli_hold_until, LegalHold};
use mox_data_compliance_svc::miji::{judge_read, judge_write, Clearance, MijiLevel};
use mox_data_compliance_svc::audit_record::format_deny_header;
use mox_data_compliance_svc::audit_record::verify_chain;

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

// ---- tr1: LH delete denied while held ----
#[test]
fn tr1_lh_delete_denied() {
    let placed_at = now_ms() - 1000;
    let hold_until = placed_at + 60_000; // 60s hold
    let lh = LegalHold {
        placed_by: "legal_dept_01".to_string(),
        placed_at_ms: placed_at,
        hold_until_ms: hold_until,
    };
    let now = placed_at + 5000; // still within hold window
    let r = check_delete(Some(&lh), now);
    assert!(r.is_err(), "delete must be denied while LH held");
    let e = r.unwrap_err();
    let msg = format!("{}", e);
    assert!(msg.contains("StillHeld") || msg.contains("held") || msg.contains("delete"));
}

// ---- tr2: LH overwrite/put denied while held ----
#[test]
fn tr2_lh_put_overwrite_denied() {
    let placed_at = now_ms() - 5000;
    let hold_until = placed_at + 3_600_000; // 1h hold
    let lh = LegalHold {
        placed_by: "auditor_alice".to_string(),
        placed_at_ms: placed_at,
        hold_until_ms: hold_until,
    };
    let now = placed_at + 10_000;
    let r = check_overwrite(Some(&lh), now);
    assert!(r.is_err(), "overwrite must be denied while LH held");
}

// ---- tr3: LH expired release at exactly hold_until ----
#[test]
fn tr3_lh_expired_release() {
    let placed_at = 1_700_000_000_000i64;
    let hold_until = placed_at + 1_000_000;
    let lh = LegalHold {
        placed_by: "auto_expire".to_string(),
        placed_at_ms: placed_at,
        hold_until_ms: hold_until,
    };
    // exactly at boundary: now == hold_until => expired => OK
    let r_del = check_delete(Some(&lh), hold_until);
    assert!(r_del.is_ok(), "delete must be OK exactly at hold_until expiration");
    let r_put = check_overwrite(Some(&lh), hold_until);
    assert!(r_put.is_ok(), "overwrite must be OK exactly at hold_until expiration");
    // 1ms before still denied
    let r_del2 = check_delete(Some(&lh), hold_until - 1);
    assert!(r_del2.is_err(), "delete must be denied 1ms before expiration");
}

// ---- tr4: LH cleared (None) => delete/overwrite both OK ----
#[test]
fn tr4_lh_release_clear() {
    let now = now_ms();
    assert!(check_delete(None, now).is_ok());
    assert!(check_overwrite(None, now).is_ok());
}

// ---- tr5: Miji read up denied (Simple Security) ----
#[test]
fn tr5_miji_read_up_denied() {
    let user = Clearance(2); // Secret clearance
    let obj = MijiLevel::Confidential; // level 3
    let r = judge_read(user, obj, true);
    assert!(r.is_err(), "clearance=2 must NOT be allowed to read level=3 (up-read)");
}

// ---- tr6: Miji read down OK ----
#[test]
fn tr6_miji_read_down_ok() {
    let user = Clearance(3); // Confidential clearance
    let obj = MijiLevel::Secret; // level 2
    assert!(judge_read(user, obj, true).is_ok(), "clearance=3 can read level=2 (down-read)");
    // same level also OK
    assert!(judge_read(Clearance(2), MijiLevel::Secret, true).is_ok());
    assert!(judge_read(Clearance(4), MijiLevel::Internal, true).is_ok());
}

// ---- tr7: Miji write star down denied (*-Property) ----
#[test]
fn tr7_miji_write_star_down_denied() {
    let user = Clearance(3); // Confidential
    let obj = MijiLevel::Secret; // level 2 - LOWER
    let r = judge_write(user, obj, true);
    assert!(r.is_err(), "clearance=3 must NOT write level=2 (down-write violates *-Property)");
}

// ---- tr8: Miji write same level OK ----
#[test]
fn tr8_miji_write_same_ok() {
    assert!(judge_write(Clearance(2), MijiLevel::Secret, true).is_ok());
    assert!(judge_write(Clearance(4), MijiLevel::TopSecret, true).is_ok());
}

// ---- tr9: Miji write up OK (low -> high allowed per star property) ----
#[test]
fn tr9_miji_write_up_ok() {
    let user = Clearance(1);
    let obj = MijiLevel::Confidential; // 3
    assert!(judge_write(user, obj, true).is_ok(), "clearance=1 can write UP to level=3");
    assert!(judge_write(Clearance(2), MijiLevel::TopSecret, true).is_ok());
}

// ---- tr10: Miji enforce off => always allow (read & write any combination) ----
#[test]
fn tr10_miji_off_allow() {
    // up-read would normally deny, but enforce=false -> Ok
    assert!(judge_read(Clearance(1), MijiLevel::TopSecret, false).is_ok());
    // down-write would normally deny, but enforce=false -> Ok
    assert!(judge_write(Clearance(4), MijiLevel::Internal, false).is_ok());
}

// ---- tr11: full matrix 100 objs * 4 levels * 4 clearances = 1600 judgments ----
#[test]
fn tr11_matrix_1600() {
    let levels = [
        MijiLevel::Internal,    // 1
        MijiLevel::Secret,      // 2
        MijiLevel::Confidential,// 3
        MijiLevel::TopSecret,   // 4
    ];
    let mut total = 0usize;
    let mut read_ok = 0usize;
    let mut write_ok = 0usize;
    for _obj_idx in 0..100 {
        for lvl in levels.iter() {
            let obj_val = *lvl as u8;
            for clearance in 1u8..=4 {
                let user = Clearance(clearance);
                let r = judge_read(user, *lvl, true);
                // Read rule: clearance >= obj => Ok
                if clearance >= obj_val {
                    assert!(r.is_ok(), "read: clearance={} >= obj={} should be OK (matrix idx #{})", clearance, obj_val, total);
                    read_ok += 1;
                } else {
                    assert!(r.is_err(), "read: clearance={} < obj={} should be Err", clearance, obj_val);
                }
                let w = judge_write(user, *lvl, true);
                // Write rule: clearance <= obj => Ok
                if clearance <= obj_val {
                    assert!(w.is_ok(), "write: clearance={} <= obj={} should be OK", clearance, obj_val);
                    write_ok += 1;
                } else {
                    assert!(w.is_err(), "write: clearance={} > obj={} should be Err", clearance, obj_val);
                }
                total += 1;
            }
        }
    }
    assert_eq!(total, 1600, "must process exactly 1600 judgments");
    // sanity checks: read_ok per (obj, clearance) pair count
    // For each level (4) x 100 objs:
    //   level 1: clearance 1..4 OK => 4 pairs
    //   level 2: clearance 2..4 OK => 3 pairs
    //   level 3: clearance 3..4 OK => 2 pairs
    //   level 4: clearance 4 OK => 1 pair
    // per level block: 10, times 100 objs = 1000? wait no: levels 4 * 4 clearances = 16 per obj
    //   level1: 4 read-ok
    //   level2: 3 read-ok
    //   level3: 2 read-ok
    //   level4: 1 read-ok
    // Total per obj: 10 read-ok, 100 objs => 1000
    assert_eq!(read_ok, 1000, "expected 1000 read-OK outcomes");
    // write: clearance <= obj =>
    //   level1: clearance 1 OK => 1
    //   level2: clearance 1,2 OK => 2
    //   level3: clearance 1,2,3 OK => 3
    //   level4: clearance 1..4 OK => 4
    // per obj: 10, times 100 = 1000
    assert_eq!(write_ok, 1000, "expected 1000 write-OK outcomes");
}

// ---- tr12: LH / Miji union priority: LH held blocks delete even if max clearance ----
#[test]
fn tr12_lh_miji_union() {
    let placed_at = now_ms() - 60_000;
    let hold_until = placed_at + 3_600_000; // 1 hour hold
    let lh = Some(LegalHold {
        placed_by: "regulator_csrc".to_string(),
        placed_at_ms: placed_at,
        hold_until_ms: hold_until,
    });
    let now = placed_at + 10_000;
    // Even top-secret maximum clearance user cannot delete LH-held object
    let _user_top = Clearance(4);
    let del_result = check_delete(lh.as_ref(), now);
    assert!(del_result.is_err(), "LH held must take priority; delete 412 denied even for clearance=4");
    // Overwrite also denied
    let put_result = check_overwrite(lh.as_ref(), now);
    assert!(put_result.is_err());
    // Release the LH, then both OK (Miji doesn't restrict delete/overwrite at API layer per design)
    let lh_none: Option<&LegalHold> = None;
    assert!(check_delete(lh_none, now).is_ok());
    assert!(check_overwrite(lh_none, now).is_ok());
}

// ---- tr13: Audit chain integrity 100 blocks ----
#[test]
fn tr13_audit_chain_integrity() {
    let ts0 = now_ms();
    let mut prev_hash = "genesis".to_string();
    let mut chain: Vec<AuditBlock> = Vec::with_capacity(100);
    for i in 0u64..100 {
        let serial = i + 1; // serial starting at 1, increments
        let ts = ts0 + (i as i64) * 1_000;
        let record = if i % 2 == 0 {
            ComplianceRecord::LegalHoldDenied {
                serial,
                timestamp_ms: ts,
                actor: format!("user_{}", i),
                object: format!("file_{}.dat", i),
                operation: "delete".to_string(),
                held_by: "legal".to_string(),
                hold_until_ms: ts + 86_400_000,
                now_ms: ts,
            }
        } else {
            ComplianceRecord::MijiAccessDenied {
                serial,
                timestamp_ms: ts,
                reason_code: "E_MIJI_READ_UP".to_string(),
                actor: format!("user_{}", i),
                object: format!("doc_{}.pdf", i),
                clearance: 1,
                miji_level: 3,
                operation: "read".to_string(),
            }
        };
        let block = AuditBlock::new(serial, ts, prev_hash.clone(), record);
        assert!(block.verify_integrity(), "block {} integrity must pass", serial);
        prev_hash = block.this_hash.clone();
        chain.push(block);
    }
    let passed = verify_chain(&chain);
    assert_eq!(passed, 100, "all 100 blocks must pass chain integrity verification");

    // serialize/deserialize via JSON and re-verify (append to JSON behavior)
    let json = serde_json::to_string(&chain).expect("json serialize chain");
    let restored: Vec<AuditBlock> = serde_json::from_str(&json).expect("json deserialize chain");
    let passed2 = verify_chain(&restored);
    assert_eq!(passed2, 100, "restored chain must still pass all 100 integrity checks");
}

// ---- tr14: Deny counters increment ----
#[test]
fn tr14_counters() {
    reset_counters();
    let before_lh = DENY_COUNTER_LH.load(std::sync::atomic::Ordering::SeqCst);
    let before_mj = DENY_COUNTER_MIJI.load(std::sync::atomic::Ordering::SeqCst);
    assert_eq!(before_lh, 0);
    assert_eq!(before_mj, 0);

    // Simulate 3 LH denials
    for _ in 0..3 {
        bump_lh_deny();
    }
    // Simulate 7 Miji denials
    for _ in 0..7 {
        bump_miji_deny();
    }

    assert_eq!(DENY_COUNTER_LH.load(std::sync::atomic::Ordering::SeqCst), 3, "LH deny counter must be 3");
    assert_eq!(DENY_COUNTER_MIJI.load(std::sync::atomic::Ordering::SeqCst), 7, "Miji deny counter must be 7");

    // Actual denial paths also bump (combined behavior)
    bump_lh_deny();
    bump_miji_deny();
    bump_miji_deny();
    assert_eq!(DENY_COUNTER_LH.load(std::sync::atomic::Ordering::SeqCst), 4);
    assert_eq!(DENY_COUNTER_MIJI.load(std::sync::atomic::Ordering::SeqCst), 9);

    reset_counters();
    assert_eq!(DENY_COUNTER_LH.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(DENY_COUNTER_MIJI.load(std::sync::atomic::Ordering::SeqCst), 0);
}

// ---- tr15: LH CLI arg parse invalid date ----
#[test]
fn tr15_lh_cli_arg_parse() {
    // valid RFC3339 => Ok
    let good = "2026-12-31T23:59:59Z";
    assert!(parse_cli_hold_until(good).is_ok(), "valid RFC3339 must parse OK");
    // invalid strings => Err
    assert!(parse_cli_hold_until("not-a-date").is_err());
    assert!(parse_cli_hold_until("2026/12/31").is_err());
    assert!(parse_cli_hold_until("").is_err());
    assert!(parse_cli_hold_until("Dec 31 2026").is_err());
    assert!(parse_cli_hold_until("2026-13-40T00:00:00Z").is_err());
}

// ---- tr16: Miji CLI level TryFrom<u8> ----
#[test]
fn tr16_miji_cli_level() {
    // 0 => Err
    assert!(MijiLevel::try_from(0u8).is_err(), "level 0 invalid");
    // 1..4 => Ok
    assert_eq!(MijiLevel::try_from(1u8).unwrap(), MijiLevel::Internal);
    assert_eq!(MijiLevel::try_from(2u8).unwrap(), MijiLevel::Secret);
    assert_eq!(MijiLevel::try_from(3u8).unwrap(), MijiLevel::Confidential);
    assert_eq!(MijiLevel::try_from(4u8).unwrap(), MijiLevel::TopSecret);
    // 5+ => Err
    assert!(MijiLevel::try_from(5u8).is_err(), "level 5 invalid");
    assert!(MijiLevel::try_from(255u8).is_err());
}

// ---- tr17: Deny reason semantic — response header must NOT contain "exists" ----
#[test]
fn tr17_deny_reason_semantic() {
    // Build a typical deny response header string
    let deny_read = format_deny_header(403, "miji_read_up_denied: clearance=2 level=4");
    let deny_write = format_deny_header(412, "legal_hold_still_held: held_by=legal until=2026");
    let deny_generic = format_deny_header(403, "forbidden_by_policy");

    let headers = [&deny_read as &str, &deny_write, &deny_generic];
    for h in headers.iter() {
        // The substring "exists" must NOT appear — to avoid leaking object-existence info
        assert!(
            !h.to_lowercase().contains("exists"),
            "deny header must NOT leak object existence: got '{}'",
            h
        );
    }
}

// ---- tr18: STS assumeRole — NO privilege escalation ----
#[test]
fn tr18_sts_no_privilege_escalation() {
    let user_clearance: u8 = 3; // Confidential user
    let now = now_ms();
    let ttl = 3600_000;

    // Case A: request SAME level => granted same
    let tok_a = StsSessionToken::assume_role(user_clearance, 3, "arn:role:reader", now, ttl);
    assert_eq!(tok_a.decoded_clearance(), 3);
    assert!(tok_a.decoded_clearance() <= user_clearance);

    // Case B: request LOWER level => granted lower (still <= original)
    let tok_b = StsSessionToken::assume_role(user_clearance, 1, "arn:role:intern", now, ttl);
    assert_eq!(tok_b.decoded_clearance(), 1);
    assert!(tok_b.decoded_clearance() <= user_clearance);

    // Case C: request HIGHER level (escalation attempt) => CLAMPED to user clearance
    let tok_c = StsSessionToken::assume_role(user_clearance, 5, "arn:role:admin-try", now, ttl);
    assert_eq!(tok_c.decoded_clearance(), 3, "requested 5 must be clamped to user clearance 3");
    assert!(tok_c.decoded_clearance() <= user_clearance);

    // Case D: request TopSecret (4) vs user=3 => must clamp
    let tok_d = StsSessionToken::assume_role(user_clearance, 4, "arn:role:topsecret-try", now, ttl);
    assert_eq!(tok_d.decoded_clearance(), 3, "requested 4 must be clamped to 3");
    assert!(tok_d.decoded_clearance() <= user_clearance);

    // General invariant for 100 random-ish combinations: decoded <= original
    use rand::Rng;
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let uc: u8 = rng.gen_range(1..=4);
        let rc: u8 = rng.gen_range(0..=255);
        let t = StsSessionToken::assume_role(uc, rc, "arn:role:rand", now, ttl);
        assert!(
            t.decoded_clearance() <= uc,
            "escalation detected: user={} requested={} got session={}",
            uc, rc, t.decoded_clearance()
        );
    }
}
