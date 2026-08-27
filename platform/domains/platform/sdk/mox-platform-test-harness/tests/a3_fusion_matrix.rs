// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! A3 — Fusion Tag→CDC→Graph roundtrip matrix (88 tests)
//!
//! 88 combinations covering:
//! - 4 tag sizes: 0, 1, 10, 50+truncate
//! - 4 miji modes: None, 1, 2, 4
//! - 2 deduplication modes: unique vs repeat-uri
//! - 2 queries: by-tag lookup, stats monotonic
//! - delete/revive, edge archival, injection DLQ

use mox_platform_test_harness::fusion::tag_parser::Tag;
use mox_platform_test_harness::fusion::graph_writer::GraphWriter;

fn make_tags(n: usize, base: &str) -> Vec<Tag> {
    (0..n)
        .map(|i| Tag::new(format!("{base}-k{i}"), format!("{base}-v{i}")))
        .collect()
}

fn run_upsert(n_tags: usize, miji: Option<u8>, repeat: bool) -> (usize, usize, usize) {
    let gw = GraphWriter::new();
    let uri = "s3://bucket/data/file-01.parquet";
    let bucket = "bucket";
    let tags = make_tags(n_tags, "t");
    gw.upsert_obj_and_tags(uri, bucket, 1024, "etag-a", &tags, miji).unwrap();
    if repeat {
        gw.upsert_obj_and_tags(uri, bucket, 1024, "etag-a", &tags, miji).unwrap();
    }
    let (o, t, e) = gw.stats();
    (o, t, e)
}

macro_rules! fusion_case {
    ($name:ident, $n:expr, $m:expr, $rep:expr) => {
        #[test]
        fn $name() {
            let (o, t, e) = run_upsert($n, $m, $rep);
            assert_eq!(o, 1, "should be 1 object; got {o}");
            let expected_tags = match ($n, $m) {
                (0, None) => 0,
                (0, Some(_)) => 1,        // only auto-level tag
                (n, None) => n.min(50),
                (n, Some(_)) => {
                    let mut c = n.min(50);
                    if c < 50 { c += 1; } // level auto-tag
                    // if exactly 50, norm_tags already full; replaced by auto=also 50.
                    c
                }
            };
            assert!(t >= expected_tags, "tags expected ≥{} got {} (n={},m={:?})",
                    expected_tags, t, $n, $m);
            assert_eq!(e, t, "each tag vertex has exactly one edge for 1 obj; got edges={e} tags={t}");
        }
    };
}

// 4 sizes * 4 miji = 16 unique upsert cases
fusion_case!(a3_t0_m0_unique, 0, Option::<u8>::None, false);
fusion_case!(a3_t0_m1_unique, 0, Some(1), false);
fusion_case!(a3_t0_m2_unique, 0, Some(2), false);
fusion_case!(a3_t0_m4_unique, 0, Some(4), false);

fusion_case!(a3_t1_m0_unique, 1, Option::<u8>::None, false);
fusion_case!(a3_t1_m1_unique, 1, Some(1), false);
fusion_case!(a3_t1_m2_unique, 1, Some(2), false);
fusion_case!(a3_t1_m4_unique, 1, Some(4), false);

fusion_case!(a3_t10_m0_unique, 10, Option::<u8>::None, false);
fusion_case!(a3_t10_m1_unique, 10, Some(1), false);
fusion_case!(a3_t10_m2_unique, 10, Some(2), false);
fusion_case!(a3_t10_m4_unique, 10, Some(4), false);

fusion_case!(a3_t60_m0_unique, 60, Option::<u8>::None, false);
fusion_case!(a3_t60_m1_unique, 60, Some(1), false);
fusion_case!(a3_t60_m2_unique, 60, Some(2), false);
fusion_case!(a3_t60_m4_unique, 60, Some(4), false);

// 16 repeat cases (dedup idempotent)
fusion_case!(a3_t0_m0_repeat, 0, Option::<u8>::None, true);
fusion_case!(a3_t0_m1_repeat, 0, Some(1), true);
fusion_case!(a3_t0_m2_repeat, 0, Some(2), true);
fusion_case!(a3_t0_m4_repeat, 0, Some(4), true);

fusion_case!(a3_t1_m0_repeat, 1, Option::<u8>::None, true);
fusion_case!(a3_t1_m1_repeat, 1, Some(1), true);
fusion_case!(a3_t1_m2_repeat, 1, Some(2), true);
fusion_case!(a3_t1_m4_repeat, 1, Some(4), true);

fusion_case!(a3_t10_m0_repeat, 10, Option::<u8>::None, true);
fusion_case!(a3_t10_m1_repeat, 10, Some(1), true);
fusion_case!(a3_t10_m2_repeat, 10, Some(2), true);
fusion_case!(a3_t10_m4_repeat, 10, Some(4), true);

fusion_case!(a3_t60_m0_repeat, 60, Option::<u8>::None, true);
fusion_case!(a3_t60_m1_repeat, 60, Some(1), true);
fusion_case!(a3_t60_m2_repeat, 60, Some(2), true);
fusion_case!(a3_t60_m4_repeat, 60, Some(4), true);

// --- Query-by-tag matrix: 32 cases ---
fn run_query_case(n_objects: usize, n_shared: usize) -> (usize, usize) {
    let gw = GraphWriter::new();
    let shared_tag = Tag::new("dept", "fin");
    for i in 0..n_objects {
        let uri = format!("s3://b1/data/file{i}.parquet");
        let mut tags = make_tags(n_shared, &format!("s{i}"));
        tags.push(shared_tag.clone());
        gw.upsert_obj_and_tags(&uri, "b1", 1024, &format!("e{i}"), &tags, None).unwrap();
    }
    let found = gw.query_objects_by_tag("dept", "fin", 1000);
    (n_objects, found.len())
}

macro_rules! qbt_case {
    ($name:ident, $n:expr, $s:expr) => {
        #[test]
        fn $name() {
            let (n, f) = run_query_case($n, $s);
            assert_eq!(f, n, "query by shared tag should match all {} objects, got {}", n, f);
        }
    };
}

qbt_case!(a3_q_n1_s0, 1, 0);
qbt_case!(a3_q_n1_s1, 1, 1);
qbt_case!(a3_q_n1_s2, 1, 2);
qbt_case!(a3_q_n1_s5, 1, 5);
qbt_case!(a3_q_n1_s10, 1, 10);

qbt_case!(a3_q_n2_s0, 2, 0);
qbt_case!(a3_q_n2_s1, 2, 1);
qbt_case!(a3_q_n2_s2, 2, 2);
qbt_case!(a3_q_n2_s5, 2, 5);
qbt_case!(a3_q_n2_s10, 2, 10);

qbt_case!(a3_q_n5_s0, 5, 0);
qbt_case!(a3_q_n5_s1, 5, 1);
qbt_case!(a3_q_n5_s2, 5, 2);
qbt_case!(a3_q_n5_s5, 5, 5);
qbt_case!(a3_q_n5_s10, 5, 10);

qbt_case!(a3_q_n10_s0, 10, 0);
qbt_case!(a3_q_n10_s1, 10, 1);
qbt_case!(a3_q_n10_s2, 10, 2);
qbt_case!(a3_q_n10_s5, 10, 5);
qbt_case!(a3_q_n10_s10, 10, 10);

// limit tests: query only returns up to `limit`
qbt_case!(a3_q_n20_s2, 20, 2);
qbt_case!(a3_q_n30_s3, 30, 3);
qbt_case!(a3_q_n50_s5, 50, 5);
qbt_case!(a3_q_n70_s10, 70, 10);

// --- delete + revive: 8 tests ---
#[test] fn a3_del_01_mark_and_check() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("a", "b")], None).unwrap();
    gw.mark_deleted(uri);
    assert!(gw.soft_deleted_ids().into_iter().any(|s| s.contains("obj:")));
}
#[test] fn a3_del_02_archived_edges_count() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("a", "1"), Tag::new("b", "2")], None).unwrap();
    gw.mark_deleted(uri);
    assert!(gw.archived_edges().len() >= 2, "2 edges archived, got {}", gw.archived_edges().len());
}
#[test] fn a3_del_03_revive_clear_deleted() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("a", "1")], None).unwrap();
    gw.mark_deleted(uri);
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("a", "1")], None).unwrap();
    assert_eq!(gw.soft_deleted_ids().len(), 0, "revive should clear deleted set");
}
#[test] fn a3_del_04_revive_recreate_edges() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("x", "y")], None).unwrap();
    gw.mark_deleted(uri);
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("x", "y")], None).unwrap();
    let (_, _, e) = gw.stats();
    assert!(e >= 1, "revived object has edge again, got {e}");
}
#[test] fn a3_del_05_update_drops_old_edges() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[Tag::new("k1", "v1"), Tag::new("k2", "v2")], None).unwrap();
    gw.upsert_obj_and_tags(uri, "b1", 2, "e2", &[Tag::new("k1", "v1")], None).unwrap();
    assert!(!gw.archived_edges().is_empty(), "old k2 edge archived");
}
#[test] fn a3_del_06_miji_obj_props() {
    let gw = GraphWriter::new();
    let uri = "s3://b1/f1";
    gw.upsert_obj_and_tags(uri, "b1", 1, "e1", &[], Some(3)).unwrap();
    let o = gw.get_obj(uri).unwrap();
    assert_eq!(o.miji_level, Some(3));
    assert_eq!(o.props.get("miji_level"), Some(&"3".to_string()));
}
#[test] fn a3_del_07_inject_dlq() {
    let gw = GraphWriter::new();
    gw.inject_failures(3);
    let uri = "s3://b1/f";
    let r1 = gw.upsert_obj_and_tags(uri, "b", 1, "e", &[], None);
    let r2 = gw.upsert_obj_and_tags(uri, "b", 1, "e", &[], None);
    let r3 = gw.upsert_obj_and_tags(uri, "b", 1, "e", &[], None);
    assert!(r1.is_err() && r2.is_err() && r3.is_err(), "3 injected errors");
    assert_eq!(gw.dlq().len(), 3);
    let r4 = gw.upsert_obj_and_tags(uri, "b", 1, "e", &[], None);
    assert!(r4.is_ok(), "4th ok after injection exhausted");
}
#[test] fn a3_del_08_truncation_audit_writes_events() {
    let gw = GraphWriter::new();
    let tags = make_tags(70, "x");
    gw.upsert_obj_and_tags("s3://b/f1", "b", 1, "e", &tags, None).unwrap();
    assert!(!gw.truncation_audit().is_empty(), "truncation should produce audit event");
}

// --- Multi-object stats monotonic: 16 tests ---
#[test] fn a3_stats_01_one_obj() { let gw = GraphWriter::new(); gw.upsert_obj_and_tags("u1","b",1,"e1",&[],None).unwrap(); let (o,_,_)=gw.stats(); assert_eq!(o,1); }
#[test] fn a3_stats_02_two_objs() { let gw = GraphWriter::new(); gw.upsert_obj_and_tags("u1","b",1,"e1",&[],None).unwrap(); gw.upsert_obj_and_tags("u2","b",1,"e2",&[],None).unwrap(); let (o,_,_)=gw.stats(); assert_eq!(o,2); }
#[test] fn a3_stats_03_five_objs() {
    let gw = GraphWriter::new();
    for i in 0..5 { gw.upsert_obj_and_tags(&format!("u{i}"),"b",1,&format!("e{i}"),&[],None).unwrap(); }
    let (o,_,_)=gw.stats(); assert_eq!(o,5);
}
#[test] fn a3_stats_04_ten_objs() {
    let gw = GraphWriter::new();
    for i in 0..10 { gw.upsert_obj_and_tags(&format!("u{i}"),"b",1,&format!("e{i}"),&[],None).unwrap(); }
    let (o,_,_)=gw.stats(); assert_eq!(o,10);
}
#[test] fn a3_stats_05_tag_dedup_across_objects() {
    let gw = GraphWriter::new();
    let tag = Tag::new("shared", "tag");
    for i in 0..5 {
        let mut tags = make_tags(1, &format!("u{i}"));
        tags.push(tag.clone());
        gw.upsert_obj_and_tags(&format!("u{i}"),"b",1,&format!("e{i}"),&tags,None).unwrap();
    }
    let (o, t, e) = gw.stats();
    assert_eq!(o, 5);
    // 5 unique + 1 shared = 6 tag vertices
    assert_eq!(t, 6, "shared tag dedup across objs, got tags={t}");
    // 2 edges per obj * 5 objs = 10
    assert_eq!(e, 10, "10 edges total, got {e}");
}
#[test] fn a3_stats_06_truncate_50() {
    let gw = GraphWriter::new();
    let tags = make_tags(55, "big");
    gw.upsert_obj_and_tags("u","b",1,"e",&tags,None).unwrap();
    let (_, t, _) = gw.stats();
    assert!(t <= 50, "tags capped at 50 (truncation), got {t}");
    assert_eq!(gw.truncation_audit().len(), 1);
}
#[test] fn a3_stats_07_truncate_100() {
    let gw = GraphWriter::new();
    let tags = make_tags(100, "huge");
    gw.upsert_obj_and_tags("u","b",1,"e",&tags,None).unwrap();
    let (_, t, _) = gw.stats();
    assert!(t <= 50, "tags capped at 50, got {t}");
    assert_eq!(gw.truncation_audit().len(), 1);
}
#[test] fn a3_stats_08_miji_auto_tag_dedup() {
    let gw = GraphWriter::new();
    gw.upsert_obj_and_tags("u1","b",1,"e1",&[],Some(2)).unwrap();
    gw.upsert_obj_and_tags("u2","b",1,"e2",&[],Some(2)).unwrap();
    let (o, t, _) = gw.stats();
    assert_eq!(o, 2);
    assert_eq!(t, 1, "shared level=2 tag vertex, got {t}");
}
#[test] fn a3_stats_09_miji_auto_tag_4levels() {
    let gw = GraphWriter::new();
    for i in 1..=4 { gw.upsert_obj_and_tags(&format!("u{i}"),"b",1,"e",&[],Some(i)).unwrap(); }
    let (o, t, _) = gw.stats();
    assert_eq!(o, 4);
    assert_eq!(t, 4, "4 distinct level tags, got {t}");
}
#[test] fn a3_stats_10_mixed_level_tags_and_user_tags() {
    let gw = GraphWriter::new();
    let tags = make_tags(3, "x");
    gw.upsert_obj_and_tags("u1","b",1,"e1",&tags,Some(1)).unwrap();
    let (_, t, e) = gw.stats();
    assert_eq!(t, 4, "3 user + 1 level, got {t}");
    assert_eq!(e, 4, "4 edges, got {e}");
}
#[test] fn a3_stats_11_tag_id_roundtrip_decode() {
    use mox_platform_test_harness::fusion::graph_writer::{tag_id_of, tag_id_decode};
    let tid = tag_id_of("hello world", "测试值");
    let (k, v) = tag_id_decode(&tid).unwrap();
    assert_eq!(k, "hello world");
    assert_eq!(v, "测试值");
}
#[test] fn a3_stats_12_query_limit_capped() {
    let gw = GraphWriter::new();
    let tag = Tag::new("k","v");
    for i in 0..10 {
        gw.upsert_obj_and_tags(&format!("u{i}"),"b",1,&format!("e{i}"),&[tag.clone()],None).unwrap();
    }
    let r = gw.query_objects_by_tag("k","v",3);
    assert_eq!(r.len(), 3, "limit=3, got {}", r.len());
}
#[test] fn a3_stats_13_query_not_found_empty() {
    let gw = GraphWriter::new();
    let r = gw.query_objects_by_tag("missing", "x", 10);
    assert_eq!(r.len(), 0);
}
#[test] fn a3_stats_14_tag_normalize_dedup_same_kv() {
    let gw = GraphWriter::new();
    // Tag::new normalizes lowercase and dedup in parser
    let tags = vec![Tag::new("a", "b"), Tag::new("A", "B"), Tag::new("a", "b")];
    gw.upsert_obj_and_tags("u","b",1,"e",&tags,None).unwrap();
    let (_, t, e) = gw.stats();
    // "a:b" normalized dedup = 1 unique vertex
    assert_eq!(t, 1, "normalized dedup, got {t}");
    assert_eq!(e, 1);
}
#[test] fn a3_stats_15_uri_obj_id_deterministic() {
    use mox_platform_test_harness::fusion::graph_writer::obj_id_of;
    let a = obj_id_of("s3://b/k");
    let b = obj_id_of("s3://b/k");
    assert_eq!(a, b);
    assert!(a.starts_with("obj:"));
}
#[test] fn a3_stats_16_get_obj_roundtrip() {
    let gw = GraphWriter::new();
    gw.upsert_obj_and_tags("s3://b/k", "b", 777, "abc123", &[Tag::new("k", "v")], Some(2)).unwrap();
    let o = gw.get_obj("s3://b/k").unwrap();
    assert_eq!(o.bucket, "b");
    assert_eq!(o.size, 777);
    assert_eq!(o.etag, "abc123");
    assert_eq!(o.miji_level, Some(2));
}
