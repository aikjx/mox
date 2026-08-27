// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! mox-sdk-cloud integration tests — in-memory facade only.
//!
//! Strategy:
//! * Verify the 30-item example ID manifest (`CLOUD_EXAMPLE_IDS`).
//! * Exercise every public API bucket to confirm basic paths return `Ok(...)`.
//! * All tests use `CloudClient::new()` (no network).

use mox_cloud_sdk::*;

// ---------------------------------------------------------------------------
// 1. Manifest: exactly 30 example IDs, well-formed, stable order
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t01_example_ids_manifest_30_count_and_shape() {
    assert_eq!(CLOUD_EXAMPLE_IDS.len(), 30, "must declare exactly 30 cloud examples");
    for (i, id) in CLOUD_EXAMPLE_IDS.iter().enumerate() {
        let expected_prefix = format!("cloud-{:03}_", i + 1);
        assert!(
            id.starts_with(&expected_prefix),
            "index {i}: id {id:?} must start with {expected_prefix:?}"
        );
        assert!(id.len() > expected_prefix.len(), "id {id:?} missing topic suffix");
    }
}

// ---------------------------------------------------------------------------
// 2. Bucket CRUD
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t02_bucket_create_list_head_delete_roundtrip() {
    let c = CloudClient::new();
    // empty by default
    assert!(c.list_buckets().await.unwrap().is_empty());
    c.create_bucket("bk-a").await.unwrap();
    c.create_bucket("bk-b").await.unwrap();
    let list = c.list_buckets().await.unwrap();
    assert_eq!(list.len(), 2);
    let h = c.head_bucket("bk-a").await.unwrap();
    assert_eq!(h.name, "bk-a");
    assert_eq!(h.acl, "private");
    c.delete_bucket("bk-a").await.unwrap();
    let list2 = c.list_buckets().await.unwrap();
    assert_eq!(list2.len(), 1);
    // head missing -> NotFound
    let err = c.head_bucket("bk-a").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)), "got: {:?}", err);
}

#[tokio::test]
async fn t03_bucket_set_acl_and_persist() {
    let c = CloudClient::new();
    c.create_bucket("pub").await.unwrap();
    c.set_bucket_acl("pub", "public-read").await.unwrap();
    let info = c.head_bucket("pub").await.unwrap();
    assert_eq!(info.acl, "public-read");
    // set ACL on missing bucket -> NotFound
    let err = c.set_bucket_acl("nope", "private").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 3. Object: put / get / delete / list / copy
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t04_object_put_get_delete_and_implicit_bucket() {
    let c = CloudClient::new();
    let etag = c.put_object("b", "k", b"abc".to_vec()).await.unwrap();
    assert!(!etag.is_empty());
    assert!(c.head_bucket("b").await.is_ok(), "put_object must implicitly create bucket");
    assert_eq!(c.get_object("b", "k").await.unwrap(), b"abc");
    c.delete_object("b", "k").await.unwrap();
    let err = c.get_object("b", "k").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
    // delete missing idempotent (S3 semantics) -> Ok
    c.delete_object("b", "k").await.unwrap();
}

#[tokio::test]
async fn t05_list_prefix_limit_and_sort() {
    let c = CloudClient::new();
    for i in 0..10u8 {
        c.put_object("x", &format!("p/{:02}.log", i), vec![i; 4]).await.unwrap();
    }
    c.put_object("x", "other.bin", vec![0u8; 1]).await.unwrap();
    let all = c.list_prefix("x", "p/", None).await.unwrap();
    assert_eq!(all.len(), 10);
    assert!(all.windows(2).all(|w| w[0].key < w[1].key), "must be sorted");
    let limited = c.list_prefix("x", "p/", Some(3)).await.unwrap();
    assert_eq!(limited.len(), 3);
}

#[tokio::test]
async fn t06_copy_object_src_missing_is_notfound() {
    let c = CloudClient::new();
    let err = c.copy_object("s", "k", "d", "k2").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)), "got: {:?}", err);
    c.put_object("s", "k", b"V".to_vec()).await.unwrap();
    c.copy_object("s", "k", "d", "k2").await.unwrap();
    assert_eq!(c.get_object("d", "k2").await.unwrap(), b"V");
}

// ---------------------------------------------------------------------------
// 4. Multipart upload
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t07_multipart_complete_assembles_parts_in_order() {
    let c = CloudClient::new();
    let uid = c.create_multipart_upload("big", "file").await.unwrap();
    let mut parts = Vec::new();
    for n in 1..=5u16 {
        let pe = c.upload_part("big", "file", &uid, n, vec![n as u8; 100]).await.unwrap();
        parts.push(pe);
    }
    c.complete_multipart_upload("big", "file", &uid, parts).await.unwrap();
    let assembled = c.get_object("big", "file").await.unwrap();
    assert_eq!(assembled.len(), 500);
    // first 100 bytes = 0x01, second 100 = 0x02, ...
    for i in 0..5usize {
        assert_eq!(assembled[i * 100], (i + 1) as u8);
    }
}

#[tokio::test]
async fn t08_multipart_abort_cleans_up() {
    let c = CloudClient::new();
    let uid = c.create_multipart_upload("ab", "f").await.unwrap();
    c.upload_part("ab", "f", &uid, 1, vec![0u8; 10]).await.unwrap();
    c.abort_multipart_upload(&uid).await.unwrap();
    let err = c.complete_multipart_upload("ab", "f", &uid, vec![]).await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 5. STS assume + chain + signature
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t09_sts_assume_duration_bounds() {
    let c = CloudClient::new();
    let t = c.sts_assume_role("arn:role:r1", 900).await.unwrap();
    assert_eq!(t.duration_secs, 900);
    assert!(!t.access_key.is_empty());
    assert!(!t.session_token.is_empty());
    // 3600s exceeds max 1800 -> StsRejected
    let err = c.sts_assume_role("arn:role:r2", 3600).await.unwrap_err();
    assert!(matches!(err, CloudError::StsRejected(_)), "got: {:?}", err);
}

#[tokio::test]
async fn t10_sts_chain_and_signature_verify() {
    let c = CloudClient::new();
    let chain = c.sts_assume_chain(&["arn:a", "arn:b", "arn:c"], 600).await.unwrap();
    assert_eq!(chain.len(), 3);
    // empty chain -> InvalidRequest
    let err = c.sts_assume_chain(&[], 100).await.unwrap_err();
    assert!(matches!(err, CloudError::InvalidRequest(_)));
    // verify signature against the last issued token (deterministic marker passes)
    let last = chain.last().unwrap();
    let ok = c.sts_verify_signature(&last.session_token, "sig-valid-t10").await.unwrap();
    assert!(ok);
    // unknown session token -> NotFound
    let err2 = c.sts_verify_signature("no-such-token", "any").await.unwrap_err();
    assert!(matches!(err2, CloudError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 6. IAM put / get / eval deny-first
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t11_iam_put_get_and_deny_first_eval() {
    let c = CloudClient::new();
    let doc = r#"{"Effect":"Deny","Action":"s3:PutObject","Resource":"critical-bucket"}"#
        .to_string();
    c.iam_put_policy(IamPolicy {
        name: "lock".into(),
        document: doc,
        version: "1".into(),
    })
    .await
    .unwrap();
    let p = c.iam_get_policy("lock").await.unwrap();
    assert_eq!(p.name, "lock");
    // Unknown policy -> NotFound
    let err = c.iam_get_policy("does-not-exist").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
    // deny-first fires when policy matches action + resource
    let err = c
        .iam_eval_policy(&["lock"], "s3:PutObject", "critical-bucket")
        .await
        .unwrap_err();
    assert!(matches!(err, CloudError::IamDeny(_)), "got: {:?}", err);
    // different action passes (no match)
    c.iam_eval_policy(&["lock"], "s3:GetObject", "critical-bucket")
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// 7. Quota set / get / check with retry-after
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t12_quota_set_get_and_retry_after_on_0rpm() {
    let c = CloudClient::new();
    c.quota_set("api", 50, 10).await.unwrap();
    let q = c.quota_get("api").await.unwrap();
    assert_eq!(q.requests_per_minute, 50);
    assert_eq!(q.burst, 10);
    c.quota_check("api", 1).await.unwrap();
    // unknown scope -> NotFound
    let err = c.quota_get("missing").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
    // 0 rpm disabled scope -> QuotaExceeded with retry-after > 0
    c.quota_set("disabled", 0, 0).await.unwrap();
    let err = c.quota_check("disabled", 1).await.unwrap_err();
    match err {
        CloudError::QuotaExceeded(ra) => assert!(ra > 0),
        other => panic!("expected QuotaExceeded, got {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// 8. WORM retention / legal-hold / compliance immutability
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t13_worm_retention_and_legal_hold_toggle() {
    let c = CloudClient::new();
    c.put_object("w", "f", b"x".to_vec()).await.unwrap();
    c.worm_put_retention("w", "f", "governance", 86400).await.unwrap();
    let r = c.worm_get("w", "f").await.unwrap();
    assert_eq!(r.mode, "governance");
    assert_eq!(r.retain_until, 86400);
    assert!(!r.legal_hold);
    c.worm_set_legal_hold("w", "f", true).await.unwrap();
    assert!(c.worm_get("w", "f").await.unwrap().legal_hold);
    c.worm_set_legal_hold("w", "f", false).await.unwrap();
    assert!(!c.worm_get("w", "f").await.unwrap().legal_hold);
    // no worm for unknown key -> NotFound
    let err = c.worm_get("w", "missing").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
}

#[tokio::test]
async fn t14_worm_compliance_immutable_blocks_overwrite_and_delete() {
    let c = CloudClient::new();
    c.put_object("sec", "x", b"v".to_vec()).await.unwrap();
    c.worm_put_retention("sec", "x", "compliance", 1_000_000).await.unwrap();
    // Cannot re-set retention on compliance-mode objects
    let err = c
        .worm_put_retention("sec", "x", "governance", 1)
        .await
        .unwrap_err();
    assert!(matches!(err, CloudError::WormLocked(_)), "got: {:?}", err);
    // Cannot delete compliance-locked object
    let err = c.delete_object("sec", "x").await.unwrap_err();
    assert!(matches!(err, CloudError::WormLocked(_)), "got: {:?}", err);
}

// ---------------------------------------------------------------------------
// 9. Lifecycle rules + stats + restore
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t15_lifecycle_rules_stats_restore_path() {
    let c = CloudClient::new();
    for i in 0..5u8 {
        c.put_object("lb", &format!("f{}", i), vec![i; 128]).await.unwrap();
    }
    c.lifecycle_put_rule(
        "lb",
        LifecycleRule {
            id: "hw".into(),
            from_storage_class: "hot".into(),
            to_storage_class: "warm".into(),
            after_days: 30,
            prefix: String::new(),
        },
    )
    .await
    .unwrap();
    c.lifecycle_put_rule(
        "lb",
        LifecycleRule {
            id: "wc".into(),
            from_storage_class: "warm".into(),
            to_storage_class: "cold".into(),
            after_days: 180,
            prefix: String::new(),
        },
    )
    .await
    .unwrap();
    let rules = c.lifecycle_list_rules("lb").await.unwrap();
    assert_eq!(rules.len(), 2);
    let stats = c.lifecycle_bucket_stats("lb").await.unwrap();
    assert_eq!(stats.bucket, "lb");
    assert_eq!(stats.hot_bytes + stats.warm_bytes + stats.cold_bytes, 5 * 128);
    assert!(stats.transitioned_last_30d > 0);
    // restore existing object -> Ok, missing -> NotFound
    c.lifecycle_restore("lb", "f0", 7).await.unwrap();
    let err = c.lifecycle_restore("lb", "nope", 3).await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 10. DengBao HashChain: append 1k blocks + verify
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t16_dbhc_append_then_verify_chain_honest() {
    let c = CloudClient::new();
    let id = "chain-test-t16";
    c.dbhc_create_chain(id).await.unwrap();
    let last = c.dbhc_append_blocks(id, 200).await.unwrap();
    assert_eq!(last, 200);
    let ok = c.dbhc_verify_chain(id).await.unwrap();
    assert!(ok);
    // Duplicate create should fail with InvalidRequest
    let err = c.dbhc_create_chain(id).await.unwrap_err();
    assert!(matches!(err, CloudError::InvalidRequest(_)));
    // verify unknown chain -> NotFound
    let err = c.dbhc_verify_chain("no-such").await.unwrap_err();
    assert!(matches!(err, CloudError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 11. State sharing across clones (Arc<Mutex<...>> semantics)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t17_client_clones_share_internal_state() {
    let a = CloudClient::new();
    let b = a.clone();
    a.put_object("shared", "k", b"hello".to_vec()).await.unwrap();
    assert_eq!(b.get_object("shared", "k").await.unwrap(), b"hello");
    b.create_bucket("also-shared").await.unwrap();
    assert!(a.head_bucket("also-shared").await.is_ok());
}
