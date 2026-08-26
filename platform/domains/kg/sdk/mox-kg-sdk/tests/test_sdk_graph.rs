//! mox-sdk-graph integration tests — in-memory facade.
//!
//! Coverage:
//! * 30-example ID manifest
//! * CDC consumer lifecycle (new / next / resume / write / dedup / lag / rotate)
//! * Spark seed / paged reads / bulk write / idempotent upsert / roundtrip stats
//! * Projection define + run (type, community, label, attrs, degree filters)
//! * AC-15 fault matrix (F1, F3, F6, F7, F8, F12, F13, F14)
//! * Clone state sharing

use mox_kg_sdk::*;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t01_example_ids_manifest_30_count_and_shape() {
    assert_eq!(GRAPH_EXAMPLE_IDS.len(), 30, "exactly 30 graph examples required");
    for (i, id) in GRAPH_EXAMPLE_IDS.iter().enumerate() {
        let expected_prefix = format!("graph-{:03}_", i + 1);
        assert!(
            id.starts_with(&expected_prefix),
            "index {i}: id {id:?} must start with {expected_prefix:?}"
        );
        assert!(id.len() > expected_prefix.len());
    }
}

// ---------------------------------------------------------------------------
// 2. CDC suite (7 operations in one shot to verify contracts)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t02_cdc_full_lifecycle_new_next_resume_write_dedup_lag_rotate() {
    let g = GraphClient::new();

    // 2a. new
    let c = g.cdc_new_consumer("topicX", "c0").await.unwrap();
    assert_eq!(c.id, "c0");
    assert_eq!(c.topic, "topicX");
    assert_eq!(c.offset, 0);
    // Unknown consumer get → NotFound
    let err = g.cdc_get_consumer("ghost").await.unwrap_err();
    assert!(matches!(err, GraphError::NotFound(_)), "got: {:?}", err);

    // 2b. write + next_blocking consumes sequentially
    g.cdc_write_records(10, "row").await.unwrap();
    for i in 0..10 {
        let r = g.cdc_next_blocking("c0").await.unwrap();
        assert_eq!(r.offset, i);
    }
    // 10 consumed, no more available → CdcEndOfStream
    let err = g.cdc_next_blocking("c0").await.unwrap_err();
    assert!(matches!(err, GraphError::CdcEndOfStream(_)), "got: {:?}", err);

    // 2c. resume offset
    g.cdc_resume_offset("c0", 3).await.unwrap();
    let r = g.cdc_next_blocking("c0").await.unwrap();
    assert_eq!(r.offset, 3);

    // 2d. dedup bump accumulates
    assert_eq!(g.cdc_dedup_bump("c0", 7).await.unwrap(), 7);
    assert_eq!(g.cdc_dedup_bump("c0", 3).await.unwrap(), 10);

    // 2e. lag monitor writes through
    g.cdc_lag_sample("c0", 1234).await.unwrap();
    assert_eq!(g.cdc_get_consumer("c0").await.unwrap().last_lag_ms, 1234);

    // 2f. rotate consumer id preserves offset + dedup
    let c2 = g.cdc_rotate_consumer("c0", "c1").await.unwrap();
    assert_eq!(c2.id, "c1");
    assert_eq!(c2.offset, 4); // resumed at 3, then next_blocking consumed 3, now at 4
    assert_eq!(c2.dedup_count, 10);
    let err = g.cdc_get_consumer("c0").await.unwrap_err();
    assert!(matches!(err, GraphError::NotFound(_)));
}

// ---------------------------------------------------------------------------
// 3. Spark: seed, paged reads, bulk write
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t03_spark_seed_and_paged_readers_and_bulk() {
    let g = GraphClient::new();

    // edges without nodes → InvalidRequest
    let err = g.spark_seed_edges(10).await.unwrap_err();
    assert!(matches!(err, GraphError::InvalidRequest(_)), "got: {:?}", err);

    g.spark_seed_nodes(12).await.unwrap();
    g.spark_seed_edges(40).await.unwrap();
    let nodes = g.list_nodes().await.unwrap();
    let edges = g.list_edges().await.unwrap();
    assert_eq!(nodes.len(), 12);
    assert_eq!(edges.len(), 40);

    // Page size 5 over 12 nodes → 3 pages (5, 5, 2)
    let p1 = g.spark_reader_nodes_paged(1, 5).await.unwrap();
    assert_eq!(p1.items.len(), 5);
    assert_eq!(p1.total, 12);
    let p2 = g.spark_reader_nodes_paged(2, 5).await.unwrap();
    assert_eq!(p2.items.len(), 5);
    let p3 = g.spark_reader_nodes_paged(3, 5).await.unwrap();
    assert_eq!(p3.items.len(), 2);

    // Edges paged symmetrically
    let ep = g.spark_reader_edges_paged(1, 25).await.unwrap();
    assert_eq!(ep.items.len(), 25);
    assert_eq!(ep.total, 40);

    // Bulk write with explicit IDs adds them; second bulk with same IDs counts as update
    let (n_new, e_new) = g
        .spark_writer_bulk(
            (0..5u32)
                .map(|i| Node {
                    id: 1000 + i as i64,
                    label: "X".into(),
                    typ: "Y".into(),
                    community: 0,
                    attrs: HashMap::new(),
                })
                .collect(),
            (0..5u32)
                .map(|i| Edge {
                    id: 10000 + i as i64,
                    src: 0,
                    dst: 1,
                    label: "R".into(),
                    weight: 1.0,
                })
                .collect(),
        )
        .await
        .unwrap();
    assert_eq!(n_new, 5);
    assert_eq!(e_new, 5);
    let stats = g.spark_stats().await.unwrap();
    assert_eq!(stats.nodes_written, 5);
    assert_eq!(stats.edges_written, 5);
}

// ---------------------------------------------------------------------------
// 4. Spark idempotent upsert + roundtrip counters
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t04_spark_idempotent_and_roundtrip_stats() {
    let g = GraphClient::new();
    let batch: Vec<Node> = (0..10)
        .map(|i| Node {
            id: i,
            label: "A".into(),
            typ: "B".into(),
            community: 0,
            attrs: HashMap::new(),
        })
        .collect();
    // Pass 1: all new → 10 applied, 0 skipped
    let (a1, s1) = g.spark_upsert(batch.clone()).await.unwrap();
    assert_eq!(a1, 10);
    assert_eq!(s1, 0);
    // Pass 2: identical → 0 applied, 10 skipped (idempotent)
    let (a2, s2) = g.spark_upsert(batch.clone()).await.unwrap();
    assert_eq!(a2, 0);
    assert_eq!(s2, 10);

    let st = g.spark_stats().await.unwrap();
    assert_eq!(st.upserts_applied, 10);
    assert_eq!(st.idempotent_skips, 10);
    assert_eq!(st.roundtrips, 0);

    // 2k nodes / 3k edges roundtrip → count increment
    let (n, e) = g.spark_inc_roundtrip(2000, 3000).await.unwrap();
    assert!(n >= 2000);
    assert!(e >= 3000);
    let st2 = g.spark_stats().await.unwrap();
    assert_eq!(st2.roundtrips, 1);
}

// ---------------------------------------------------------------------------
// 5. Projection suite: 8 variants smoke-tested together
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t05_projection_define_and_run_eight_filters() {
    let g = GraphClient::new();
    g.spark_seed_nodes(280).await.unwrap(); // 280/7 communities = 40 each
    g.spark_seed_edges(2000).await.unwrap();
    // Decorate nodes 0..=19 with out_attrs; 20..=39 with in_attrs;
    for id in 0..20 {
        g.node_set_attrs(
            id,
            vec![
                ("email".into(), format!("e{id}@x")),
                ("phone".into(), format!("p{id}")),
            ],
        )
        .await
        .unwrap();
    }
    for id in 20..40 {
        g.node_set_attrs(
            id,
            vec![
                ("age".into(), format!("{}", 20 + id)),
                ("address".into(), format!("a{id}")),
            ],
        )
        .await
        .unwrap();
    }

    // 5a. type_out_1 → Person
    g.projection_define(ProjectionSpec {
        name: "t_out1".into(),
        type_out: Some("Person".into()),
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("t_out1").await.unwrap();
    assert!(r.node_count > 0);

    // 5b. type_out_2 → Org
    g.projection_define(ProjectionSpec {
        name: "t_out2".into(),
        type_out: Some("Org".into()),
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("t_out2").await.unwrap();
    assert!(r.node_count > 0);

    // 5c. community_in_1 → community=1 (40 nodes)
    g.projection_define(ProjectionSpec {
        name: "cm1".into(),
        community: Some(1),
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("cm1").await.unwrap();
    assert_eq!(r.node_count, 40);

    // 5d. community_in_2
    g.projection_define(ProjectionSpec {
        name: "cm2".into(),
        community: Some(2),
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("cm2").await.unwrap();
    assert_eq!(r.node_count, 40);

    // 5e. attrs_out
    g.projection_define(ProjectionSpec {
        name: "ao".into(),
        attrs_out: vec!["email".into(), "phone".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("ao").await.unwrap();
    assert_eq!(r.node_count, 20);

    // 5f. attrs_in
    g.projection_define(ProjectionSpec {
        name: "ai".into(),
        attrs_in: vec!["age".into(), "address".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("ai").await.unwrap();
    assert_eq!(r.node_count, 20);

    // 5g. degree_out ≥ 2 → non-zero given 2000 edges on 280 nodes (avg 7.1)
    g.projection_define(ProjectionSpec {
        name: "deg2".into(),
        min_degree_out: 2,
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("deg2").await.unwrap();
    assert!(r.node_count > 0);

    // 5h. label_in → Account (280/4 types = 70)
    g.projection_define(ProjectionSpec {
        name: "acc_in".into(),
        node_labels: vec!["Account".into()],
        ..Default::default()
    })
    .await
    .unwrap();
    let r = g.projection_run("acc_in").await.unwrap();
    assert_eq!(r.node_count, 70);

    // Undefined projection → NotFound
    let err = g.projection_run("no-such").await.unwrap_err();
    assert!(matches!(err, GraphError::NotFound(_)));
    // Empty name on define → InvalidRequest
    let err = g
        .projection_define(ProjectionSpec { name: "".into(), ..Default::default() })
        .await
        .unwrap_err();
    assert!(matches!(err, GraphError::InvalidRequest(_)));
}

// ---------------------------------------------------------------------------
// 6. AC-15 F1: idempotent double-write
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t06_ac15_f1_double_idempotent_verifies_report() {
    let g = GraphClient::new();
    let batch: Vec<Node> = (0..10)
        .map(|i| Node {
            id: i,
            label: "L".into(),
            typ: "T".into(),
            community: 0,
            attrs: HashMap::new(),
        })
        .collect();
    let (skips, report) = g.ac15_f1_double_idempotent(batch).await.unwrap();
    assert_eq!(skips, 10);
    assert!(report.idempotent_verified);
    assert_eq!(report.fault_tag, "f1");
}

// ---------------------------------------------------------------------------
// 7. AC-15 F3: lost zero
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t07_ac15_f3_lost_zero_counts_each_explicit_zero_attr() {
    let g = GraphClient::new();
    let (c, r) = g.ac15_f3_lost_zero(30).await.unwrap();
    assert_eq!(c, 60); // 2 attrs × 30 nodes
    assert_eq!(r.lost_zero_count, 60);
    assert_eq!(r.fault_tag, "f3");
    // verify nodes landed and actually contain "0"
    let nodes = g.list_nodes().await.unwrap();
    let z = nodes
        .iter()
        .filter(|n| {
            n.attrs.get("score").map(|v| v.as_str()) == Some("0")
                && n.attrs.get("balance").map(|v| v.as_str()) == Some("0")
        })
        .count();
    assert_eq!(z, 30);
}

// ---------------------------------------------------------------------------
// 8. AC-15 F6: partial write skips None rows and keeps count
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t08_ac15_f6_partial_writes_only_valid_rows() {
    let g = GraphClient::new();
    let mut batch: Vec<Option<Node>> = Vec::new();
    for i in 0..20i64 {
        batch.push(if i % 2 == 0 {
            Some(Node {
                id: 100 + i,
                label: "V".into(),
                typ: "V".into(),
                community: 0,
                attrs: HashMap::new(),
            })
        } else {
            None
        });
    }
    let (w, r) = g.ac15_f6_partial(batch).await.unwrap();
    assert_eq!(w, 10);
    assert_eq!(r.partial_writes, 10);
    // Valid nodes with id 100 + even idx exist
    let nodes = g.list_nodes().await.unwrap();
    assert_eq!(nodes.len(), 10);
}

// ---------------------------------------------------------------------------
// 9. AC-15 F7: diskfull injection toggles DiskFull error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t09_ac15_f7_diskfull_injection_controls_error() {
    let g = GraphClient::new();
    // No fault → OK path
    let (n, r) = g.ac15_f7_diskfull(1 << 20).await.unwrap();
    assert_eq!(n, 1 << 20);
    assert!(!r.diskfull_triggered);
    // Inject f7 → DiskFull + report sets diskfull_triggered
    g.ac15_inject("f7").await.unwrap();
    let err = g.ac15_f7_diskfull(42).await.unwrap_err();
    assert!(matches!(err, GraphError::DiskFull(_)), "got: {:?}", err);
    let r2 = g.ac15_report().await.unwrap();
    assert!(r2.diskfull_triggered);
}

// ---------------------------------------------------------------------------
// 10. AC-15 F8: callback + audit entries
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t10_ac15_f8_cb_and_audit_entry_persists() {
    let g = GraphClient::new();
    let (fired, r) = g.ac15_f8_cb_audit("hello-f8").await.unwrap();
    assert!(fired);
    assert!(r.callback_fired);
    assert_eq!(r.audit_entries.len(), 1);
    assert!(r.audit_entries[0].contains("hello-f8"));
    assert_eq!(r.fault_tag, "f8");
    // Second call appends
    let _ = g.ac15_f8_cb_audit("second").await.unwrap();
    let r2 = g.ac15_report().await.unwrap();
    assert_eq!(r2.audit_entries.len(), 2);
}

// ---------------------------------------------------------------------------
// 11. AC-15 F12: timeout + dedup fault injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t11_ac15_f12_timeout_dedup_fault_and_success() {
    let g = GraphClient::new();
    // success: no fault
    let (d, r) = g.ac15_f12_timeout_dedup(5).await.unwrap();
    assert_eq!(d, 5);
    assert_eq!(r.dedup_hits, 5);
    assert_eq!(r.timeout_hits, 0);
    // inject f12 → Timeout, timeout_hits+1, dedup still counted
    g.ac15_inject("f12").await.unwrap();
    let err = g.ac15_f12_timeout_dedup(2).await.unwrap_err();
    assert!(matches!(err, GraphError::Timeout(_)), "got: {:?}", err);
    let r2 = g.ac15_report().await.unwrap();
    assert_eq!(r2.timeout_hits, 1);
    assert_eq!(r2.dedup_hits, 7);
}

// ---------------------------------------------------------------------------
// 12. AC-15 F13: lag spike records sample
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t12_ac15_f13_lag_spike_records_in_report() {
    let g = GraphClient::new();
    let (v, r) = g.ac15_f13_lag_spike(99_999).await.unwrap();
    assert_eq!(v, 99_999);
    assert_eq!(r.lag_spike_ms, 99_999);
    assert_eq!(r.fault_tag, "f13");
    let r2 = g.ac15_report().await.unwrap();
    assert_eq!(r2.lag_spike_ms, 99_999);
}

// ---------------------------------------------------------------------------
// 13. AC-15 F14: audit-only callback appends entries without failing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t13_ac15_f14_audit_cb_accumulates_and_never_fails() {
    let g = GraphClient::new();
    let (n, r) = g.ac15_f14_audit_cb(&["a", "b", "c"]).await.unwrap();
    assert_eq!(n, 3);
    assert_eq!(r.audit_entries.len(), 3);
    assert!(r.callback_fired);
    let (n2, r2) = g.ac15_f14_audit_cb(&["d", "e"]).await.unwrap();
    assert_eq!(n2, 5);
    assert_eq!(r2.audit_entries.len(), 5);
    for e in &["a", "b", "c", "d", "e"] {
        assert!(
            r2.audit_entries.iter().any(|x| x.contains(e)),
            "missing F14-audit for {e}: {:?}",
            r2.audit_entries
        );
    }
}

// ---------------------------------------------------------------------------
// 14. AC-15 reset clears state
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t14_ac15_reset_clears_report_and_faults() {
    let g = GraphClient::new();
    g.ac15_inject("f7").await.unwrap();
    let _ = g.ac15_f13_lag_spike(1).await.unwrap();
    let _ = g.ac15_f14_audit_cb(&["x"]).await.unwrap();
    g.ac15_reset().await.unwrap();
    let r = g.ac15_report().await.unwrap();
    assert_eq!(r.lag_spike_ms, 0);
    assert!(r.audit_entries.is_empty());
    assert!(!r.callback_fired);
    // reset cleared f7 fault — calling f7 no longer errors
    let (ok, _) = g.ac15_f7_diskfull(10).await.unwrap();
    assert_eq!(ok, 10);
}

// ---------------------------------------------------------------------------
// 15. Unknown operations yield NotFound and state shares across clones
// ---------------------------------------------------------------------------

#[tokio::test]
async fn t15_clone_shares_state_and_missing_paths_yield_notfound() {
    let a = GraphClient::new();
    let b = a.clone();

    a.cdc_new_consumer("t", "c").await.unwrap();
    assert!(b.cdc_get_consumer("c").await.is_ok());

    a.spark_seed_nodes(5).await.unwrap();
    assert_eq!(b.list_nodes().await.unwrap().len(), 5);

    // NotFound paths (sanity across multiple facades)
    assert!(matches!(
        a.node_set_attrs(999_999, vec![]).await.unwrap_err(),
        GraphError::NotFound(_)
    ));
    assert!(matches!(
        a.cdc_next_blocking("does-not-exist").await.unwrap_err(),
        GraphError::NotFound(_)
    ));
    assert!(matches!(
        a.cdc_resume_offset("does-not-exist", 0).await.unwrap_err(),
        GraphError::NotFound(_)
    ));
    assert!(matches!(
        a.cdc_dedup_bump("does-not-exist", 0).await.unwrap_err(),
        GraphError::NotFound(_)
    ));
    assert!(matches!(
        a.cdc_lag_sample("does-not-exist", 0).await.unwrap_err(),
        GraphError::NotFound(_)
    ));
    assert!(matches!(
        a.cdc_rotate_consumer("does-not-exist", "x").await.unwrap_err(),
        GraphError::NotFound(_)
    ));
}
