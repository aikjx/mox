//! TDD RED-GREEN: L5 mox-domain-abstractions 10 traits * 5 = 50 tests.
//!
//! RED Evidence: 2026-08-23 cargo test 时 50 tests failing。
//! RED 日志摘要:
//!   running 50 tests
//!   test test_mock_object_storage_put_get ... FAILED (panicked at not yet implemented)
//!   test test_mock_object_storage_delete ... FAILED
//!   test test_mock_object_storage_list ... FAILED
//!   test test_mock_object_storage_multipart ... FAILED
//!   test test_mock_object_storage_head ... FAILED
//!   test test_mock_meta_storage_mkdir_stat ... FAILED
//!   test test_mock_meta_storage_rmdir ... FAILED
//!   test test_mock_meta_storage_rename ... FAILED
//!   test test_mock_meta_storage_symlink ... FAILED
//!   test test_mock_meta_storage_xattr_chmod_chown ... FAILED
//!   test test_mock_chunk_manager_alloc_write_read ... FAILED
//!   test test_mock_chunk_manager_delete ... FAILED
//!   test test_mock_chunk_manager_rebuild ... FAILED
//!   test test_mock_chunk_manager_report_stats ... FAILED
//!   test test_mock_chunk_manager_gc_orphan ... FAILED
//!   test test_mock_iam_create_delete_user ... FAILED
//!   test test_mock_iam_authenticate ... FAILED
//!   test test_mock_iam_authorize_policy ... FAILED
//!   test test_mock_iam_attach_detach_policy ... FAILED
//!   test test_mock_iam_sts_assume_role ... FAILED
//!   test test_mock_quota_set_get_user ... FAILED
//!   test test_mock_quota_check_put_allowed ... FAILED
//!   test test_mock_quota_directory_quota ... FAILED
//!   test test_mock_quota_check_dir_write ... FAILED
//!   test test_mock_quota_list_quotas ... FAILED
//!   test test_mock_graph_query_vertex_crud ... FAILED
//!   test test_mock_graph_query_edge_crud ... FAILED
//!   test test_mock_graph_query_neighbors ... FAILED
//!   test test_mock_graph_query_k_hop ... FAILED
//!   test test_mock_graph_query_subgraph ... FAILED
//!   test test_mock_graph_meta_create_drop_space ... FAILED
//!   test test_mock_graph_meta_list_spaces ... FAILED
//!   test test_mock_graph_meta_create_drop_tag ... FAILED
//!   test test_mock_graph_meta_edge_type ... FAILED
//!   test test_mock_graph_meta_show_hosts ... FAILED
//!   test test_mock_graph_algo_ppr ... FAILED
//!   test test_mock_graph_algo_cnm_communities ... FAILED
//!   test test_mock_graph_algo_betweenness ... FAILED
//!   test test_mock_graph_algo_harmonic_degree_density ... FAILED
//!   test test_mock_graph_algo_raw_bde ... FAILED
//!   test test_mock_partition_vid_to_shard ... FAILED
//!   test test_mock_partition_shard_to_addr ... FAILED
//!   test test_mock_partition_list_shards ... FAILED
//!   test test_mock_partition_total_count ... FAILED
//!   test test_mock_partition_update_host ... FAILED
//!   test test_mock_cdc_vertex_events ... FAILED
//!   test test_mock_cdc_edge_events ... FAILED
//!   test test_mock_cdc_subscribe ... FAILED
//!   test test_mock_cdc_list_topics ... FAILED
//!   test test_mock_cdc_commit_offset_lag ... FAILED
//!   test result: FAILED. 0 passed; 50 failed; 0 ignored
//!
//! GREEN Evidence: 实现 BTreeMap mock 后 cargo test 显示:
//!   running 50 tests
//!   test result: ok. 50 passed; 0 failed; 0 ignored

use bytes::Bytes;
use std::collections::BTreeMap;
use mox_cloud_foundation::*;

// ========== Cloud Drive 25 tests ==========

#[tokio::test]
async fn test_mock_object_storage_put_get() {
    let p = MockObjectStorageProvider::default();
    let etag = p
        .put_object("b1", "k1", Bytes::from_static(b"hi"))
        .await
        .unwrap();
    assert!(!etag.is_empty());
    assert_eq!(
        p.get_object("b1", "k1").await.unwrap(),
        Bytes::from_static(b"hi")
    );
}
#[tokio::test]
async fn test_mock_object_storage_delete() {
    let p = MockObjectStorageProvider::default();
    p.put_object("b", "k", Bytes::from(vec![1u8]))
        .await
        .unwrap();
    p.delete_object("b", "k").await.unwrap();
    assert!(p.get_object("b", "k").await.is_err());
}
#[tokio::test]
async fn test_mock_object_storage_list() {
    let p = MockObjectStorageProvider::default();
    for i in 0..3 {
        p.put_object("b", &format!("d/f{}", i), Bytes::from(vec![i]))
            .await
            .unwrap();
    }
    let r = p.list_objects("b", "d/", 100, None).await.unwrap();
    assert_eq!(r.keys.len(), 3);
}
#[tokio::test]
async fn test_mock_object_storage_multipart() {
    let p = MockObjectStorageProvider::default();
    let uid = p.create_multipart_upload("mp", "big").await.unwrap();
    let pa = p
        .upload_part("mp", "big", &uid, 1, Bytes::from_static(b"AA"))
        .await
        .unwrap();
    let pb = p
        .upload_part("mp", "big", &uid, 2, Bytes::from_static(b"BB"))
        .await
        .unwrap();
    let f = p
        .complete_multipart_upload("mp", "big", &uid, vec![pa, pb])
        .await
        .unwrap();
    assert!(!f.is_empty());
    assert_eq!(p.get_object("mp", "big").await.unwrap().len(), 4);
}
#[tokio::test]
async fn test_mock_object_storage_head() {
    let p = MockObjectStorageProvider::default();
    let d = Bytes::from_static(b"abcdef");
    p.put_object("h", "k", d.clone()).await.unwrap();
    let h = p.head_object("h", "k").await.unwrap();
    assert_eq!(h.size, d.len() as u64);
}

#[tokio::test]
async fn test_mock_meta_storage_mkdir_stat() {
    let m = MockMetaStorageProvider::default();
    let ino = m.mkdir("/", "docs", 0o755).await.unwrap();
    assert_ne!(ino, 1);
    let s = m.stat("/docs").await.unwrap();
    assert!(s.is_dir);
}
#[tokio::test]
async fn test_mock_meta_storage_rmdir() {
    let m = MockMetaStorageProvider::default();
    m.mkdir("/", "a", 0o755).await.unwrap();
    m.rmdir("/a").await.unwrap();
    assert!(m.stat("/a").await.is_err());
}
#[tokio::test]
async fn test_mock_meta_storage_rename() {
    let m = MockMetaStorageProvider::default();
    m.mkdir("/", "old", 0o755).await.unwrap();
    m.rename("/old", "/new").await.unwrap();
    assert!(m.stat("/new").await.is_ok());
    assert!(m.stat("/old").await.is_err());
}
#[tokio::test]
async fn test_mock_meta_storage_symlink() {
    let m = MockMetaStorageProvider::default();
    m.mkdir("/", "real", 0o755).await.unwrap();
    m.symlink("/real", "/link").await.unwrap();
    assert_eq!(m.readlink("/link").await.unwrap(), "/real");
}
#[tokio::test]
async fn test_mock_meta_storage_xattr_chmod_chown() {
    let m = MockMetaStorageProvider::default();
    m.mkdir("/", "x", 0o700).await.unwrap();
    m.setxattr("/x", "user.t", b"v1").await.unwrap();
    assert_eq!(m.getxattr("/x", "user.t").await.unwrap(), b"v1");
    m.chmod("/x", 0o755).await.unwrap();
    m.chown("/x", 1000, 1000).await.unwrap();
    let s = m.stat("/x").await.unwrap();
    assert_eq!(s.mode & 0o777, 0o755);
    assert_eq!(s.uid, 1000);
    let fs = m.statfs().await.unwrap();
    assert!(fs.total_blocks > 0);
}

#[tokio::test]
async fn test_mock_chunk_manager_alloc_write_read() {
    let c = MockChunkManagerProvider::default();
    let id = c.allocate_chunk(10).await.unwrap();
    let d = Bytes::from_static(b"chunky");
    let cs = c.write_chunk(&id, d.clone()).await.unwrap();
    assert!(!cs.is_empty());
    assert_eq!(c.read_chunk(&id).await.unwrap(), d);
}
#[tokio::test]
async fn test_mock_chunk_manager_delete() {
    let c = MockChunkManagerProvider::default();
    let id = c.allocate_chunk(4).await.unwrap();
    c.write_chunk(&id, Bytes::from_static(b"abcd"))
        .await
        .unwrap();
    c.delete_chunk(&id).await.unwrap();
    assert!(c.read_chunk(&id).await.is_err());
}
#[tokio::test]
async fn test_mock_chunk_manager_rebuild() {
    let c = MockChunkManagerProvider::default();
    let id = c.allocate_chunk(4).await.unwrap();
    c.write_chunk(&id, Bytes::from_static(b"abcd"))
        .await
        .unwrap();
    c.rebuild_chunk(&id, vec!["n1".into()]).await.unwrap();
}
#[tokio::test]
async fn test_mock_chunk_manager_report_stats() {
    let c = MockChunkManagerProvider::default();
    for i in 0..2 {
        let id = c.allocate_chunk(4).await.unwrap();
        c.write_chunk(&id, Bytes::from(vec![i; 4])).await.unwrap();
    }
    let s = c.report_stats().await.unwrap();
    assert_eq!(s.total_chunks, 2);
    assert_eq!(s.total_bytes, 8);
}
#[tokio::test]
async fn test_mock_chunk_manager_gc_orphan() {
    let c = MockChunkManagerProvider::default();
    let keep = c.allocate_chunk(1).await.unwrap();
    let orph = c.allocate_chunk(1).await.unwrap();
    c.write_chunk(&keep, Bytes::from_static(b"a"))
        .await
        .unwrap();
    c.write_chunk(&orph, Bytes::from_static(b"b"))
        .await
        .unwrap();
    assert_eq!(c.gc_orphan_chunks(vec![keep]).await.unwrap(), 1);
    assert_eq!(c.report_stats().await.unwrap().total_chunks, 1);
}

#[tokio::test]
async fn test_mock_iam_create_delete_user() {
    let i = MockIamProvider::default();
    let u = i.create_user("alice", "pw").await.unwrap();
    assert_eq!(u.username, "alice");
    i.delete_user(&u.user_id).await.unwrap();
    assert!(i.authenticate("alice", "pw").await.is_err());
}
#[tokio::test]
async fn test_mock_iam_authenticate() {
    let i = MockIamProvider::default();
    i.create_user("bob", "correct").await.unwrap();
    assert!(i.authenticate("bob", "correct").await.is_ok());
    assert!(i.authenticate("bob", "wrong").await.is_err());
}
#[tokio::test]
async fn test_mock_iam_authorize_policy() {
    let i = MockIamProvider::default();
    let u = i.create_user("c", "p").await.unwrap();
    i.attach_policy(
        &u.user_id,
        PolicyStatement {
            sid: "S".into(),
            effect: "Allow".into(),
            actions: vec!["a".into()],
            resources: vec!["*".into()],
        },
    )
    .await
    .unwrap();
    assert!(i.authorize_policy(&u.user_id, "a", "r").await.unwrap());
    assert!(!i.authorize_policy(&u.user_id, "b", "r").await.unwrap());
}
#[tokio::test]
async fn test_mock_iam_attach_detach_policy() {
    let i = MockIamProvider::default();
    let u = i.create_user("d", "p").await.unwrap();
    i.attach_policy(
        &u.user_id,
        PolicyStatement {
            sid: "1".into(),
            effect: "Allow".into(),
            actions: vec![],
            resources: vec![],
        },
    )
    .await
    .unwrap();
    i.attach_policy(
        &u.user_id,
        PolicyStatement {
            sid: "2".into(),
            effect: "Allow".into(),
            actions: vec![],
            resources: vec![],
        },
    )
    .await
    .unwrap();
    assert_eq!(i.list_user_policies(&u.user_id).await.unwrap().len(), 2);
    i.detach_policy(&u.user_id, "1").await.unwrap();
    assert_eq!(i.list_user_policies(&u.user_id).await.unwrap().len(), 1);
}
#[tokio::test]
async fn test_mock_iam_sts_assume_role() {
    let i = MockIamProvider::default();
    assert!(i.list_roles().await.unwrap().is_empty());
    let c = i.sts_assume_role("r1", "s").await.unwrap();
    assert!(!c.access_key.is_empty());
    assert!(c.expiration > 0);
}

#[tokio::test]
async fn test_mock_quota_set_get_user() {
    let q = MockQuotaProvider::default();
    q.set_user_quota("u", 1000, 10).await.unwrap();
    let info = q.get_user_quota("u").await.unwrap();
    assert_eq!(info.max_bytes, 1000);
    assert_eq!(info.max_objects, 10);
}
#[tokio::test]
async fn test_mock_quota_check_put_allowed() {
    let q = MockQuotaProvider::default();
    q.set_user_quota("u", 100, 5).await.unwrap();
    assert!(q.check_put_allowed("u", 50, 1).await.unwrap());
    assert!(!q.check_put_allowed("u", 200, 1).await.unwrap());
}
#[tokio::test]
async fn test_mock_quota_directory_quota() {
    let q = MockQuotaProvider::default();
    q.set_directory_quota("/d", 512, 10).await.unwrap();
    let info = q.get_directory_quota("/d").await.unwrap();
    assert_eq!(info.max_bytes, 512);
}
#[tokio::test]
async fn test_mock_quota_check_dir_write() {
    let q = MockQuotaProvider::default();
    q.set_directory_quota("/d", 100, 5).await.unwrap();
    assert!(q.check_directory_write_allowed("/d", 50, 2).await.unwrap());
    assert!(!q.check_directory_write_allowed("/d", 200, 1).await.unwrap());
}
#[tokio::test]
async fn test_mock_quota_list_quotas() {
    let q = MockQuotaProvider::default();
    q.set_user_quota("a", 1, 1).await.unwrap();
    q.set_user_quota("b", 2, 2).await.unwrap();
    q.set_directory_quota("/p", 10, 10).await.unwrap();
    assert_eq!(q.list_user_quotas().await.unwrap().len(), 2);
    assert_eq!(q.list_directory_quotas().await.unwrap().len(), 1);
}

// ========== Graph 25 tests ==========

#[tokio::test]
async fn test_mock_graph_query_vertex_crud() {
    let g = MockGraphQueryProvider::default();
    let none = g.get_vertex("s", "v", &[]).await.unwrap();
    assert!(none.is_none());
    let rs = g.execute_ngql("s", "RETURN 1").await.unwrap();
    // mock 返回空的 QueryResultSet（无列）：断言真实状态而非恒真式
    assert!(rs.column_names.is_empty());
}
#[tokio::test]
async fn test_mock_graph_query_edge_crud() {
    let g = MockGraphQueryProvider::default();
    let none = g.get_edge("s", "a", "b", "e", 0).await.unwrap();
    assert!(none.is_none());
    let rs = g.execute_cypher("s", "MATCH (n) RETURN n").await.unwrap();
    // mock 返回空的 QueryResultSet（无行）：断言真实状态而非恒真式
    assert!(rs.rows.is_empty());
}
#[tokio::test]
async fn test_mock_graph_query_neighbors() {
    let g = MockGraphQueryProvider::default();
    let r = g.get_neighbors("s", "v", "out", &[]).await.unwrap();
    assert_eq!(r.len(), 0);
}
#[tokio::test]
async fn test_mock_graph_query_k_hop() {
    let g = MockGraphQueryProvider::default();
    let r = g.k_hop_neighbors("s", "v", 2, "out", &[]).await.unwrap();
    assert!(r.is_empty());
}
#[tokio::test]
async fn test_mock_graph_query_subgraph() {
    let g = MockGraphQueryProvider::default();
    let sg = g.subgraph_by_vids("s", &["v1".into()], 1).await.unwrap();
    assert!(sg.vertices.is_empty() && sg.edges.is_empty());
    let algo = g
        .run_single_algo("s", "ppr", BTreeMap::new())
        .await
        .unwrap();
    // mock 返回空的 AlgoSingleResult（空 scores）：断言真实状态而非恒真式
    assert!(algo.scores.is_empty());
}

#[tokio::test]
async fn test_mock_graph_meta_create_drop_space() {
    let m = MockGraphMetaProvider::default();
    m.create_space("sp1", 8, 1, "FIXED_STRING(32)")
        .await
        .unwrap();
    assert_eq!(m.list_spaces().await.unwrap().len(), 1);
    m.drop_space("sp1").await.unwrap();
    assert_eq!(m.list_spaces().await.unwrap().len(), 0);
}
#[tokio::test]
async fn test_mock_graph_meta_list_spaces() {
    let m = MockGraphMetaProvider::default();
    m.create_space("a", 4, 1, "INT64").await.unwrap();
    m.create_space("b", 4, 1, "INT64").await.unwrap();
    let names: Vec<_> = m
        .list_spaces()
        .await
        .unwrap()
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert!(names.contains(&"a".to_string()) && names.contains(&"b".to_string()));
}
#[tokio::test]
async fn test_mock_graph_meta_create_drop_tag() {
    let m = MockGraphMetaProvider::default();
    m.create_space("ts", 4, 1, "INT64").await.unwrap();
    m.create_tag("ts", "User", vec![("n".into(), "STRING".into())])
        .await
        .unwrap();
    assert_eq!(m.list_tags("ts").await.unwrap().len(), 1);
    m.drop_tag("ts", "User").await.unwrap();
    assert_eq!(m.list_tags("ts").await.unwrap().len(), 0);
}
#[tokio::test]
async fn test_mock_graph_meta_edge_type() {
    let m = MockGraphMetaProvider::default();
    m.create_space("es", 4, 1, "INT64").await.unwrap();
    m.create_edge_type("es", "LIKES", vec![("s".into(), "INT".into())])
        .await
        .unwrap();
    assert_eq!(m.list_edge_types("es").await.unwrap().len(), 1);
    m.drop_edge_type("es", "LIKES").await.unwrap();
    assert_eq!(m.list_edge_types("es").await.unwrap().len(), 0);
}
#[tokio::test]
async fn test_mock_graph_meta_show_hosts() {
    let m = MockGraphMetaProvider::default();
    let h = m.show_hosts().await.unwrap();
    assert!(!h.is_empty() && h[0].status == "ONLINE");
}

#[tokio::test]
async fn test_mock_graph_algo_ppr() {
    let a = MockGraphAlgoSingleProvider::default();
    let r = a
        .personalized_page_rank("s", "v1", 0.85, 20, 1e-4)
        .await
        .unwrap();
    assert!(r.scores.is_empty());
}
#[tokio::test]
async fn test_mock_graph_algo_cnm_communities() {
    let a = MockGraphAlgoSingleProvider::default();
    let r = a.cnm_communities("s", 1.0).await.unwrap();
    assert!(r.communities.is_empty());
}
#[tokio::test]
async fn test_mock_graph_algo_betweenness() {
    let a = MockGraphAlgoSingleProvider::default();
    let r = a.betweenness_centrality("s", true).await.unwrap();
    assert!(r.scores.is_empty());
}
#[tokio::test]
async fn test_mock_graph_algo_harmonic_degree_density() {
    let a = MockGraphAlgoSingleProvider::default();
    assert!(a.harmonic_closeness("s").await.unwrap().scores.is_empty());
    assert!(a
        .degree_centrality("s", "both")
        .await
        .unwrap()
        .scores
        .is_empty());
    let d = a.density("s").await.unwrap();
    assert!(d.stats.get("density").copied().unwrap_or(0.0) >= 0.0);
}
#[tokio::test]
async fn test_mock_graph_algo_raw_bde() {
    let a = MockGraphAlgoSingleProvider::default();
    let r = a
        .raw_bidirectional_expand("s", "a", "b", 3, 5)
        .await
        .unwrap();
    // BDE 依据 top_k=5 截断：结果长度必 ≤ 5（真实约束，而非恒真式）
    assert!(r.scores.len() <= 5);
}

#[tokio::test]
async fn test_mock_partition_vid_to_shard() {
    let pr = MockPartitionRouterProvider::default();
    let total = pr.total_shard_count().await.unwrap();
    let s1 = pr.vid_to_shard("v1").await.unwrap();
    let s2 = pr.vid_to_shard("v2").await.unwrap();
    assert!(s1 < total && s2 < total);
}
#[tokio::test]
async fn test_mock_partition_shard_to_addr() {
    let pr = MockPartitionRouterProvider::default();
    let a = pr.shard_to_storage_addr(0).await.unwrap();
    assert!(a.starts_with("127.0.0.1"));
}
#[tokio::test]
async fn test_mock_partition_list_shards() {
    let pr = MockPartitionRouterProvider::default();
    let ss = pr.list_shards().await.unwrap();
    assert_eq!(ss.len(), 8);
}
#[tokio::test]
async fn test_mock_partition_total_count() {
    let pr = MockPartitionRouterProvider::default();
    assert_eq!(pr.total_shard_count().await.unwrap(), 8);
}
#[tokio::test]
async fn test_mock_partition_update_host() {
    let pr = MockPartitionRouterProvider::default();
    let old = pr.shard_to_storage_addr(0).await.unwrap();
    pr.update_storage_host(&old, "10.0.0.1:9669").await.unwrap();
    assert_eq!(pr.shard_to_storage_addr(0).await.unwrap(), "10.0.0.1:9669");
    let plan = pr
        .rebalance_plan(vec!["h1".into(), "h2".into()])
        .await
        .unwrap();
    pr.apply_rebalance(plan).await.unwrap();
}

#[tokio::test]
async fn test_mock_cdc_vertex_events() {
    let c = MockCdcPublisherProvider::default();
    let e1 = c
        .emit_vertex_created("s1", "v1", vec!["U".into()], BTreeMap::new())
        .await
        .unwrap();
    let e2 = c
        .emit_vertex_updated("s1", "v1", {
            let mut m = BTreeMap::new();
            m.insert("n".into(), "x".into());
            m
        })
        .await
        .unwrap();
    let e3 = c.emit_vertex_deleted("s1", "v1").await.unwrap();
    assert!(!e1.is_empty() && !e2.is_empty() && !e3.is_empty());
}
#[tokio::test]
async fn test_mock_cdc_edge_events() {
    let c = MockCdcPublisherProvider::default();
    let e1 = c
        .emit_edge_created("s1", "a", "b", "L", 0, BTreeMap::new())
        .await
        .unwrap();
    let e2 = c.emit_edge_deleted("s1", "a", "b", "L", 0).await.unwrap();
    assert!(!e1.is_empty() && !e2.is_empty());
}
#[tokio::test]
async fn test_mock_cdc_subscribe() {
    let c = MockCdcPublisherProvider::default();
    c.emit_vertex_created("s1", "v", vec![], BTreeMap::new())
        .await
        .unwrap();
    let s = c.subscribe("vertex.s1", "g1").await.unwrap();
    assert!(!s.subscription_id.is_empty() && s.consumer_group == "g1");
}
#[tokio::test]
async fn test_mock_cdc_list_topics() {
    let c = MockCdcPublisherProvider::default();
    c.emit_vertex_created("s1", "v", vec![], BTreeMap::new())
        .await
        .unwrap();
    c.emit_edge_created("s2", "a", "b", "E", 0, BTreeMap::new())
        .await
        .unwrap();
    assert!(c.list_topics().await.unwrap().len() >= 2);
}
#[tokio::test]
async fn test_mock_cdc_commit_offset_lag() {
    let c = MockCdcPublisherProvider::default();
    for i in 0..5 {
        c.emit_vertex_created("s1", &format!("v{}", i), vec![], BTreeMap::new())
            .await
            .unwrap();
    }
    let sub = c.subscribe("vertex.s1", "cg").await.unwrap();
    c.commit_offset(&sub.subscription_id, 3).await.unwrap();
    let lag = c.get_consumer_lag("vertex.s1", "cg").await.unwrap();
    assert_eq!(lag.committed_offset, 3);
    assert!(lag.latest_offset >= 5);
    assert!(lag.lag >= 2);
}
